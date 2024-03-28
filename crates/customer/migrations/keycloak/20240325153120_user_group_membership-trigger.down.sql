-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_user_group_membership_update ON user_group_membership;
DROP FUNCTION IF EXISTS user_group_membership_update;