-- Add down migration script here
DROP INDEX IF EXISTS customer_name_idx;
DROP INDEX IF EXISTS organization_name_idx;
DROP INDEX IF EXISTS institution_name_idx;
DROP INDEX IF EXISTS organization_unit_name_idx;
DROP FUNCTION IF EXISTS edge_gram_tsvector;