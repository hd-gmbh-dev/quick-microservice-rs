-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_group_role_mapping_update ON group_role_mapping;
DROP FUNCTION IF EXISTS group_role_mapping_update;