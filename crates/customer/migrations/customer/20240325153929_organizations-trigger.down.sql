-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_organizations_update ON organizations;
DROP FUNCTION IF EXISTS organizations_update;