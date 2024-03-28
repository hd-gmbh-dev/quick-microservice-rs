-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_organization_unit_members_update ON organization_unit_members;
DROP FUNCTION IF EXISTS organization_unit_members_update;