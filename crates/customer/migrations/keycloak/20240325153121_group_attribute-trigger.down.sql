-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_group_attribute_update ON group_attribute;
DROP FUNCTION IF EXISTS group_attribute_update;