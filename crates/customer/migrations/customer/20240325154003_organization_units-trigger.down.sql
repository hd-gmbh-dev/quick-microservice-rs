-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_organization_units_update ON organization_units;
DROP FUNCTION IF EXISTS organization_units_update;