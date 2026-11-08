#!/usr/bin/env bash
set -e

COUCHDB_HOST=${COUCHDB_HOST:-"http://admin:adminsecretpassword@localhost:5984"}
DB_NAME=${DB_NAME:-"negocio_demo"}

echo "Creando base de datos si no existe: ${DB_NAME}"
curl -s -X PUT "${COUCHDB_HOST}/${DB_NAME}" || true

echo "Creando índices Mango..."
curl -s -X POST "${COUCHDB_HOST}/${DB_NAME}/_index" \
     -H "Content-Type: application/json" \
     -d @couchdb/indexes/idx_tipo_fecha.json

curl -s -X POST "${COUCHDB_HOST}/${DB_NAME}/_index" \
     -H "Content-Type: application/json" \
     -d @couchdb/indexes/idx_tipo_estado.json

curl -s -X POST "${COUCHDB_HOST}/${DB_NAME}/_index" \
     -H "Content-Type: application/json" \
     -d @couchdb/indexes/idx_periodo_declaracion.json

echo "Comprobando índices con _explain..."
curl -s -X POST "${COUCHDB_HOST}/${DB_NAME}/_explain" \
     -H "Content-Type: application/json" \
     -d '{"selector":{"tipo":"venta","fecha":{"$gte":"2026-11-01"}}}'

echo "Índices creados exitosamente."
