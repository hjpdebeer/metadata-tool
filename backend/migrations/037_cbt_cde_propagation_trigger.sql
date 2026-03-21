-- Migration 037: CBT → CDE and Classification Propagation via DB Triggers
--
-- ADR-0005: CDE Propagation — auto-acceptance via inheritance.
--
-- When a glossary term is a CBT (Critical Business Term), all linked data
-- elements automatically become CDEs (Critical Data Elements).
-- When a glossary term has a classification, linked data elements inherit it.
--
-- This bypasses the review/approval workflow intentionally: the Owner approved
-- the data element and its linkage to the glossary term, so the inherited
-- properties are aligned with that governance decision.
--
-- Two triggers:
-- 1. On data_elements: when glossary_term_id is set/changed, inherit from term
-- 2. On glossary_terms: when is_cbt or classification_id changes, propagate to elements

-- =========================================================================
-- 1. TRIGGER: Data element links to a glossary term → inherit CBT/classification
-- =========================================================================

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

            -- Inherit classification (only if element doesn't already have one)
            IF v_classification_id IS NOT NULL AND NEW.classification_id IS NULL THEN
                NEW.classification_id := v_classification_id;
            END IF;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_propagate_term_to_element ON data_elements;
CREATE TRIGGER trg_propagate_term_to_element
    BEFORE INSERT OR UPDATE OF glossary_term_id ON data_elements
    FOR EACH ROW
    EXECUTE FUNCTION propagate_term_to_element();

COMMENT ON FUNCTION propagate_term_to_element() IS 'ADR-0005: Inherits CDE and classification from linked CBT glossary term. Auto-acceptance via inheritance — bypasses workflow intentionally.';

-- =========================================================================
-- 2. TRIGGER: Glossary term CBT/classification changes → propagate to elements
-- =========================================================================

CREATE OR REPLACE FUNCTION propagate_cbt_to_elements()
RETURNS TRIGGER AS $$
BEGIN
    -- If term becomes a CBT, mark all linked elements as CDE
    IF NEW.is_cbt = TRUE AND (OLD.is_cbt = FALSE OR OLD.is_cbt IS NULL) THEN
        UPDATE data_elements
        SET is_cde = TRUE,
            cde_rationale = COALESCE(
                cde_rationale,
                'Auto-designated: inherited from Critical Business Term (ADR-0005)'
            ),
            cde_designated_at = COALESCE(cde_designated_at, CURRENT_TIMESTAMP),
            updated_at = CURRENT_TIMESTAMP
        WHERE glossary_term_id = NEW.term_id
          AND deleted_at IS NULL
          AND is_cde = FALSE;
    END IF;

    -- If term loses CBT status, remove auto-designated CDE from linked elements
    -- (only removes if the rationale indicates it was auto-designated)
    IF NEW.is_cbt = FALSE AND OLD.is_cbt = TRUE THEN
        UPDATE data_elements
        SET is_cde = FALSE,
            cde_rationale = NULL,
            cde_designated_at = NULL,
            updated_at = CURRENT_TIMESTAMP
        WHERE glossary_term_id = NEW.term_id
          AND deleted_at IS NULL
          AND cde_rationale LIKE '%Auto-designated%';
    END IF;

    -- If classification changes, propagate to elements that inherited it
    IF NEW.classification_id IS DISTINCT FROM OLD.classification_id THEN
        -- Update elements that had the old classification (inherited)
        IF OLD.classification_id IS NOT NULL THEN
            UPDATE data_elements
            SET classification_id = NEW.classification_id,
                updated_at = CURRENT_TIMESTAMP
            WHERE glossary_term_id = NEW.term_id
              AND deleted_at IS NULL
              AND classification_id = OLD.classification_id;
        END IF;

        -- Also set classification on elements that don't have one yet
        IF NEW.classification_id IS NOT NULL THEN
            UPDATE data_elements
            SET classification_id = NEW.classification_id,
                updated_at = CURRENT_TIMESTAMP
            WHERE glossary_term_id = NEW.term_id
              AND deleted_at IS NULL
              AND classification_id IS NULL;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_propagate_cbt_to_elements ON glossary_terms;
CREATE TRIGGER trg_propagate_cbt_to_elements
    AFTER UPDATE OF is_cbt, classification_id ON glossary_terms
    FOR EACH ROW
    EXECUTE FUNCTION propagate_cbt_to_elements();

COMMENT ON FUNCTION propagate_cbt_to_elements() IS 'ADR-0005: Propagates CBT→CDE and classification changes to all linked data elements. Auto-acceptance via inheritance.';
