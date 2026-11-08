#!/usr/bin/env bash
# Crea bases, seguridad, design docs, indices Mango, usuario de servicio y reglas semilla.
# Idempotente. Uso:  COUCH_ADMIN_USER=.. COUCH_ADMIN_PASS=.. SVC_PASS=.. ./bootstrap.sh <negocio_id>
# Requiere: bash, curl, jq. Ninguna credencial vive en el repositorio (vienen del entorno / .env).
set -euo pipefail

: "${COUCH_URL:=http://localhost:5984}"
: "${COUCH_ADMIN_USER:?falta COUCH_ADMIN_USER}"
: "${COUCH_ADMIN_PASS:?falta COUCH_ADMIN_PASS}"
: "${SVC_USER:=svc_asistente}"
: "${SVC_PASS:?falta SVC_PASS (clave del usuario de servicio del backend Rust)}"
NEGOCIO_ID="${1:?uso: bootstrap.sh <negocio_id>  (a-z, 0-9, _)}"
[[ "$NEGOCIO_ID" =~ ^[a-z0-9_]+$ ]] || { echo "negocio_id invalido" >&2; exit 1; }

HERE="$(cd "$(dirname "$0")/.." && pwd)"
DB_NEGOCIO="negocio_${NEGOCIO_ID}"
ROL_NEGOCIO="negocio_${NEGOCIO_ID}"

cc() { curl -fsS -u "${COUCH_ADMIN_USER}:${COUCH_ADMIN_PASS}" -H 'Content-Type: application/json' "$@"; }

echo "==> Esperando a CouchDB en ${COUCH_URL}"
for _ in $(seq 1 60); do curl -fs "${COUCH_URL}/_up" >/dev/null 2>&1 && break; sleep 1; done
curl -fs "${COUCH_URL}/_up" >/dev/null || { echo "CouchDB no responde" >&2; exit 1; }

ensure_db() {
  local code
  code=$(curl -s -o /dev/null -w '%{http_code}' -u "${COUCH_ADMIN_USER}:${COUCH_ADMIN_PASS}" -X PUT "${COUCH_URL}/$1")
  case "$code" in 201|202|412) echo "   db $1 ok ($code)";; *) echo "Error creando $1: HTTP $code" >&2; exit 1;; esac
}

# upsert <url>  (cuerpo JSON por stdin): conserva _rev si el documento ya existe
upsert() {
  local url="$1" body rev
  body="$(cat)"
  rev="$(curl -s -u "${COUCH_ADMIN_USER}:${COUCH_ADMIN_PASS}" "$url" | jq -r '._rev // empty')"
  if [ -n "$rev" ]; then body="$(jq --arg r "$rev" '._rev=$r' <<<"$body")"; fi
  cc -X PUT "$url" -d "$body" >/dev/null
}

# build_ddoc <carpeta> <con_vista:0|1>  -> imprime el design doc _design/fiscal en JSON
build_ddoc() {
  local dir="$1" with_view="$2"
  if [ "$with_view" = "1" ]; then
    jq -n --rawfile v "$dir/validate_doc_update.js" --rawfile m "$dir/iva_mensual.map.js" \
      '{_id:"_design/fiscal", language:"javascript", validate_doc_update:$v,
        views:{ iva_mensual:{ map:$m, reduce:"_sum" } } }'
  else
    jq -n --rawfile v "$dir/validate_doc_update.js" \
      '{_id:"_design/fiscal", language:"javascript", validate_doc_update:$v}'
  fi
}

create_indexes() { # <db> <archivo>
  jq -c '.[]' "$2" | while read -r idx; do
    r=$(cc -X POST "${COUCH_URL}/$1/_index" -d "$idx" | jq -r '.result')
    echo "   indice $(jq -r .name <<<"$idx") en $1: $r"
  done
}

echo "==> Bases del sistema"
ensure_db _users
ensure_db _replicator
ensure_db _global_changes

echo "==> Usuario de servicio (backend Rust): ${SVC_USER}"
jq -n --arg n "$SVC_USER" --arg p "$SVC_PASS" '{name:$n, password:$p, roles:["fiscal_backend"], type:"user"}' \
  | upsert "${COUCH_URL}/_users/org.couchdb.user:${SVC_USER}"

echo "==> Base ${DB_NEGOCIO}"
ensure_db "$DB_NEGOCIO"
jq -n --arg svc "$SVC_USER" --arg rol "$ROL_NEGOCIO" \
  '{admins:{names:[], roles:["fiscal_admin"]}, members:{names:[$svc], roles:[$rol]}}' \
  | cc -X PUT "${COUCH_URL}/${DB_NEGOCIO}/_security" -d @- >/dev/null
build_ddoc "$HERE/design-docs/negocio" 1 | upsert "${COUCH_URL}/${DB_NEGOCIO}/_design/fiscal"
create_indexes "$DB_NEGOCIO" "$HERE/indexes/negocio_indexes.json"

echo "==> Base reglas_fiscales (solo lectura para usuarios; escritura = fiscal_admin)"
ensure_db reglas_fiscales
jq -n --arg svc "$SVC_USER" \
  '{admins:{names:[], roles:["fiscal_admin"]}, members:{names:[$svc], roles:["fiscal_lector"]}}' \
  | cc -X PUT "${COUCH_URL}/reglas_fiscales/_security" -d @- >/dev/null
build_ddoc "$HERE/design-docs/reglas" 0 | upsert "${COUCH_URL}/reglas_fiscales/_design/fiscal"
create_indexes reglas_fiscales "$HERE/indexes/reglas_indexes.json"
jq -c '.docs[]' "$HERE/seed/reglas_fiscales.json" | while read -r doc; do
  id="$(jq -r '._id' <<<"$doc")"
  enc="$(jq -rn --arg s "$id" '$s|@uri')"
  echo "$doc" | upsert "${COUCH_URL}/reglas_fiscales/${enc}"
  echo "   regla $id"
done

echo "==> Base usuarios_app (NO se replica; hashes argon2 solo viven en el servidor)"
ensure_db usuarios_app
jq -n --arg svc "$SVC_USER" \
  '{admins:{names:[], roles:["fiscal_admin"]}, members:{names:[$svc], roles:[]}}' \
  | cc -X PUT "${COUCH_URL}/usuarios_app/_security" -d @- >/dev/null
build_ddoc "$HERE/design-docs/usuarios" 0 | upsert "${COUCH_URL}/usuarios_app/_design/fiscal"

echo "==> Listo. Siguiente: scripts/explain_check.sh ${NEGOCIO_ID}"
