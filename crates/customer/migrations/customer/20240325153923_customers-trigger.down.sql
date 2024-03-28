-- Add down migration script here
DROP TRIGGER IF EXISTS trigger_customers_update ON customers;
DROP FUNCTION IF EXISTS customers_update;