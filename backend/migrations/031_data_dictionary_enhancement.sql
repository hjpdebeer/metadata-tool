-- Migration 031: Data Dictionary Enhancement
--
-- Enhances the Data Dictionary module to match the governance maturity of
-- the Business Glossary and Application Register modules.
--
-- Changes:
-- 1. Link source_systems to applications (application_id FK)
-- 2. Add full governance fields to data_elements (approver, org unit, review, versioning)
-- 3. Add is_pii tracking to data_elements and technical_tables
-- 4. Add column_relationships table for PK/FK/constraint documentation
-- 5. Update Data Element Review workflow to two-stage (Steward → Owner)

-- =========================================================================
-- 1. LINK SOURCE SYSTEMS TO APPLICATIONS
-- =========================================================================

ALTER TABLE source_systems
    ADD COLUMN IF NOT EXISTS application_id UUID REFERENCES applications(application_id),
    ADD COLUMN IF NOT EXISTS vendor VARCHAR(256),
    ADD COLUMN IF NOT EXISTS environment VARCHAR(50);

COMMENT ON COLUMN source_systems.application_id IS 'FK to the application that owns this database/source system';
COMMENT ON COLUMN source_systems.environment IS 'Environment: PRODUCTION, UAT, DEVELOPMENT, DR';

CREATE INDEX IF NOT EXISTS idx_source_systems_application ON source_systems(application_id) WHERE deleted_at IS NULL;

-- =========================================================================
-- 2. ADD GOVERNANCE FIELDS TO DATA_ELEMENTS
-- =========================================================================

ALTER TABLE data_elements
    ADD COLUMN IF NOT EXISTS approver_user_id UUID REFERENCES users(user_id),
    ADD COLUMN IF NOT EXISTS organisational_unit VARCHAR(256),
    ADD COLUMN IF NOT EXISTS review_frequency_id UUID REFERENCES glossary_review_frequencies(frequency_id),
    ADD COLUMN IF NOT EXISTS next_review_date DATE,
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS is_pii BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS version_number INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS is_current_version BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS previous_version_id UUID REFERENCES data_elements(element_id);

COMMENT ON COLUMN data_elements.approver_user_id IS 'User responsible for final approval in the workflow';
COMMENT ON COLUMN data_elements.is_pii IS 'Contains Personally Identifiable Information (GDPR/POPIA)';
COMMENT ON COLUMN data_elements.version_number IS 'Version number for amendment tracking';
COMMENT ON COLUMN data_elements.is_current_version IS 'Whether this is the current active version';
COMMENT ON COLUMN data_elements.previous_version_id IS 'FK to the version this amendment is based on';

-- Change element_code unique constraint to composite (code + version_number)
ALTER TABLE data_elements DROP CONSTRAINT IF EXISTS data_elements_element_code_key;
CREATE UNIQUE INDEX IF NOT EXISTS idx_data_elements_code_version
    ON data_elements (element_code, version_number)
    WHERE deleted_at IS NULL;

-- =========================================================================
-- 3. ADD PII TRACKING TO TECHNICAL_TABLES
-- =========================================================================

ALTER TABLE technical_tables
    ADD COLUMN IF NOT EXISTS is_pii BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS data_classification_id UUID REFERENCES data_classifications(classification_id),
    ADD COLUMN IF NOT EXISTS retention_policy TEXT;

COMMENT ON COLUMN technical_tables.is_pii IS 'Table contains PII data';
COMMENT ON COLUMN technical_tables.retention_policy IS 'Data retention policy description';

-- =========================================================================
-- 4. COLUMN RELATIONSHIPS TABLE
-- =========================================================================

CREATE TABLE IF NOT EXISTS column_relationships (
    relationship_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_column_id    UUID NOT NULL REFERENCES technical_columns(column_id) ON DELETE CASCADE,
    target_column_id    UUID NOT NULL REFERENCES technical_columns(column_id) ON DELETE CASCADE,
    relationship_type   VARCHAR(50) NOT NULL,
    constraint_name     VARCHAR(256),
    description         TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_no_self_reference CHECK (source_column_id != target_column_id),
    CONSTRAINT uq_column_relationship UNIQUE (source_column_id, target_column_id, relationship_type)
);

COMMENT ON TABLE column_relationships IS 'Documents PK/FK/UNIQUE/INDEX relationships between technical columns';

CREATE INDEX IF NOT EXISTS idx_column_rel_source ON column_relationships(source_column_id);
CREATE INDEX IF NOT EXISTS idx_column_rel_target ON column_relationships(target_column_id);

-- Seed relationship types as check constraint
ALTER TABLE column_relationships
    ADD CONSTRAINT chk_relationship_type
    CHECK (relationship_type IN ('PRIMARY_KEY', 'FOREIGN_KEY', 'UNIQUE', 'INDEX', 'CHECK'));

-- =========================================================================
-- 5. UPDATE DATA ELEMENT REVIEW WORKFLOW (TWO-STAGE)
-- =========================================================================

DO $$
DECLARE
    v_de_def_id UUID;
    v_draft_id UUID;
    v_proposed_id UUID;
    v_under_review_id UUID;
    v_pending_approval_id UUID;
    v_accepted_id UUID;
    v_rejected_id UUID;
    v_revised_id UUID;
    v_deprecated_id UUID;
BEGIN
    SELECT workflow_def_id INTO v_de_def_id
    FROM workflow_definitions WHERE workflow_name = 'Data Element Review';

    SELECT state_id INTO v_draft_id FROM workflow_states WHERE state_code = 'DRAFT';
    SELECT state_id INTO v_proposed_id FROM workflow_states WHERE state_code = 'PROPOSED';
    SELECT state_id INTO v_under_review_id FROM workflow_states WHERE state_code = 'UNDER_REVIEW';
    SELECT state_id INTO v_pending_approval_id FROM workflow_states WHERE state_code = 'PENDING_APPROVAL';
    SELECT state_id INTO v_accepted_id FROM workflow_states WHERE state_code = 'ACCEPTED';
    SELECT state_id INTO v_rejected_id FROM workflow_states WHERE state_code = 'REJECTED';
    SELECT state_id INTO v_revised_id FROM workflow_states WHERE state_code = 'REVISED';
    SELECT state_id INTO v_deprecated_id FROM workflow_states WHERE state_code = 'DEPRECATED';

    -- Remove old transitions
    DELETE FROM workflow_transitions WHERE workflow_def_id = v_de_def_id;

    -- Insert new two-stage transitions (matching glossary/application pattern)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description) VALUES
        (v_de_def_id, v_draft_id, v_under_review_id, 'SUBMIT', 'Submit for Review',
         'Submit data element for Data Steward review'),
        (v_de_def_id, v_under_review_id, v_pending_approval_id, 'APPROVE', 'Approve (Steward)',
         'Data Steward approves — forwarded to Owner for final approval'),
        (v_de_def_id, v_under_review_id, v_rejected_id, 'REJECT', 'Reject',
         'Data Steward rejects the data element'),
        (v_de_def_id, v_under_review_id, v_revised_id, 'REVISE', 'Request Revision',
         'Data Steward requests revision'),
        (v_de_def_id, v_pending_approval_id, v_accepted_id, 'APPROVE', 'Final Approval (Owner)',
         'Data Owner gives final approval'),
        (v_de_def_id, v_pending_approval_id, v_rejected_id, 'REJECT', 'Reject',
         'Data Owner rejects the data element'),
        (v_de_def_id, v_pending_approval_id, v_under_review_id, 'REVISE', 'Return to Steward',
         'Data Owner returns to Data Steward for re-review'),
        (v_de_def_id, v_revised_id, v_under_review_id, 'SUBMIT', 'Resubmit',
         'Resubmit revised data element for Data Steward review'),
        (v_de_def_id, v_accepted_id, v_deprecated_id, 'DEPRECATE', 'Deprecate',
         'Deprecate an accepted data element');

    -- Migrate any existing PROPOSED data element instances → UNDER_REVIEW
    UPDATE workflow_instances
    SET current_state_id = v_under_review_id, updated_at = CURRENT_TIMESTAMP
    WHERE workflow_def_id = v_de_def_id
      AND current_state_id = v_proposed_id;

END $$;

-- Migrate any PROPOSED data elements → UNDER_REVIEW entity status
UPDATE data_elements
SET status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'UNDER_REVIEW'),
    updated_at = CURRENT_TIMESTAMP
WHERE status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'PROPOSED')
  AND deleted_at IS NULL;
