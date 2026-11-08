# Metrología e Instrumentación (ISO/IEC 25010)

Este directorio contiene los scripts y pruebas para evaluar las métricas requeridas por la Fase 4:

1. **Eficiencia de desempeño**:
   - Medición de Throughput y Latencia (p50, p95, p99) con `k6 run bench/k6/load_test.js`.
   - Medición de uso de recursos con `docker stats --no-stream`.
2. **Fiabilidad**:
   - Pruebas de resiliencia ante pérdida de conexión y degradación de red con `bench/network/simulate_packet_loss_tc.sh`.
3. **Mantenibilidad**:
   - Verificación estática con `cargo clippy --workspace -- -D warnings` y `flutter analyze`.
4. **Adecuación funcional**:
   - Pruebas unitarias de casos extremos y redondeos con `cargo test --workspace`.
