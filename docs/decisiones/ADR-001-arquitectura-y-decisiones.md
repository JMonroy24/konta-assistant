# Decisiones Arquitectónicas (ADR)

## ADR-001: Clean Architecture y Monorepo
- **Contexto**: Se requiere aislar el dominio y permitir una comparación experimental justa entre X1 (CouchDB) y X0 (PostgreSQL).
- **Decisión**: Monorepo con backend estructurado en crates: `domain` (sin dependencias externas), `application` (traits/puertos y casos de uso), `infrastructure` (adaptadores CouchDB y Reqwest), `tax-engine` (motor de reglas) y `api` (Axum).
- **Consecuencias**: `domain` permanece puro; X1 y X0 implementan los mismos traits de `application`.

## ADR-002: Estrategia de Persistencia y Replicación CouchDB
- **Contexto**: CouchDB maneja documentos flexibles JSON y replicación maestro-maestro con resolución de revisiones.
- **Decisión**: Una base de datos `negocio_{id}` por microempresa para aislamiento y privacidad, una base `reglas_fiscales` de solo lectura replicada desde remoto, y `_users` sin replicar.
- **Validaciones**: Se implementa `validate_doc_update` en CouchDB además de la validación estricta de tipos en Rust.

## ADR-003: Estrategia Cliente Multiplataforma y Modo Offline
- **Contexto**: PouchDB no se usa en Flutter.
- **Decisión**: En escritorio y sandbox experimental se utiliza CouchDB local con replicación bidireccional continua. En móvil y web se implementa la cola local (`OfflineQueue`) con identificadores UUID únicos generados en el cliente para garantizar idempotencia al sincronizar con la API de Rust.
- **Limitación declarada**: Documentado formalmente como limitación de arquitectura en el informe.

## ADR-004: Simulador de Validación de Hacienda (Mock)
- **Contexto**: No existe conexión legal/productiva abierta con el Ministerio de Hacienda para este alcance de Fase 4.
- **Decisión**: Se implementa un servicio mock en Rust (`mock-hacienda`) que responde validaciones DTE simuladas. La app rotula explícitamente todo documento como "Aceptado Simulado" y nunca como validación oficial.
