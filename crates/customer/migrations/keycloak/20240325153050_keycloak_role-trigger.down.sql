-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_keycloak_role_update ON keycloak_role;
DROP FUNCTION IF EXISTS keycloak_role_update;