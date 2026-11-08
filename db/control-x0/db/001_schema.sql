-- Control X0: mismo contrato de datos que CouchDB (X1), en modelo relacional.
-- Ejecutar sobre una base NUEVA:  psql -v ON_ERROR_STOP=1 -f 001_schema.sql
-- Equivalencias con X1: CHECKs == validate_doc_update; indices == indices Mango; v_iva_mensual == vista map/reduce.
BEGIN;

CREATE EXTENSION IF NOT EXISTS btree_gist;   -- para el EXCLUDE de vigencias de reglas
CREATE SCHEMA IF NOT EXISTS fiscal;
SET LOCAL search_path = fiscal, public;

-- ---------- tipos ----------
CREATE TYPE tipo_contribuyente AS ENUM ('natural', 'juridica');
CREATE TYPE rol_usuario        AS ENUM ('administrador', 'contador', 'operador', 'lectura');
CREATE TYPE tipo_operacion     AS ENUM ('venta', 'compra');
CREATE TYPE estado_operacion   AS ENUM ('registrada', 'anulada');
CREATE TYPE origen_registro    AS ENUM ('local', 'remoto', 'api');
CREATE TYPE estado_factura     AS ENUM ('pendiente', 'aceptado_simulado', 'rechazado');
CREATE TYPE obligacion_fiscal  AS ENUM ('iva_f07', 'pago_cuenta_f14', 'renta_anual_f11');
CREATE TYPE estado_declaracion AS ENUM ('borrador', 'presentada');
CREATE TYPE estado_recordatorio AS ENUM ('pendiente', 'enviado', 'cumplido');

-- ---------- utilidades ----------
CREATE FUNCTION set_actualizado_en() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN NEW.actualizado_en := now(); RETURN NEW; END $$;

-- ---------- negocios y usuarios ----------
CREATE TABLE negocios (
  id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  nombre             text NOT NULL CHECK (length(nombre) > 0),
  nit                text,
  nrc                text,
  tipo_contribuyente tipo_contribuyente NOT NULL,
  creado_en          timestamptz NOT NULL DEFAULT now(),
  actualizado_en     timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE usuarios (
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email          text NOT NULL CHECK (email = lower(email) AND position('@' in email) > 1),
  password_hash  text NOT NULL CHECK (password_hash LIKE '$argon2%'),
  rol            rol_usuario NOT NULL,
  activo         boolean NOT NULL DEFAULT true,
  creado_en      timestamptz NOT NULL DEFAULT now(),
  actualizado_en timestamptz NOT NULL DEFAULT now(),
  UNIQUE (email)
);

CREATE TABLE usuarios_negocios (            -- un usuario puede operar varios negocios
  usuario_id uuid NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
  negocio_id uuid NOT NULL REFERENCES negocios(id) ON DELETE CASCADE,
  PRIMARY KEY (usuario_id, negocio_id)
);

-- ---------- ventas y compras (tabla unica, como en X1 donde ambas comparten esquema) ----------
CREATE TABLE operaciones (
  id             uuid PRIMARY KEY,                            -- generado en el CLIENTE (idempotencia)
  negocio_id     uuid NOT NULL REFERENCES negocios(id) ON DELETE CASCADE,
  tipo           tipo_operacion NOT NULL,
  fecha          date NOT NULL,
  descripcion    text NOT NULL CHECK (length(descripcion) > 0),
  base           numeric(14,2) NOT NULL CHECK (base >= 0),
  iva            numeric(14,2) NOT NULL CHECK (iva >= 0),
  total          numeric(14,2) NOT NULL CHECK (total >= 0),
  contraparte    jsonb,
  estado         estado_operacion NOT NULL DEFAULT 'registrada',
  creado_por     text NOT NULL,
  origen         origen_registro NOT NULL,
  creado_en      timestamptz NOT NULL DEFAULT now(),
  actualizado_en timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT total_es_base_mas_iva CHECK (total = base + iva),
  CONSTRAINT contraparte_objeto CHECK (contraparte IS NULL OR jsonb_typeof(contraparte) = 'object'),
  UNIQUE (id, negocio_id)
);

-- ---------- facturas (siempre simuladas) ----------
CREATE TABLE facturas (
  id               uuid PRIMARY KEY,
  negocio_id       uuid NOT NULL REFERENCES negocios(id) ON DELETE CASCADE,
  operacion_id     uuid NOT NULL,
  numero_control   text NOT NULL CHECK (length(numero_control) > 0),
  estado           estado_factura NOT NULL DEFAULT 'pendiente',
  hora_emision     timestamptz NOT NULL,
  simulado         boolean NOT NULL DEFAULT true CHECK (simulado),   -- nunca "oficial"
  sello_simulado   text,
  creado_en        timestamptz NOT NULL DEFAULT now(),
  actualizado_en   timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT sello_si_aceptada CHECK (estado <> 'aceptado_simulado' OR sello_simulado IS NOT NULL),
  FOREIGN KEY (operacion_id, negocio_id) REFERENCES operaciones (id, negocio_id) ON DELETE RESTRICT,
  UNIQUE (negocio_id, numero_control)
);

-- ---------- declaraciones ----------
CREATE TABLE declaraciones (
  id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  negocio_id         uuid NOT NULL REFERENCES negocios(id) ON DELETE CASCADE,
  obligacion         obligacion_fiscal NOT NULL,
  periodo            text NOT NULL CHECK (periodo ~ '^\d{4}(-(0[1-9]|1[0-2]))?$'),
  montos             jsonb NOT NULL CHECK (jsonb_typeof(montos) = 'object'),
  estado             estado_declaracion NOT NULL DEFAULT 'borrador',
  fecha_presentacion date,
  creado_en          timestamptz NOT NULL DEFAULT now(),
  actualizado_en     timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT fecha_si_presentada CHECK (estado <> 'presentada' OR fecha_presentacion IS NOT NULL)
);
-- una sola declaracion presentada por obligacion y periodo (los borradores pueden repetirse)
CREATE UNIQUE INDEX uq_declaracion_presentada
  ON declaraciones (negocio_id, obligacion, periodo) WHERE estado = 'presentada';

-- ---------- recordatorios ----------
CREATE TABLE recordatorios (
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  negocio_id     uuid NOT NULL REFERENCES negocios(id) ON DELETE CASCADE,
  obligacion     obligacion_fiscal NOT NULL,
  periodo        text NOT NULL,
  fecha_limite   date NOT NULL,
  estado         estado_recordatorio NOT NULL DEFAULT 'pendiente',
  creado_en      timestamptz NOT NULL DEFAULT now(),
  actualizado_en timestamptz NOT NULL DEFAULT now(),
  UNIQUE (negocio_id, obligacion, periodo)
);

-- ---------- reglas fiscales versionadas (global, igual que la base reglas_fiscales) ----------
CREATE TABLE reglas_fiscales (
  id                text PRIMARY KEY,                       -- 'regla:<codigo>:v<version>' como en X1
  codigo            text NOT NULL CHECK (codigo ~ '^[a-z0-9_]+$'),
  version           integer NOT NULL CHECK (version >= 1),
  descripcion       text,
  parametros        jsonb NOT NULL CHECK (jsonb_typeof(parametros) = 'object'),
  vigencia_desde    date NOT NULL,
  vigencia_hasta    date,
  fuente_normativa  text NOT NULL CHECK (length(fuente_normativa) >= 5),
  verificada        boolean NOT NULL,
  vigencia          daterange GENERATED ALWAYS AS (daterange(vigencia_desde, vigencia_hasta, '[]')) STORED,
  creado_en         timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT id_coherente CHECK (id = 'regla:' || codigo || ':v' || version),
  CONSTRAINT vigencia_ordenada CHECK (vigencia_hasta IS NULL OR vigencia_hasta >= vigencia_desde),
  UNIQUE (codigo, version),
  EXCLUDE USING gist (codigo WITH =, vigencia WITH &&)     -- sin vigencias solapadas por codigo
);

-- ---------- telemetria del propio X0 (para comparar con X1 en la Fase 5) ----------
CREATE TABLE metricas_requests (
  id           bigserial PRIMARY KEY,
  ts           timestamptz NOT NULL DEFAULT now(),
  request_id   uuid NOT NULL,
  sistema      text NOT NULL DEFAULT 'X0',
  metodo       text NOT NULL,
  ruta         text NOT NULL,
  codigo_http  smallint NOT NULL,
  duracion_ms  numeric(10,3) NOT NULL CHECK (duracion_ms >= 0)
);

-- ---------- indices equivalentes a los Mango de X1 ----------
CREATE INDEX idx_operaciones_tipo_fecha   ON operaciones (negocio_id, tipo, fecha);          -- idx_tipo_fecha
CREATE INDEX idx_facturas_estado          ON facturas (negocio_id, estado);                  -- idx_tipo_estado
CREATE INDEX idx_declaraciones_periodo    ON declaraciones (negocio_id, periodo, obligacion);-- idx_tipo_periodo
CREATE INDEX idx_recordatorios_activos    ON recordatorios (negocio_id, estado, fecha_limite);-- idx_recordatorio_activos
CREATE INDEX idx_facturas_operacion       ON facturas (operacion_id);                        -- idx_factura_venta
CREATE INDEX idx_reglas_codigo_vigencia   ON reglas_fiscales (codigo, vigencia_desde);       -- idx_regla_vigencia
CREATE INDEX idx_metricas_ruta_ts         ON metricas_requests (ruta, ts);

-- ---------- vista equivalente a iva_mensual (map/reduce) ----------
CREATE VIEW v_iva_mensual AS
SELECT negocio_id,
       to_char(fecha, 'YYYY-MM') AS periodo,
       tipo,
       sum(base)  AS base,
       sum(iva)   AS iva,
       sum(total) AS total,
       count(*)   AS operaciones
FROM operaciones
WHERE estado <> 'anulada'
GROUP BY negocio_id, to_char(fecha, 'YYYY-MM'), tipo;

-- ---------- triggers de actualizado_en ----------
CREATE TRIGGER trg_negocios_upd      BEFORE UPDATE ON negocios      FOR EACH ROW EXECUTE FUNCTION set_actualizado_en();
CREATE TRIGGER trg_usuarios_upd      BEFORE UPDATE ON usuarios      FOR EACH ROW EXECUTE FUNCTION set_actualizado_en();
CREATE TRIGGER trg_operaciones_upd   BEFORE UPDATE ON operaciones   FOR EACH ROW EXECUTE FUNCTION set_actualizado_en();
CREATE TRIGGER trg_facturas_upd      BEFORE UPDATE ON facturas      FOR EACH ROW EXECUTE FUNCTION set_actualizado_en();
CREATE TRIGGER trg_declaraciones_upd BEFORE UPDATE ON declaraciones FOR EACH ROW EXECUTE FUNCTION set_actualizado_en();
CREATE TRIGGER trg_recordatorios_upd BEFORE UPDATE ON recordatorios FOR EACH ROW EXECUTE FUNCTION set_actualizado_en();

COMMIT;
