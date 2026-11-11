# AGENTS.md

Reglas obligatorias para todo el repositorio.

## Estructura

- El workspace de Rust vive en `backend/` (crates en `backend/crates/`).
- Clean Architecture, dependencias permitidas:
  - `domain`: no depende de ningún otro crate del workspace.
  - `application`: solo depende de `domain`.
  - `tax-engine`: solo depende de `domain`.

## Dinero y redondeo

- Todo valor monetario usa `rust_decimal` (`Decimal`). Nunca `f64` para dinero.
- Serializar dinero siempre como string.
- Redondeo half-up a 2 decimales: `MidpointAwayFromZero` (ej. `d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)`).

## Calidad de código

- Prohibido `unwrap()` y `expect()` fuera de tests.
- Errores definidos con `thiserror`.
- Comentarios y mensajes de usuario en español.

## Verificación

Tras cada cambio, ejecutar desde `backend/`:

```bash
cargo fmt && cargo clippy --workspace -- -D warnings && cargo test --workspace
```

El cambio no está terminado hasta que los tres comandos pasen.

## Zonas intocables

No modificar: `backend/crates/infrastructure/`, `backend/crates/api/`, `app/`, `control-x0/`, `mock-hacienda/`.
