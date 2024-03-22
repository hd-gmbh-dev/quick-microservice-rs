-- Your SQL goes here
CREATE TABLE organization_unit_members
(
    organization_unit_id INTEGER NOT NULL REFERENCES organization_units (id),
    customer_id          INTEGER NOT NULL REFERENCES customers (id),
    organization_id      INTEGER NOT NULL REFERENCES organizations (id),
    institution_id       INTEGER NOT NULL REFERENCES institutions (id),
    PRIMARY KEY(organization_unit_id, customer_id, organization_id, institution_id)
)
