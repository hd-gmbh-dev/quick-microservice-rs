-- Your SQL goes here
CREATE TABLE organizations
(
    id             SERIAL PRIMARY KEY,
    customer_id    INTEGER NOT NULL REFERENCES customers (id),
    name           VARCHAR(50) NOT NULL,
    created_by     uuid NOT NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by     uuid,
    updated_at     TIMESTAMP,
    UNIQUE(customer_id, name)
)