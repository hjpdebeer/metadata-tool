-- Migration 026: Auto-generate application_code, remove SLA tier
--
-- Application code should be auto-generated like glossary term_code.
-- SLA tier removed for simplicity — DR tier covers operational classification.

-- 1. Create sequence for application codes
CREATE SEQUENCE IF NOT EXISTS application_code_seq START WITH 1;

-- 2. Create trigger function to auto-generate application_code
CREATE OR REPLACE FUNCTION generate_application_code()
RETURNS TRIGGER AS $$
DECLARE
    v_class_code VARCHAR(10);
    v_seq_num BIGINT;
BEGIN
    -- Get classification code
    SELECT classification_code INTO v_class_code
    FROM application_classifications
    WHERE classification_id = NEW.classification_id;

    -- If no classification, use 'GEN' for general
    IF v_class_code IS NULL THEN
        v_class_code := 'GEN';
    END IF;

    -- Get next sequence number
    v_seq_num := nextval('application_code_seq');

    -- Generate code: APP-{CLASSIFICATION}-{SEQ with leading zeros}
    NEW.application_code := 'APP-' || v_class_code || '-' || LPAD(v_seq_num::TEXT, 5, '0');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 3. Make application_code nullable (trigger will populate it)
ALTER TABLE applications ALTER COLUMN application_code DROP NOT NULL;

-- 4. Create trigger (fires when application_code is NULL on insert)
DROP TRIGGER IF EXISTS trg_generate_application_code ON applications;
CREATE TRIGGER trg_generate_application_code
    BEFORE INSERT ON applications
    FOR EACH ROW
    WHEN (NEW.application_code IS NULL)
    EXECUTE FUNCTION generate_application_code();

-- 5. Remove SLA tier FK from applications
ALTER TABLE applications DROP COLUMN IF EXISTS sla_tier_id;

-- 6. Drop SLA tiers table (no longer needed)
DROP TABLE IF EXISTS sla_tiers;

COMMENT ON FUNCTION generate_application_code() IS 'Auto-generates application_code as APP-{CLASSIFICATION}-{SEQ}';
