#!/usr/bin/env bash
# Crea documentos en _replicator del CouchDB LOCAL (escritorio / sandbox X1):
#   negocio_<id>:   local <-> remoto (continua, bidireccional)
#   reglas_fiscales: remoto -> local (solo bajada)
# Uso: LOCAL_URL=.. LOCAL_USER=.. LOCAL_PASS=.. REMOTE_URL=.. REMOTE_USER=.. REMOTE_PASS=.. ./replicate.sh <negocio_id>
set -euo pipefail
: "${LOCAL_URL:=http://localhost:5984}"
: "${LOCAL_USER:?}" "${LOCAL_PASS:?}" "${REMOTE_URL:?}" "${REMOTE_USER:?}" "${REMOTE_PASS:?}"
NEGOCIO_ID="${1:?uso: replicate.sh <negocio_id>}"
DB="negocio_${NEGOCIO_ID}"

ep() { # <url-base> <db> <user> <pass>
  jq -n --arg u "$1/$2" --arg us "$3" --arg pw "$4" '{url:$u, auth:{basic:{username:$us, password:$pw}}}'
}
upsert_repl() { # <id> <json>
  local rev
  rev="$(curl -s -u "$LOCAL_USER:$LOCAL_PASS" "$LOCAL_URL/_replicator/$1" | jq -r '._rev // empty')"
  [ -n "$rev" ] && set -- "$1" "$(jq --arg r "$rev" '._rev=$r' <<<"$2")"
  curl -fsS -u "$LOCAL_USER:$LOCAL_PASS" -H 'Content-Type: application/json' -X PUT "$LOCAL_URL/_replicator/$1" -d "$2" >/dev/null
  echo "   replicacion $1 configurada"
}

# Selector explicito: NO replica design docs (el usuario sin rol admin no podria escribirlos en destino
# y la replicacion quedaria con doc_write_failures). Los design docs se crean con bootstrap.sh en cada lado.
SEL_NEGOCIO='{"_id":{"$regex":"^(negocio|venta|compra|factura|declaracion|recordatorio):"}}'
COMUN='{"continuous":true,"create_target":false,"use_checkpoints":true,"checkpoint_interval":5000,
        "retries_per_request":10,"worker_processes":4,"http_connections":20,"connection_timeout":60000}'

upsert_repl "push_${DB}" "$(jq -n --argjson c "$COMUN" --argjson s "$SEL_NEGOCIO" \
  --argjson src "$(ep "$LOCAL_URL" "$DB" "$LOCAL_USER" "$LOCAL_PASS")" \
  --argjson tgt "$(ep "$REMOTE_URL" "$DB" "$REMOTE_USER" "$REMOTE_PASS")" \
  '$c + {source:$src, target:$tgt, selector:$s}')"
upsert_repl "pull_${DB}" "$(jq -n --argjson c "$COMUN" --argjson s "$SEL_NEGOCIO" \
  --argjson src "$(ep "$REMOTE_URL" "$DB" "$REMOTE_USER" "$REMOTE_PASS")" \
  --argjson tgt "$(ep "$LOCAL_URL" "$DB" "$LOCAL_USER" "$LOCAL_PASS")" \
  '$c + {source:$src, target:$tgt, selector:$s}')"
upsert_repl "pull_reglas_fiscales" "$(jq -n --argjson c "$COMUN" \
  --argjson src "$(ep "$REMOTE_URL" reglas_fiscales "$REMOTE_USER" "$REMOTE_PASS")" \
  --argjson tgt "$(ep "$LOCAL_URL" reglas_fiscales "$LOCAL_USER" "$LOCAL_PASS")" \
  '$c + {source:$src, target:$tgt, selector:{"tipo":"regla_fiscal"}}')"
echo "Estado: curl -u ... $LOCAL_URL/_scheduler/docs"
