-- Add up migration script here
CREATE OR REPLACE FUNCTION user_role_mapping_update() RETURNS TRIGGER AS $$
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

    PERFORM pg_notify('user_role_mapping_update', output);
    
    -- Returning null because it is an after trigger.
    RETURN NULL;
    END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER trigger_user_role_mapping_update
  AFTER INSERT OR UPDATE OR DELETE
  ON user_role_mapping
  FOR EACH ROW
  EXECUTE PROCEDURE user_role_mapping_update();