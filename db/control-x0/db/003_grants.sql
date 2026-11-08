-- Permisos minimos para el rol de aplicacion del backend X0 (ejecutar despues de 001 y 002).
GRANT USAGE ON SCHEMA fiscal TO app_x0;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA fiscal TO app_x0;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA fiscal TO app_x0;
REVOKE INSERT, UPDATE, DELETE ON fiscal.reglas_fiscales FROM app_x0;   -- reglas: solo administracion fiscal
ALTER ROLE app_x0 SET search_path = fiscal, public;
