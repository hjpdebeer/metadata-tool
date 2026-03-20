-- Migration 028: Fix search vector trigger after golden_source column removal
--
-- The search vector trigger still referenced NEW.golden_source which was
-- dropped in migration 027.

CREATE OR REPLACE FUNCTION update_glossary_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.term_name, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.abbreviation, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.term_code, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.definition, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.definition_notes, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.business_context, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.examples, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.counter_examples, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.formula, '')), 'D') ||
        setweight(to_tsvector('english', COALESCE(NEW.regulatory_reporting_usage, '')), 'D') ||
        setweight(to_tsvector('english', COALESCE(NEW.external_reference, '')), 'D');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
