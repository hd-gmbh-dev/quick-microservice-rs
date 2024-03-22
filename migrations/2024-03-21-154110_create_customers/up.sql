-- Your SQL goes here
CREATE TABLE customers
(
    id             SERIAL PRIMARY KEY,
    name           VARCHAR(50) NOT NULL CONSTRAINT name_unique UNIQUE,
    created_by     uuid NOT NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by     uuid,
    updated_at     TIMESTAMP
)