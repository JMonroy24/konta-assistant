# Metrología e Instrumentación (ISO/IEC 25010)

Este directorio contiene herramientas para evaluar desempeño, consumo de recursos y resiliencia.

## Prueba de carga con k6

Se eligió k6 sobre Apache JMeter porque su motor ligero reduce el consumo de recursos
del generador durante las pruebas, sus escenarios se definen como código versionable y
exporta métricas de forma nativa a JSON. Así se puede mantener reproducible la misma
carga para X0 y X1 y reservar más recursos del equipo para los sistemas bajo prueba.

Cada ejecución envía cargas JSON a `POST /ventas`, `POST /compras` y `POST /facturas`,
que representa una venta, una compra con IVA retenido y una factura electrónica simulada.
Cada tipo corre con el mismo número fijo de iteraciones y VUs. Usa los mismos valores
para ambas APIs: por defecto se envían 1.000 solicitudes de cada tipo (3.000 en total),
sin depender de la velocidad de respuesta del sistema.

```bash
mkdir -p bench/results

ITERATIONS_PER_TYPE=1000 VUS_PER_TYPE=1 \
TEST_SCENARIO=baseline API_URL=http://localhost:8080 \
SUMMARY_JSON=bench/results/x1-summary.json \
k6 run --out json=bench/results/x1-timeseries.json bench/k6/load_test.js

ITERATIONS_PER_TYPE=1000 VUS_PER_TYPE=1 \
TEST_SCENARIO=baseline API_URL=http://localhost:8082 \
SUMMARY_JSON=bench/results/x0-summary.json \
k6 run --out json=bench/results/x0-timeseries.json bench/k6/load_test.js
```

`documents_processed` cuenta solicitudes 2xx aceptadas y `documents_attempted` el
volumen total; la tasa reportada equivale a documentos por segundo. `document_latency`
reporta p50, p95 y p99. El JSON final incluye `workload` (volumen planeado) y
`resilience.success_percent`, calculado como solicitudes 2xx dividido por intentadas;
los timeouts y respuestas no 2xx cuentan como fallidas. El JSON de serie temporal se
genera aparte. El umbral de latencia configura una meta, no altera el volumen fijo.

Para registrar resiliencia bajo degradación, ejecuta cada prueba en un entorno
controlado, aplica las mismas condiciones de red a X0 y X1, y etiqueta los resúmenes:

```bash
sudo bash bench/network/simulate_packet_loss_tc.sh eth0 loss
TEST_SCENARIO=degraded-loss API_URL=http://localhost:8080 \
SUMMARY_JSON=bench/results/x1-loss-summary.json \
k6 run --out json=bench/results/x1-loss-timeseries.json bench/k6/load_test.js
sudo bash bench/network/simulate_packet_loss_tc.sh eth0 reset
```

Repite para X0 y para el escenario `delay` si aplica. `eth0` debe ser la interfaz
adecuada para el tráfico probado; `tc` requiere privilegios y afecta el tráfico de esa
interfaz. Restaura la red con `reset` incluso si la prueba falla.

Para consultar un resumen compacto en tablas Markdown, ejecuta:

```bash
python3 bench/report.py
```

El informe se guarda en `bench/results/readable-summary.md`. Resume latencias, throughput, checks HTTP,
resiliencia, recursos por contenedor y las muestras de latencia de replicación; los
JSON y CSV originales se conservan para análisis detallado.

La carga es deliberadamente moderada (hasta 10 VUs). Ajusta `stages` para el experimento,
mantén iguales las etapas entre ejecuciones y no apuntes el generador a servicios ajenos
o de producción.

> **Limitación del estado actual:** las rutas de X1 devuelven respuestas provisionales y
> no persisten operaciones; X0 solo tiene una respuesta provisional para `POST /ventas`.
> Por eso estos resultados miden el camino HTTP actual, no una comparación válida de
> persistencia CouchDB frente a PostgreSQL. Repite el experimento de persistencia cuando
> ambos backends implementen almacenamiento real.

## Monitoreo de CPU y RSS de contenedores

Ejecuta el monitor en otra terminal mientras k6 está corriendo. En Linux con cgroups v2,
lee el RSS real desde `VmRSS` de cada proceso del cgroup del contenedor y suma esos
procesos; no usa la memoria genérica reportada por `docker stats`. CPU sí se toma de
`docker stats`. El CSV incluye una muestra por contenedor cada segundo por defecto.
El archivo se crea bajo `bench/results/` con nombre UTC; se puede proporcionar una ruta.

```bash
INTERVAL_SECONDS=1 bash bench/monitor/docker_stats.sh
# Muestra única útil para verificar el acceso a cgroups:
bash bench/monitor/docker_stats.sh --once /tmp/container-rss.csv
```

El host debe exponer `/sys/fs/cgroup` en modo v2 y permitir leer `/proc/<pid>/status` y
los `cgroup.procs` de los contenedores. Si esa información no está disponible, el
monitor falla explícitamente en vez de guardar una memoria aproximada como RSS.

## Latencia de replicación CouchDB (X1)

El medidor crea una base de datos de benchmark aislada en ambos CouchDB, inserta
documentos con `timestamp_origen` en el nodo local, solicita una replicación puntual al
remoto y espera el documento en su feed `_changes`. La latencia es la diferencia entre
el timestamp de origen y el instante en que el script observa el documento en el feed.
Se requiere que ambos nodos estén disponibles; el usuario y contraseña se configuran
con las mismas variables de entorno que Docker Compose.

```bash
COUCHDB_LOCAL_URL=http://localhost:5985 \
COUCHDB_REMOTE_URL=http://localhost:5984 \
COUCHDB_USER=admin COUCHDB_PASSWORD=adminsecretpassword \
python3 bench/replication_latency.py --count 50 --timeout 60
```

`COUCHDB_DATABASE` cambia la base aislada (por defecto `bench-replication`).
`--output` permite elegir el archivo CSV de salida.

## Telemetría de la API Rust

El suscriptor de la API Axum está configurado para emitir JSON mediante `tracing` y
`tracing-subscriber`. El middleware de
`backend/crates/api/src/telemetry.rs` ya registra método, ruta, estado HTTP y duración,
pero actualmente los separa en dos eventos: la ruta y el método aparecen al entrar; el
estado y la duración, al responder. Para cumplir estrictamente con un registro por
petición que contenga simultáneamente `endpoint`, `metodo`, `status` y `duracion_ms`,
queda pendiente consolidar esos campos en un solo evento estructurado.

Para ver los eventos JSON en Docker:

```bash
docker compose logs -f backend-api
```

## Otras verificaciones

- **Fiabilidad**: `bench/network/simulate_packet_loss_tc.sh` prueba degradación de red.
- **Mantenibilidad**: `cargo clippy --workspace -- -D warnings` y `flutter analyze`.
- **Adecuación funcional**: `cargo test --workspace`.
