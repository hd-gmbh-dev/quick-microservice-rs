-- Add up migration script here
CREATE OR REPLACE FUNCTION edge_gram_tsvector(text text) RETURNS tsvector AS
$BODY$
BEGIN
    RETURN (select array_to_tsvector((select array_agg(distinct substring(lexeme for len)) from unnest(to_tsvector(text)), generate_series(1,length(lexeme)) len)));
END;
$BODY$
IMMUTABLE
language plpgsql;
CREATE INDEX IF NOT EXISTS customer_name_idx on customers USING gin(edge_gram_tsvector(name));
CREATE INDEX IF NOT EXISTS organization_name_idx on organizations USING gin(edge_gram_tsvector(name));
CREATE INDEX IF NOT EXISTS institution_name_idx on institutions USING gin(edge_gram_tsvector(name));
CREATE INDEX IF NOT EXISTS organization_unit_name_idx on organization_units USING gin(edge_gram_tsvector(name));