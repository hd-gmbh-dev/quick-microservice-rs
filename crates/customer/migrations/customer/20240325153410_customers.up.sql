-- Add up migration script here
CREATE TABLE IF NOT EXISTS customers
(
    id             BIGSERIAL PRIMARY KEY,
    name           VARCHAR(255) NOT NULL CONSTRAINT customers_name_unique UNIQUE,
    ty             VARCHAR(255) NOT NULL,
    created_by     uuid NOT NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by     uuid,
    updated_at     TIMESTAMP
);