# Konta Assistant - Asistente de Cumplimiento Fiscal y Contable

Proyecto 6: Asistente de cumplimiento fiscal y contable automatizado para microempresas (El Salvador).  
**Fase 4: Desarrollo, Metrología e Instrumentación de Software**  
**Stack Principal:** Rust (backend exclusivo) + Apache CouchDB + Dart/Flutter (multiplataforma)

---

## 1. Alcance y Límites del Proyecto

### Dentro del Alcance (Producto Mínimo)
- **Registro de operaciones**: Ventas, compras y facturas electrónicas simuladas (documentos JSON).
- **Motor fiscal**: Cálculo automático de IVA (13%), renta y pago a cuenta (1.75%) con reglas versionadas en CouchDB sin recompilar.
- **Calendario y recordatorios**: Vencimientos fiscales de acuerdo a la normativa tributaria salvadoreña (ej. declaración mensual F-07).
- **Modo offline y sincronización**: Replicación continua con Apache CouchDB e identificadores UUID generados en el cliente para idempotencia.
- **Seguridad**: Control de acceso basado en roles (RBAC: Administrador, Contador, Operador, Lectura).
- **Grupo Control X0**: Backend síncrono sobre base relacional (PostgreSQL) para contraste empírico de latencia y uso de recursos.
- **Metrología ISO/IEC 25010**: Instrumentación de latencia p50/p95/p99, throughput (k6), uso de memoria/CPU, linters y exportación a CSV para la Fase 5.

### Fuera del Alcance (Declarado como Limitación)
- **Conexión oficial con el Ministerio de Hacienda**: Se utiliza un simulador mock (`mock-hacienda`). La aplicación rotula explícitamente todo documento como "Aceptado Simulado".
- **Contabilidad completa**: Balances generales, planillas y múltiples jurisdicciones quedan fuera.
- **Estudio con usuarios reales**: La comparación se realiza con datos sintéticos y perfiles de carga controlados.

---

## 2. Estructura del Monorepo

```text
konta-assistant/
├── backend/                  # Rust workspace (Clean Architecture)
│   ├── Cargo.toml
│   └── crates/
│       ├── domain/           # Entidades y reglas puras (sin dependencias externas)
│       ├── application/      # Casos de uso y puertos (traits)
│       ├── infrastructure/   # Adaptadores CouchDB (Reqwest), mock Hacienda
│       ├── api/              # Axum: rutas, DTOs, telemetría y middleware
│       └── tax-engine/       # Motor de cálculo tributario (IVA, pago a cuenta)
├── control-x0/               # Backend síncrono + PostgreSQL (Grupo Control)
├── couchdb/                  # Configuración CouchDB, vistas, índices y datos sintéticos
│   ├── design-docs/          # validate_doc_update y vistas MapReduce
│   ├── indexes/              # Definiciones de índices Mango y script de carga
│   └── seed/                 # Reglas fiscales iniciales y transacciones sintéticas
├── app/                      # Cliente Flutter (multiplataforma)
│   └── lib/
│       ├── core/             # Cliente HTTP, cola offline, sincronización, tema
│       ├── features/         # auth, ventas_compras, facturacion, declaraciones, recordatorios, dashboard
│       └── main.dart         # Punto de entrada con franja de conectividad y cambios pendientes
├── mock-hacienda/            # Simulador de validación de DTE
├── bench/                    # Scripts de k6 y escenarios de degradación de red tc-netem
├── docs/                     # Esquemas JSON de documentos, OpenAPI y ADRs
├── docker-compose.yml        # Orquestación de servicios en un solo comando
├── .env.example              # Plantilla de variables de entorno (credenciales fuera del repo)
└── README.md
```

---

## 3. División de Responsabilidades del Equipo

| Integrante | Rol Principal | Componentes a Cargo |
| :--- | :--- | :--- |
| **Melvin** | Núcleo de dominio y motor fiscal | Capas `domain`, `application`, `tax-engine`; reglas versionadas; pruebas del motor y revisión de contratos de integración. |
| **Josué** | Infraestructura CouchDB y API | Repositorios con Reqwest y Mango Queries, índices, `validate_doc_update`, replicación, rutas Axum, Docker Compose, grupo control X0 y `mock-hacienda`. |
| **Kevin** | Autenticación y facturación | RBAC y tokens (M1), emisión de DTEs simulados (M3), pantallas Flutter de login y facturación, generador de datos sintéticos. |
| **William** | Declaraciones, recordatorios y telemetría | Borrador de declaración (M5), calendario y recordatorios (M6), middleware de telemetría (M8), exportación de métricas a CSV. |
| **Benito** | Ventas/compras y panel de sincronización | Registro y consulta de operaciones (M2), dashboard Flutter con estado de conexión y cola offline, clippy, linters y pruebas de la app. |

---

## 4. Guía de Inicio Rápido (Quickstart)

### Requisitos previos
- Docker & Docker Compose
- Rust 1.80+ y Cargo (opcional para desarrollo local)
- Flutter 3.x (opcional para el cliente)

### 1. Variables de entorno
```bash
cp .env.example .env
```

### 2. Levantar la infraestructura completa con Docker Compose
```bash
docker compose up -d
```
Esto inicializa:
- `couchdb-remote` en el puerto `5984`
- `couchdb-local` en el puerto `5985`
- `mock-hacienda` en el puerto `8081`
- `backend-api` (X1) en el puerto `8080`
- `postgres-control` en el puerto `5432`
- `control-x0` en el puerto `8082`

### 3. Cargar índices y reglas iniciales en CouchDB
```bash
./couchdb/indexes/init_indexes.sh
```

### 4. Probar endpoints principales
```bash
# Verificación de salud y telemetría
curl http://localhost:8080/salud
curl http://localhost:8080/metricas

# Consulta de operaciones por mes (Mango Query)
curl "http://localhost:8080/operaciones?mes=2026-11"

# Resumen de impuestos calculado por el motor fiscal
curl "http://localhost:8080/impuestos/resumen?mes=2026-11"
```

---

## 5. Instrumentación y Calidad (ISO/IEC 25010)

- **Pruebas unitarias de Rust**:
  ```bash
  cargo test --workspace
  ```
- **Linter y estilo Rust**:
  ```bash
  cargo clippy --workspace -- -D warnings
  cargo fmt --check
  ```
- **Análisis estático de Flutter**:
  ```bash
  cd app && flutter analyze
  ```
- **Pruebas de carga (k6)**:
  ```bash
  k6 run bench/k6/load_test.js
  ```
