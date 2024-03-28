-- Add up migration script here
CREATE TABLE IF NOT EXISTS organization_unit_members
(
    organization_unit_id BIGINT NOT NULL,
    customer_id          BIGINT NOT NULL,
    organization_id      BIGINT NOT NULL,
    institution_id       BIGINT NOT NULL,
    PRIMARY KEY(organization_unit_id, customer_id, organization_id, institution_id),
    FOREIGN KEY(organization_unit_id)
       REFERENCES organization_units(id)
       ON DELETE CASCADE,
    FOREIGN KEY(customer_id)
       REFERENCES customers(id)
       ON DELETE CASCADE,
    FOREIGN KEY(organization_id)
       REFERENCES organizations(id)
       ON DELETE CASCADE,
    FOREIGN KEY(institution_id)
       REFERENCES institutions(id)
       ON DELETE CASCADE
);