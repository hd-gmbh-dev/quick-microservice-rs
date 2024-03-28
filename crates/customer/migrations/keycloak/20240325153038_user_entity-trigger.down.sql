-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_user_entity_update ON user_entity;
DROP FUNCTION IF EXISTS user_entity_update;