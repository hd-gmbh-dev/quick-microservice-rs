-- Add up migration script here
CREATE TABLE IF NOT EXISTS organizations
(
    id             BIGSERIAL PRIMARY KEY,
    customer_id    BIGINT NOT NULL,
    name           VARCHAR(255) NOT NULL,
    created_by     uuid NOT NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by     uuid,
    updated_at     TIMESTAMP,
    UNIQUE(customer_id, name),
    FOREIGN KEY(customer_id) 
       REFERENCES customers(id)
       ON DELETE CASCADE
);

