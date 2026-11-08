#!/usr/bin/env bash
# Ejemplo: totales mensuales (centavos) por tipo desde la vista map/reduce.
# Uso: ./vista_iva.sh <negocio_id> [AAAA-MM]
set -euo pipefail
: "${COUCH_URL:=http://localhost:5984}"; : "${COUCH_ADMIN_USER:?}"; : "${COUCH_ADMIN_PASS:?}"
DB="negocio_${1:?}"; MES="${2:-}"
Q="group_level=2"
[ -n "$MES" ] && Q="$Q&startkey=%5B%22$MES%22%5D&endkey=%5B%22$MES%22,%7B%7D%5D"
curl -fsS -u "$COUCH_ADMIN_USER:$COUCH_ADMIN_PASS" "$COUCH_URL/$DB/_design/fiscal/_view/iva_mensual?$Q" \
 | jq '.rows[] | {periodo:.key[0], tipo:.key[1], base_cent:.value[0], iva_cent:.value[1], total_cent:.value[2]}'
