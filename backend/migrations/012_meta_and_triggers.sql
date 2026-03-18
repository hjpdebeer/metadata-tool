-- Self-describing metadata

CREATE TABLE meta_tables (
    meta_table_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_name   VARCHAR(128) NOT NULL DEFAULT 'public',
    table_name    VARCHAR(128) NOT NULL,
    display_name  VARCHAR(256) NOT NULL,
    description   TEXT NOT NULL,
    domain        VARCHAR(128),
    is_reference  BOOLEAN NOT NULL DEFAULT FALSE,
    is_append_only BOOLEAN NOT NULL DEFAULT FALSE,
    change_frequency VARCHAR(20),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(schema_name, table_name)
);

CREATE TABLE meta_columns (
    meta_column_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    meta_table_id  UUID NOT NULL REFERENCES meta_tables(meta_table_id),
    column_name    VARCHAR(128) NOT NULL,
    display_name   VARCHAR(256) NOT NULL,
    description    TEXT NOT NULL,
    data_type      VARCHAR(64) NOT NULL,
    is_required    BOOLEAN NOT NULL DEFAULT FALSE,
    is_primary_key BOOLEAN NOT NULL DEFAULT FALSE,
    is_foreign_key BOOLEAN NOT NULL DEFAULT FALSE,
    fk_references  VARCHAR(256),
    example_value  VARCHAR(512),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(meta_table_id, column_name)
);

-- Auto-update timestamps trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at trigger to all applicable tables
DO $$
DECLARE
    tbl TEXT;
BEGIN
    FOR tbl IN
        SELECT table_name
        FROM information_schema.columns
        WHERE table_schema = 'public'
        AND column_name = 'updated_at'
        AND table_name NOT LIKE 'meta_%'
    LOOP
        EXECUTE format('
            CREATE TRIGGER trg_%s_updated_at
                BEFORE UPDATE ON %I
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
        ', tbl, tbl);
    END LOOP;
END;
$$;

-- Full-text search trigger for glossary_terms
CREATE OR REPLACE FUNCTION update_glossary_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.term_name, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.definition, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.business_context, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.examples, '')), 'D');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_glossary_terms_search
    BEFORE INSERT OR UPDATE OF term_name, definition, business_context, examples
    ON glossary_terms
    FOR EACH ROW
    EXECUTE FUNCTION update_glossary_search_vector();

-- Full-text search trigger for data_elements
CREATE OR REPLACE FUNCTION update_element_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.element_name, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.element_code, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.business_definition, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.business_rules, '')), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_data_elements_search
    BEFORE INSERT OR UPDATE OF element_name, element_code, description, business_definition, business_rules
    ON data_elements
    FOR EACH ROW
    EXECUTE FUNCTION update_element_search_vector();

-- Naming standard validation trigger for technical columns
CREATE OR REPLACE FUNCTION validate_column_naming_standards()
RETURNS TRIGGER AS $$
DECLARE
    standard RECORD;
    is_compliant BOOLEAN := TRUE;
    violation_msg TEXT := '';
BEGIN
    FOR standard IN
        SELECT pattern_regex, standard_name
        FROM naming_standards
        WHERE applies_to = 'COLUMN' AND is_mandatory = TRUE AND deleted_at IS NULL
    LOOP
        IF NOT (NEW.column_name ~ standard.pattern_regex) THEN
            is_compliant := FALSE;
            violation_msg := violation_msg || standard.standard_name || '; ';
        END IF;
    END LOOP;

    NEW.naming_standard_compliant := is_compliant;
    NEW.naming_standard_violation := NULLIF(TRIM(TRAILING '; ' FROM violation_msg), '');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_columns_naming_validation
    BEFORE INSERT OR UPDATE OF column_name ON technical_columns
    FOR EACH ROW
    EXECUTE FUNCTION validate_column_naming_standards();
