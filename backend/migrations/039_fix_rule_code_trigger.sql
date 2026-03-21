-- Migration 039: Fix quality rule code trigger variable size
--
-- The v_dim_code variable was VARCHAR(10) but dimension codes like
-- COMPLETENESS (12 chars) and CONSISTENCY (11 chars) overflow it.

CREATE OR REPLACE FUNCTION generate_quality_rule_code()
RETURNS TRIGGER AS $$
DECLARE
    v_dim_code VARCHAR(50);
    v_seq_num BIGINT;
BEGIN
    SELECT dimension_code INTO v_dim_code
    FROM quality_dimensions
    WHERE dimension_id = NEW.dimension_id;

    IF v_dim_code IS NULL THEN
        v_dim_code := 'GEN';
    END IF;

    v_seq_num := nextval('quality_rule_code_seq');
    NEW.rule_code := 'QR-' || v_dim_code || '-' || LPAD(v_seq_num::TEXT, 5, '0');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
