-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_keycloak_group_update ON keycloak_group;
DROP FUNCTION IF EXISTS keycloak_group_update;