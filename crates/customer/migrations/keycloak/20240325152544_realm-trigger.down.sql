-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_realm_update ON realm;
DROP FUNCTION IF EXISTS realm_update;