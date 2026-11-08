#!/usr/bin/env bash
# Para /docker-entrypoint-initdb.d/ (se ejecuta antes que los .sql por orden alfabetico).
# La clave viene de X0_APP_PASSWORD (.env fuera del repo). Rol de aplicacion SIN superusuario.
set -euo pipefail
: "${X0_APP_PASSWORD:?falta X0_APP_PASSWORD}"
psql -v ON_ERROR_STOP=1 -v app_password="$X0_APP_PASSWORD" --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<'SQL'
CREATE ROLE app_x0 LOGIN PASSWORD :'app_password' NOSUPERUSER NOCREATEDB NOCREATEROLE;
SQL
