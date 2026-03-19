-- ============================================================================
-- Migration: 019_review_frequency_default.sql
-- Purpose: Calculate next_review_date from created_at (not approved_at)
-- Note: Default review_frequency_id = ANNUAL is set in application code
--       (create_term handler) because PostgreSQL doesn't allow subquery defaults.
-- ============================================================================

-- Update the trigger to use created_at as the base date
CREATE OR REPLACE FUNCTION calculate_next_review_date()
RETURNS TRIGGER AS $$
DECLARE
    v_months INT;
    v_base_date TIMESTAMPTZ;
BEGIN
    -- Recalculate on INSERT or when review_frequency_id/approved_at changes
    IF (TG_OP = 'INSERT') OR
       (NEW.review_frequency_id IS DISTINCT FROM OLD.review_frequency_id) OR
       (NEW.approved_at IS DISTINCT FROM OLD.approved_at) THEN

        IF NEW.review_frequency_id IS NOT NULL THEN
            SELECT months_interval INTO v_months
            FROM glossary_review_frequencies
            WHERE frequency_id = NEW.review_frequency_id;

            -- Base date: approved_at if approved, otherwise created_at
            v_base_date := COALESCE(NEW.approved_at, NEW.created_at, CURRENT_TIMESTAMP);

            NEW.next_review_date := (v_base_date + (v_months || ' months')::INTERVAL)::DATE;
        ELSE
            NEW.next_review_date := NULL;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
