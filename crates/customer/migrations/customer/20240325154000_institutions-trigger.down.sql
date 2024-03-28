-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_institutions_update ON institutions;
DROP FUNCTION IF EXISTS institutions_update;