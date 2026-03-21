-- Migration 033: Auto-generate element_code
--
-- Element code should be auto-generated like term_code and application_code.
-- Pattern: DE-{DOMAIN}-{SEQ} (e.g., DE-FIN-00001)

CREATE SEQUENCE IF NOT EXISTS data_element_code_seq START WITH 1;

CREATE OR REPLACE FUNCTION generate_element_code()
RETURNS TRIGGER AS $$
DECLARE
    v_domain_code VARCHAR(10);
    v_seq_num BIGINT;
BEGIN
    -- Get domain code if domain_id is set
    SELECT domain_code INTO v_domain_code
    FROM glossary_domains
    WHERE domain_id = NEW.domain_id;

    IF v_domain_code IS NULL THEN
        v_domain_code := 'GEN';
    END IF;

    v_seq_num := nextval('data_element_code_seq');

    NEW.element_code := 'DE-' || v_domain_code || '-' || LPAD(v_seq_num::TEXT, 5, '0');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Make element_code nullable so the trigger can populate it
ALTER TABLE data_elements ALTER COLUMN element_code DROP NOT NULL;

-- Create trigger (fires when element_code is NULL on insert)
DROP TRIGGER IF EXISTS trg_generate_element_code ON data_elements;
CREATE TRIGGER trg_generate_element_code
    BEFORE INSERT ON data_elements
    FOR EACH ROW
    WHEN (NEW.element_code IS NULL)
    EXECUTE FUNCTION generate_element_code();

COMMENT ON FUNCTION generate_element_code() IS 'Auto-generates element_code as DE-{DOMAIN}-{SEQ}';
