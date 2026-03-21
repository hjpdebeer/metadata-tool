-- Migration 041: Fix classification inheritance — term classification always overrides
--
-- A Confidential business term should not have Internal data elements.
-- The term's classification always takes precedence over the element's.

CREATE OR REPLACE FUNCTION propagate_term_to_element()
RETURNS TRIGGER AS $$
DECLARE
    v_is_cbt BOOLEAN;
    v_classification_id UUID;
BEGIN
    -- Only act if glossary_term_id changed
    IF NEW.glossary_term_id IS DISTINCT FROM OLD.glossary_term_id THEN
        IF NEW.glossary_term_id IS NOT NULL THEN
            -- Look up the linked term's CBT flag and classification
            SELECT is_cbt, classification_id
            INTO v_is_cbt, v_classification_id
            FROM glossary_terms
            WHERE term_id = NEW.glossary_term_id
              AND deleted_at IS NULL
              AND is_current_version = TRUE;

            -- Inherit CDE from CBT
            IF v_is_cbt = TRUE THEN
                NEW.is_cde := TRUE;
                NEW.cde_rationale := COALESCE(
                    NEW.cde_rationale,
                    'Auto-designated: inherited from Critical Business Term (ADR-0005)'
                );
                NEW.cde_designated_at := COALESCE(NEW.cde_designated_at, CURRENT_TIMESTAMP);
            END IF;

            -- Always inherit classification from term (term is the governing concept)
            IF v_classification_id IS NOT NULL THEN
                NEW.classification_id := v_classification_id;
            END IF;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Backfill: fix existing elements that have a different classification than their term
UPDATE data_elements de
SET classification_id = gt.classification_id,
    updated_at = CURRENT_TIMESTAMP
FROM glossary_terms gt
WHERE gt.term_id = de.glossary_term_id
  AND gt.is_current_version = TRUE
  AND gt.deleted_at IS NULL
  AND gt.classification_id IS NOT NULL
  AND de.classification_id IS DISTINCT FROM gt.classification_id
  AND de.deleted_at IS NULL;
