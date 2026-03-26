-- Auto-generate process_code when NULL (same pattern as quality_rules and applications).
-- Format: BP-{CATEGORY}-{SEQ} (e.g., BP-CORE-00001, BP-RISK-00001)
-- Also adds steward_user_id column to align with other domain entities.

CREATE SEQUENCE IF NOT EXISTS process_code_seq START 1;

CREATE OR REPLACE FUNCTION generate_process_code()
RETURNS TRIGGER AS $$
DECLARE
    v_cat_code VARCHAR(50);
    v_seq_num BIGINT;
BEGIN
    -- Get category name and convert to uppercase code
    SELECT UPPER(REPLACE(LEFT(category_name, 20), ' ', '_')) INTO v_cat_code
    FROM process_categories
    WHERE category_id = NEW.category_id;

    IF v_cat_code IS NULL THEN
        v_cat_code := 'GEN';
    END IF;

    v_seq_num := nextval('process_code_seq');
    NEW.process_code := 'BP-' || v_cat_code || '-' || LPAD(v_seq_num::TEXT, 5, '0');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_generate_process_code ON business_processes;
CREATE TRIGGER trg_generate_process_code
    BEFORE INSERT ON business_processes
    FOR EACH ROW
    WHEN (NEW.process_code IS NULL)
    EXECUTE FUNCTION generate_process_code();

-- Add steward_user_id to align with glossary_terms, data_elements, applications
ALTER TABLE business_processes
    ADD COLUMN IF NOT EXISTS steward_user_id UUID REFERENCES users(user_id);

COMMENT ON COLUMN business_processes.steward_user_id IS 'Data Steward responsible for governance review';
