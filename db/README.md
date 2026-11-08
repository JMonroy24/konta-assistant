# Scripts de base de datos - Asistente fiscal (Fase 4)

## Contenido
```
couchdb/
  config/local.ini                      configuracion minima (sin secretos)
  design-docs/negocio/                  validate_doc_update.js + iva_mensual.map.js (reduce _sum, centavos enteros)
  design-docs/reglas/                   validate de reglas_fiscales (solo fiscal_admin escribe)
  design-docs/usuarios/                 validate de usuarios_app (hash argon2, rol, negocios)
  indexes/                              indices Mango (negocio y reglas)
  seed/reglas_fiscales.json             reglas semilla (TODAS verificada:false)
  seed/generate_seed.py                 datos sinteticos reproducibles (--seed)
  scripts/bootstrap.sh                  crea bases, seguridad, design docs, indices, usuario de servicio, reglas
  scripts/replicate.sh                  replicacion continua local<->remoto (+ reglas remoto->local)
  scripts/explain_check.sh              comprueba con _explain que cada consulta usa su indice
  scripts/vista_iva.sh                  consulta de la vista de totales mensuales
control-x0/db/
  000_roles.sh  001_schema.sql  002_seed_reglas.sql (generado del mismo JSON)  003_grants.sql
```

## Orden de ejecucion
```bash
export COUCH_ADMIN_USER=admin COUCH_ADMIN_PASS=...   SVC_PASS=...     # desde .env, nunca en git
./couchdb/scripts/bootstrap.sh demo1
python3 couchdb/seed/generate_seed.py demo1 --post http://localhost:5984
./couchdb/scripts/explain_check.sh demo1                               # debe terminar con codigo 0
./couchdb/scripts/vista_iva.sh demo1 2026-10
# X0 (Postgres): montar control-x0/db en /docker-entrypoint-initdb.d/ ; los archivos corren en orden 000..003
```

## Antes de usar las reglas fiscales
Los parametros de `reglas_fiscales.json` son valores de partida cargados de memoria. Verificarlos con la
normativa vigente de Hacienda, completar `fuente_normativa` con el articulo exacto y recien entonces
poner `verificada: true`. `multa_extemporanea` es un placeholder vacio a proposito.
