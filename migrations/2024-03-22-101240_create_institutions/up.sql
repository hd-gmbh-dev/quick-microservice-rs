-- Your SQL goes here
CREATE TABLE institutions
(
    id                 SERIAL PRIMARY KEY,
    customer_id        INTEGER NOT NULL REFERENCES customers (id),
    organization_id    INTEGER NOT NULL REFERENCES organizations (id),
    name               VARCHAR(50) NOT NULL,
    created_by         uuid NOT NULL,
    created_at         TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by         uuid,
    updated_at         TIMESTAMP,
    UNIQUE(organization_id, name)
)