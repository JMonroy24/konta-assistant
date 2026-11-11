# ADR-005: Versionado y verificación de reglas fiscales

- **Estado:** Aceptado
- **Fecha:** 2026-11-11

## Contexto

Las reglas fiscales contienen tasas, tramos y plazos sujetos a cambios
normativos. El seed puede incluir datos de referencia que todavía no han sido
contrastados con una fuente oficial; no deben confundirse con reglas aptas para
producir resultados fiscales.

## Decisión

1. Cada cambio normativo o corrección material se publica como una nueva versión
   de la regla. Las versiones anteriores se conservan para explicar y reproducir
   cálculos históricos; no se sobrescriben para cambiar su significado.
2. `vigencia_desde` y `vigencia_hasta` delimitan el intervalo en que aplica cada
   versión. `vigencia_hasta: null` significa que no se ha fijado un fin de
   vigencia. Los intervalos se interpretan inclusivamente; las versiones no
   deben tener vigencias contradictorias para el mismo código.
3. Toda fuente se documenta como `Ley/Código, art., fecha de consulta`. Los
   datos faltantes se marcan `TODO`, sin inferir la fuente ni alterar valores.
   `verificada_por` y `fecha_verificacion` identifican a quien verificó y la
   fecha de esa revisión; permanecen `null` mientras no haya verificación.
4. Ningún valor de una regla se usa en producción mientras
   `verificada != true`. La ausencia de verificación, los campos `TODO` o el
   estado `por_definir` excluyen la regla del cálculo productivo. Solo después
   de contrastar la fuente oficial, vigencia y parámetros se registra quién y
   cuándo verificó, y se habilita expresamente la regla con `verificada: true`.

## Consecuencias

- El seed es informativo y no constituye una fuente normativa verificada.
- Al cambiar una regla se conserva la trazabilidad mediante una nueva versión y
  su intervalo de vigencia.
- Los consumidores de reglas deben filtrar por código, fecha de vigencia y
  `verificada: true` antes de usar parámetros en producción.
