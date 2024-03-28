-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_user_role_mapping_update ON user_role_mapping;
DROP FUNCTION IF EXISTS user_role_mapping_update;