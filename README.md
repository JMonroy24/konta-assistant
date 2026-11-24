<p align="center">
  <img src="docs/assets/logo.png" alt="Konta Assistant" width="320"/>
</p>

<h1 align="center">Konta Assistant</h1>

<p align="center">
  <strong>Asistente de cumplimiento fiscal y contable para microempresas de El Salvador</strong><br/>
  Offline-first · Motor fiscal versionado · Arquitectura limpia en Rust
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust"/>
  <img alt="Flutter" src="https://img.shields.io/badge/Flutter-3.x-02569B?logo=flutter"/>
  <img alt="CouchDB" src="https://img.shields.io/badge/Apache%20CouchDB-3.3-E42528?logo=apachecouchdb"/>
  <img alt="PostgreSQL" src="https://img.shields.io/badge/PostgreSQL-16-4169E1?logo=postgresql&logoColor=white"/>
  <img alt="Docker" src="https://img.shields.io/badge/Docker-Compose-2496ED?logo=docker&logoColor=white"/>
  <img alt="Licencia" src="https://img.shields.io/badge/licencia-MIT-green"/>
  <img alt="Estado" src="https://img.shields.io/badge/estado-prototipo%20académico-yellow"/>
</p>

---

## Tabla de contenidos

1. [Resumen](#1-resumen)
2. [Estado actual del proyecto](#2-estado-actual-del-proyecto)
3. [Alcance y límites](#3-alcance-y-límites)
4. [Arquitectura](#4-arquitectura)
5. [Estructura del repositorio](#5-estructura-del-repositorio)
6. [Inicio rápido](#6-inicio-rápido)
7. [Configuración](#7-configuración)
8. [API](#8-api)
9. [Motor fiscal y reglas versionadas](#9-motor-fiscal-y-reglas-versionadas)
10. [Modo offline y sincronización](#10-modo-offline-y-sincronización)
11. [Calidad, pruebas y metrología](#11-calidad-pruebas-y-metrología)
12. [Convenciones de desarrollo](#12-convenciones-de-desarrollo)
13. [Documentación adicional](#13-documentación-adicional)
14. [Equipo](#14-equipo)
15. [Aviso legal](#15-aviso-legal)
16. [Licencia](#16-licencia)

---

## 1. Resumen

**Konta Assistant** es un asistente que ayuda a una microempresa salvadoreña a registrar sus operaciones, calcular sus obligaciones tributarias y no perder vencimientos, incluso sin conexión a internet.

Es el **Proyecto 6** de la **Fase 4: Desarrollo, Metrología e Instrumentación de Software**. Además de ser un producto funcional, es un **experimento controlado**: compara dos arquitecturas de backend bajo la misma carga para medir latencia, throughput y consumo de recursos conforme a **ISO/IEC 25010**.

| Grupo | Arquitectura | Persistencia | Puerto |
| :--- | :--- | :--- | :---: |
| **X1** (propuesta) | Clean Architecture, offline-first, replicación | Apache CouchDB | `8080` |
| **X0** (control) | Backend síncrono tradicional | PostgreSQL 16 | `8082` |

**Stack:** Rust (backend exclusivo, Axum) · Apache CouchDB · Dart/Flutter (cliente multiplataforma) · Docker Compose · k6.

### Capacidades

- **Registro de operaciones:** ventas, compras y facturas electrónicas simuladas (documentos JSON).
- **Motor fiscal:** IVA (13 %), pago a cuenta (1,75 %) y renta, con decimales exactos y redondeo half-up.
- **Reglas versionadas:** tasas y tramos almacenados como documentos en CouchDB, modificables sin recompilar.
- **Calendario y recordatorios:** vencimientos de acuerdo con la normativa salvadoreña (p. ej. declaración mensual F-07).
- **Offline-first:** cola local con UUID generados en el cliente para sincronización idempotente.
- **Seguridad:** modelo de roles (Administrador, Contador, Operador, Lectura) y validación en la base de datos.
- **Instrumentación:** latencia p50/p95/p99, throughput, CPU/RSS por contenedor y latencia de replicación, exportables a CSV.

---

## 2. Estado actual del proyecto

> **Prototipo académico en desarrollo.** Esta tabla refleja con honestidad qué está implementado y qué es todavía provisional, para que nadie interprete un *stub* como funcionalidad terminada.

| Componente | Estado | Notas |
| :--- | :---: | :--- |
| Dominio, casos de uso y puertos (`domain`, `application`) | ✅ Implementado | Con pruebas unitarias. |
| Motor fiscal (`tax-engine`) | ✅ Implementado | IVA, pago a cuenta, renta, calendario, multas y explicación de cálculo. |
| `POST /ventas`, `POST /compras`, `GET /operaciones` | ✅ Persisten en CouchDB | Validación de dominio antes de guardar; `409` ante duplicados. |
| `GET /impuestos/resumen` | ✅ Funcional | Usa el motor fiscal sobre las operaciones del período. Parámetros actuales fijos (13 % / 1,75 %). |
| `POST /facturas` | 🟡 Parcial | Valida contra `mock-hacienda`; la factura **aún no se persiste**. |
| `POST /auth/login` | 🟠 Simulado | Devuelve un token fijo; **no hay verificación de credenciales ni JWT real**. |
| RBAC | 🟠 En CouchDB únicamente | Definido en `validate_doc_update`; la API aún no lo aplica por endpoint. |
| `POST /declaraciones/borrador`, `GET /recordatorios`, `GET /sync/estado`, `GET /metricas` | 🟠 Respuestas provisionales | Contratos definidos en OpenAPI; lógica por conectar con los casos de uso. |
| Backend X0 (`control-x0`) | 🟠 Mínimo | Solo `/salud`, `/ventas` y `/operaciones` provisionales; **sin persistencia real en PostgreSQL todavía**. |
| Reglas fiscales semilla | ⚠️ Sin verificar | Todas con `verificada: false` (ver [§9](#9-motor-fiscal-y-reglas-versionadas)). |
| Cliente Flutter | ✅ Pantallas y cola offline | Login, dashboard, ventas/compras, facturación, declaraciones y recordatorios. |

> **Consecuencia para el experimento:** mientras X0 y X1 no persistan de forma equivalente, las mediciones comparan el camino HTTP actual, **no** CouchDB frente a PostgreSQL. Ver [`bench/README.md`](bench/README.md).

---

## 3. Alcance y límites

### Dentro del alcance

- Registro y consulta de operaciones; emisión de DTE **simulados**.
- Cálculo automático de IVA, pago a cuenta y renta con reglas versionadas.
- Calendario de vencimientos y recordatorios.
- Modo offline con sincronización y detección de conflictos.
- Grupo de control X0 y metrología ISO/IEC 25010 para la Fase 5.

### Fuera del alcance (limitaciones declaradas)

- **Conexión oficial con el Ministerio de Hacienda.** Se usa `mock-hacienda`; la aplicación rotula todo documento como **«Aceptado Simulado»**, nunca como validación oficial.
- **Contabilidad completa:** balances generales, planillas y múltiples jurisdicciones.
- **Estudio con usuarios reales:** la comparación usa datos sintéticos y perfiles de carga controlados.
- **Asesoría tributaria:** ver [Aviso legal](#15-aviso-legal).

---

## 4. Arquitectura

```text
┌────────────────────┐        ┌─────────────────────────────────────────────┐
│  Cliente Flutter   │  HTTP  │          backend-api  (X1 · Axum)           │
│  cola offline      ├───────►│  api ─► application ─► domain               │
│  UUID en cliente   │        │            ▲   ▲                            │
└────────────────────┘        │  infrastructure │ tax-engine                │
                              └───────┬──────────────────┬──────────────────┘
                                      │                  │
                        ┌─────────────▼───┐   ┌──────────▼──────────┐
                        │ CouchDB local   │◄─►│  mock-hacienda      │
                        │ (:5985)         │   │  (DTE simulado)     │
                        └───────┬─────────┘   └─────────────────────┘
                   replicación  │ bidireccional continua
                        ┌───────▼─────────┐
                        │ CouchDB remoto  │      ┌────────────────────────┐
                        │ (:5984)         │      │ control-x0 + PostgreSQL│
                        └─────────────────┘      │ (grupo de control)     │
                                                 └────────────────────────┘
```

### Clean Architecture en el backend

Reglas de dependencia (obligatorias, ver [`AGENTS.md`](AGENTS.md)):

| Crate | Responsabilidad | Puede depender de |
| :--- | :--- | :--- |
| `domain` | Entidades, objetos de valor y errores. Sin I/O. | — |
| `application` | Casos de uso y puertos (`trait`). | `domain` |
| `tax-engine` | Cálculo tributario puro. | `domain` |
| `infrastructure` | Adaptadores: CouchDB (Reqwest) y cliente de Hacienda simulado. | `application`, `domain` |
| `api` | Rutas Axum, DTOs, telemetría y middleware. | todos |

X1 y X0 implementan los **mismos puertos** de `application`, lo que garantiza una comparación justa.

### Decisiones de diseño

Registradas como ADR en [`docs/decisiones/`](docs/decisiones/):

- **ADR-001** Clean Architecture y monorepo.
- **ADR-002** Una base `negocio_{id}` por microempresa; `reglas_fiscales` de solo lectura replicada desde el remoto; `_users` sin replicar.
- **ADR-003** Cola local (`OfflineQueue`) con UUID de cliente en móvil y web, sin PouchDB.
- **ADR-004** Mock de Hacienda con rotulado explícito de simulación.
- **ADR-005** Versionado y verificación de reglas fiscales.

---

## 5. Estructura del repositorio

```text
konta-assistant/
├── backend/                     # Workspace Rust (Clean Architecture)
│   └── crates/
│       ├── domain/              # Entidades, value objects, errores
│       ├── application/         # Casos de uso y puertos (traits)
│       ├── infrastructure/      # Adaptador CouchDB, cliente mock-hacienda
│       ├── tax-engine/          # IVA, renta, pago a cuenta, calendario, multas
│       └── api/                 # Axum: rutas, handlers, telemetría
├── control-x0/                  # Backend síncrono + PostgreSQL (grupo de control)
├── mock-hacienda/               # Simulador de validación de DTE
├── app/                         # Cliente Flutter (multiplataforma)
│   ├── lib/core/                # HTTP, auth, cola offline, sincronización, tema
│   ├── lib/features/            # auth, dashboard, ventas_compras, facturacion,
│   │                            # declaraciones, recordatorios
│   └── test/                    # Pruebas de cola offline, modelos y servicios
├── db/
│   ├── couchdb/
│   │   ├── config/              # local.ini (sin secretos)
│   │   ├── design-docs/         # validate_doc_update y vistas MapReduce
│   │   ├── indexes/             # Índices Mango
│   │   ├── scripts/             # bootstrap, replicación, explain_check, vista_iva
│   │   └── seed/                # Reglas fiscales y generador de datos sintéticos
│   └── control-x0/db/           # Esquema SQL, roles, semilla y permisos (PostgreSQL)
├── bench/                       # k6, monitor de contenedores, degradación de red,
│                                # latencia de replicación, generador de informes
├── docs/
│   ├── openapi.yaml             # Contrato de la API
│   ├── contratos.md             # Puertos y casos de uso del backend
│   ├── schemas/                 # JSON Schema de documentos
│   └── decisiones/              # ADRs
├── docker-compose.yml
├── .env.example
└── AGENTS.md                    # Reglas obligatorias de contribución
```

---

## 6. Inicio rápido

### Requisitos

| Herramienta | Versión | Necesaria para |
| :--- | :--- | :--- |
| Docker y Docker Compose | reciente | Levantar todos los servicios |
| Rust y Cargo | 1.80+ | Desarrollo y pruebas del backend |
| Flutter | 3.x (Dart `>=3.0 <4.0`) | Cliente |
| k6 | reciente | Pruebas de carga |
| Python 3 | 3.9+ | Datos sintéticos y reportes |

### 1. Variables de entorno

```bash
cp .env.example .env
```

> Cambia **todas** las contraseñas y el `JWT_SECRET` antes de exponer el sistema fuera de tu máquina.

### 2. Levantar los servicios

```bash
docker compose up -d --build
```

| Servicio | Descripción | Puerto |
| :--- | :--- | :---: |
| `couchdb-remote` | CouchDB central | `5984` |
| `couchdb-local` | CouchDB del nodo cliente (X1) | `5985` |
| `mock-hacienda` | Simulador de DTE | `8081` |
| `backend-api` | API Rust (X1) | `8080` |
| `postgres-control` | PostgreSQL (X0) | `5432` |
| `control-x0` | API síncrona (X0) | `8082` |

### 3. Inicializar CouchDB (bases, seguridad, índices y reglas)

```bash
export COUCH_ADMIN_USER=admin COUCH_ADMIN_PASS=<tu_contraseña> SVC_PASS=<contraseña_servicio>
./db/couchdb/scripts/bootstrap.sh demo1
```

Opcionalmente, carga datos sintéticos reproducibles y verifica que cada consulta use su índice:

```bash
python3 db/couchdb/seed/generate_seed.py demo1 --post http://localhost:5984
./db/couchdb/scripts/explain_check.sh demo1      # debe terminar con código 0
./db/couchdb/scripts/vista_iva.sh demo1 2026-10  # totales mensuales de IVA
```

> La API usa la base `negocio_<COUCHDB_NEGOCIO_ID>` (por defecto `demo`). Define `COUCHDB_NEGOCIO_ID` para que coincida con el identificador usado en `bootstrap.sh`.

### 4. Verificar

```bash
curl http://localhost:8080/salud
curl "http://localhost:8080/operaciones?mes=2026-11"
curl "http://localhost:8080/impuestos/resumen?mes=2026-11"
```

### 5. Ejecutar el cliente Flutter

```bash
cd app
flutter pub get
flutter run -d chrome    # o el dispositivo de tu preferencia
```

---

## 7. Configuración

Variables definidas en [`.env.example`](.env.example):

| Variable | Descripción | Valor de ejemplo |
| :--- | :--- | :--- |
| `ENVIRONMENT` | Entorno de ejecución | `development` |
| `PORT` | Puerto de la API | `8080` |
| `COUCHDB_USER` / `COUCHDB_PASSWORD` | Credenciales de administración | `admin` / *(cambiar)* |
| `COUCHDB_REMOTE_URL` | CouchDB central | `http://localhost:5984` |
| `COUCHDB_LOCAL_URL` | CouchDB local del cliente | `http://localhost:5985` |
| `COUCHDB_NEGOCIO_ID` | Identificador del negocio (`negocio_<id>`) | `demo` |
| `MOCK_HACIENDA_URL` | Simulador de Hacienda | `http://localhost:8081` |
| `JWT_SECRET` | Secreto de firma (≥ 32 caracteres) | *(cambiar)* |
| `TOKEN_EXPIRATION_HOURS` | Vigencia del token | `24` |
| `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB` | Credenciales de X0 | *(cambiar)* |
| `DATABASE_URL` | Cadena de conexión de X0 | `postgres://…` |

`.env` está en `.gitignore`: **nunca** subas credenciales reales.

---

## 8. API

Contrato completo en [`docs/openapi.yaml`](docs/openapi.yaml). Estado de cada endpoint en la [sección 2](#2-estado-actual-del-proyecto).

| Método | Ruta | Descripción |
| :---: | :--- | :--- |
| `POST` | `/auth/login` | Autenticación con roles |
| `POST` | `/ventas` | Registrar venta |
| `POST` | `/compras` | Registrar compra |
| `GET` | `/operaciones?mes=AAAA-MM` | Listar operaciones del período |
| `POST` | `/facturas` | Emitir factura electrónica **simulada** |
| `GET` | `/impuestos/resumen?mes=AAAA-MM` | IVA y pago a cuenta del período |
| `POST` | `/declaraciones/borrador` | Borrador de declaración mensual |
| `GET` | `/recordatorios` | Vencimientos próximos |
| `GET` | `/sync/estado` | Cola de sincronización y conflictos |
| `GET` | `/salud` | Healthcheck |
| `GET` | `/metricas` | Telemetría por endpoint |

**Convenciones:** los montos se serializan como **cadenas** (`"13.00"`), los períodos usan `AAAA-MM` y los mensajes al usuario están en español.

**Ejemplo: resumen de impuestos**

```bash
curl "http://localhost:8080/impuestos/resumen?mes=2026-11"
```

```json
{
  "periodo": "2026-11",
  "ingresos_gravados": "1000.00",
  "debito_iva": "130.00",
  "credito_iva": "52.00",
  "iva_a_pagar": "78.00",
  "remanente": "0.00",
  "pago_a_cuenta": "17.50",
  "estado": "calculado"
}
```

*(Valores ilustrativos del formato de respuesta.)*

---

## 9. Motor fiscal y reglas versionadas

El crate `tax-engine` ofrece cálculos puros y deterministas:

| Módulo | Función |
| :--- | :--- |
| `lib.rs` | IVA (débito, crédito, remanente) y pago a cuenta |
| `renta.rs` | Renta por tramos progresivos (persona natural) y tasas de persona jurídica |
| `calendario.rs` | Fechas de vencimiento |
| `multas.rs` | Cálculo de multas |
| `reglas.rs` | Selección de reglas por código y vigencia |
| `explicacion.rs` | Explicación paso a paso del cálculo |

**Reglas de precisión (obligatorias):** todo valor monetario usa `rust_decimal::Decimal` (nunca `f64`), se serializa como cadena y se redondea *half-up* a 2 decimales (`MidpointAwayFromZero`).

### Versionado de reglas (ADR-005)

Las reglas viven como documentos `regla:<código>:v<n>` en la base `reglas_fiscales`, con `vigencia_desde` / `vigencia_hasta`. Un cambio normativo crea una **nueva versión**; las anteriores se conservan para reproducir cálculos históricos.

> ### ⚠️ Las reglas incluidas NO están verificadas
> Los parámetros de [`db/couchdb/seed/reglas_fiscales.json`](db/couchdb/seed/reglas_fiscales.json) son **valores de partida informativos**. Todos tienen `verificada: false` y `fuente_normativa` con fecha de consulta `TODO`.
>
> Ninguna regla debe usarse en producción hasta contrastarla con la normativa vigente de Hacienda, completar el artículo exacto y registrar `verificada_por`, `fecha_verificacion` y `verificada: true`. `multa_extemporanea` es un *placeholder* vacío a propósito.

---

## 10. Modo offline y sincronización

- **Escritorio / sandbox experimental:** CouchDB local con replicación bidireccional continua (`db/couchdb/scripts/replicate.sh`); las reglas fiscales se replican remoto → local.
- **Móvil y web:** `OfflineQueue` encola operaciones con UUID generado en el cliente; al reconectar, se envían a la API de forma idempotente. Los fallos permanentes pasan a una cola *dead-letter* sin bloquear la principal.
- **Conflictos:** el esquema [`docs/schemas/conflicto.json`](docs/schemas/conflicto.json) documenta su representación.
- **Interfaz:** una franja de conectividad muestra el estado de la red y los cambios pendientes.

> Limitación de arquitectura declarada (ADR-003): PouchDB no se utiliza en Flutter.

---

## 11. Calidad, pruebas y metrología

### Pruebas y análisis estático

```bash
# Backend (ejecutar desde backend/)
cargo fmt && cargo clippy --workspace -- -D warnings && cargo test --workspace

# Cliente
cd app && flutter analyze && flutter test
```

Un cambio en el backend **no está terminado** hasta que los tres comandos de Rust pasen.

### Pruebas de carga con k6

```bash
mkdir -p bench/results

ITERATIONS_PER_TYPE=1000 VUS_PER_TYPE=1 TEST_SCENARIO=baseline \
API_URL=http://localhost:8080 SUMMARY_JSON=bench/results/x1-summary.json \
k6 run --out json=bench/results/x1-timeseries.json bench/k6/load_test.js
```

Repite con `API_URL=http://localhost:8082` para X0 y con las mismas condiciones. Usa carga moderada y **nunca** apuntes el generador a servicios ajenos o de producción.

### Instrumentos disponibles

| Métrica | Herramienta |
| :--- | :--- |
| Latencia p50/p95/p99 y throughput | `bench/k6/load_test.js` |
| CPU y RSS por contenedor | `bench/monitor/docker_stats.sh` |
| Latencia de replicación CouchDB | `bench/replication_latency.py` |
| Resiliencia ante pérdida/retardo de red | `bench/network/simulate_packet_loss_tc.sh` |
| Informe legible en Markdown | `python3 bench/report.py` |
| Uso de índices Mango | `db/couchdb/scripts/explain_check.sh` |
| Telemetría JSON de la API | `docker compose logs -f backend-api` |

Detalle metodológico, limitaciones y pendientes en [`bench/README.md`](bench/README.md).

### Mapa ISO/IEC 25010

| Característica | Verificación |
| :--- | :--- |
| Adecuación funcional | `cargo test --workspace`, `flutter test` |
| Eficiencia de desempeño | k6, monitor de contenedores |
| Fiabilidad | Degradación de red con `tc`, cola offline |
| Mantenibilidad | `clippy -D warnings`, `cargo fmt --check`, `flutter analyze` |
| Seguridad | `validate_doc_update`, secretos fuera del repositorio |

---

## 12. Convenciones de desarrollo

Resumen de [`AGENTS.md`](AGENTS.md), de cumplimiento obligatorio:

- Respetar las dependencias entre crates de la [sección 4](#clean-architecture-en-el-backend).
- Dinero siempre con `Decimal`, serializado como cadena y redondeado *half-up* a 2 decimales.
- Prohibido `unwrap()` y `expect()` fuera de pruebas; errores con `thiserror`.
- Comentarios y mensajes de usuario en español.
- **Zonas protegidas** (no modificar sin acuerdo del equipo): `backend/crates/infrastructure/`, `backend/crates/api/`, `app/`, `control-x0/`, `mock-hacienda/`.

---

## 13. Documentación adicional

| Documento | Contenido |
| :--- | :--- |
| [`docs/openapi.yaml`](docs/openapi.yaml) | Especificación OpenAPI de la API |
| [`docs/contratos.md`](docs/contratos.md) | Puertos, casos de uso y firmas del backend |
| [`docs/schemas/`](docs/schemas/) | JSON Schema: venta, compra, factura, declaración, recordatorio, regla fiscal y conflicto |
| [`docs/decisiones/`](docs/decisiones/) | ADRs |
| [`db/README.md`](db/README.md) | Scripts de CouchDB y PostgreSQL |
| [`bench/README.md`](bench/README.md) | Metodología de medición |

---

## 14. Equipo

| Integrante | Rol | Responsabilidades |
| :--- | :--- | :--- |
| **Melvin Monroy** | Núcleo de dominio y motor fiscal | `domain`, `application`, `tax-engine`, reglas versionadas, revisión de contratos |
| **Josué Escobar EA24012** | Infraestructura CouchDB y API | Repositorios, índices, `validate_doc_update`, replicación, rutas Axum, Docker Compose, X0, `mock-hacienda` |
| **Kevin Javier JC24011** | Autenticación y facturación | RBAC y tokens, DTE simulados, pantallas de login y facturación, datos sintéticos |
| **William Zuniga LZ23003** | Declaraciones, recordatorios y telemetría | Borrador de declaración, calendario, middleware de telemetría, exportación a CSV |
| **Benito Palma PR24039** | Ventas/compras y sincronización | Operaciones, dashboard con estado de conexión y cola offline, linters y pruebas de la app |

---

## 15. Aviso legal

Konta Assistant es un **proyecto académico**. Sus cálculos son orientativos y **no constituyen asesoría contable, legal ni tributaria**. Los documentos emitidos son **simulaciones** sin validez ante el Ministerio de Hacienda. Las reglas fiscales deben verificarse con la normativa vigente antes de cualquier uso real.

---

## 16. Licencia

Distribuido bajo la licencia **MIT** (ver `license` en [`backend/Cargo.toml`](backend/Cargo.toml)). Se recomienda añadir un archivo `LICENSE` en la raíz del repositorio.
