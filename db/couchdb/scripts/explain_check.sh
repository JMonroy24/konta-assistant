#!/usr/bin/env bash
# Verifica con _explain que cada consulta canonica usa el indice esperado (requisito de la rubrica).
# Sale con codigo 1 si alguna cae en all_docs o en otro indice. Uso: ./explain_check.sh <negocio_id>
set -euo pipefail
: "${COUCH_URL:=http://localhost:5984}"
: "${COUCH_ADMIN_USER:?}" "${COUCH_ADMIN_PASS:?}"
DB="negocio_${1:?uso: explain_check.sh <negocio_id>}"

# nombre_esperado|cuerpo de la consulta
CASOS=(
'idx_tipo_fecha|{"selector":{"tipo":"venta","fecha":{"$gte":"2026-11-01","$lt":"2026-12-01"}},"sort":[{"tipo":"asc"},{"fecha":"asc"}],"use_index":"idx_tipo_fecha","limit":200}'
'idx_tipo_fecha|{"selector":{"tipo":"compra","fecha":{"$gte":"2026-11-01","$lt":"2026-12-01"}},"sort":[{"tipo":"asc"},{"fecha":"asc"}],"use_index":"idx_tipo_fecha","limit":200}'
'idx_tipo_estado|{"selector":{"tipo":"factura","estado":"pendiente"},"use_index":"idx_tipo_estado"}'
'idx_tipo_periodo|{"selector":{"tipo":"declaracion","periodo":"2026-10"},"use_index":"idx_tipo_periodo"}'
'idx_recordatorio_activos|{"selector":{"tipo":"recordatorio","estado":"pendiente","fecha_limite":{"$lte":"2026-12-31"}},"sort":[{"tipo":"asc"},{"estado":"asc"},{"fecha_limite":"asc"}],"use_index":"idx_recordatorio_activos"}'
'idx_factura_venta|{"selector":{"tipo":"factura","venta_id":"venta:2026-11-07:7f3c"},"use_index":"idx_factura_venta"}'
)

fallos=0
for caso in "${CASOS[@]}"; do
  esperado="${caso%%|*}"; cuerpo="${caso#*|}"
  real=$(curl -fsS -u "${COUCH_ADMIN_USER}:${COUCH_ADMIN_PASS}" -H 'Content-Type: application/json' \
         -X POST "${COUCH_URL}/${DB}/_explain" -d "$cuerpo" | jq -r '.index.name')
  if [ "$real" = "$esperado" ]; then echo "OK    $esperado"; else echo "FALLA esperado=$esperado real=$real"; fallos=1; fi
done
exit $fallos
