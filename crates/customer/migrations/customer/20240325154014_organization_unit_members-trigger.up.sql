-- Add up migration script here
CREATE OR REPLACE FUNCTION organization_unit_members_update() RETURNS TRIGGER AS $$
    DECLARE
    row RECORD;
    output TEXT;
    
    BEGIN
    -- Checking the Operation Type
    IF (TG_OP = 'DELETE') THEN     
      output = '{ "op": "' || TG_OP || '", "old": ' || ROW_TO_JSON(OLD)::text || '}';
    ELSE
      IF (TG_OP = 'UPDATE') THEN
        output = '{ "op": "' || TG_OP || '", "new": ' || ROW_TO_JSON(NEW)::text || ', "old": ' || ROW_TO_JSON(OLD)::text || '}';
      ELSE
        output = '{ "op": "' || TG_OP || '", "new": ' || ROW_TO_JSON(NEW)::text || '}';
      END IF;
    END IF;
    
    -- Calling the pg_notify for my_table_update event with output as payload

    PERFORM pg_notify('organization_unit_members_update', output);
    
    -- Returning null because it is an after trigger.
    RETURN NULL;
    END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER trigger_organization_unit_members_update
  AFTER INSERT OR UPDATE OR DELETE
  ON organization_unit_members
  FOR EACH ROW
  EXECUTE PROCEDURE organization_unit_members_update();