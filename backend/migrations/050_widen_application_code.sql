-- Fix application_code auto-generation: the trigger's internal variable
-- v_class_code was VARCHAR(10) but classification codes like CORE_BANKING (12),
-- DATA_ANALYTICS (14), INFRASTRUCTURE (14) exceed that limit.

CREATE OR REPLACE FUNCTION generate_application_code()
RETURNS TRIGGER AS $$
DECLARE
    v_class_code VARCHAR(50);
    v_seq_num BIGINT;
BEGIN
    SELECT classification_code INTO v_class_code
    FROM application_classifications
    WHERE classification_id = NEW.classification_id;

    IF v_class_code IS NULL THEN
        v_class_code := 'GEN';
    END IF;

    v_seq_num := nextval('application_code_seq');
    NEW.application_code := 'APP-' || v_class_code || '-' || LPAD(v_seq_num::TEXT, 5, '0');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
