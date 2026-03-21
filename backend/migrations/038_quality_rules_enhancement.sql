-- Migration 038: Quality Rules Enhancement + Score Ingestion Support
--
-- Enhances quality_rules with structured rule metadata.
-- Enhances quality_scores for external profiling tool ingestion.
-- Auto-generates rule_code (QR-{DIMENSION}-{SEQ}).

-- =========================================================================
-- 1. ENHANCE QUALITY_RULES
-- =========================================================================

ALTER TABLE quality_rules
    ADD COLUMN IF NOT EXISTS rule_expression TEXT,
    ADD COLUMN IF NOT EXISTS comparison_type VARCHAR(30),
    ADD COLUMN IF NOT EXISTS comparison_value TEXT,
    ADD COLUMN IF NOT EXISTS scope VARCHAR(30) NOT NULL DEFAULT 'RECORD',
    ADD COLUMN IF NOT EXISTS check_frequency VARCHAR(30),
    ADD COLUMN IF NOT EXISTS steward_user_id UUID REFERENCES users(user_id),
    ADD COLUMN IF NOT EXISTS approver_user_id UUID REFERENCES users(user_id),
    ADD COLUMN IF NOT EXISTS organisational_unit VARCHAR(256),
    ADD COLUMN IF NOT EXISTS review_frequency_id UUID REFERENCES glossary_review_frequencies(frequency_id),
    ADD COLUMN IF NOT EXISTS next_review_date DATE,
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS version_number INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS is_current_version BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS previous_version_id UUID REFERENCES quality_rules(rule_id);

-- Comparison type check
ALTER TABLE quality_rules DROP CONSTRAINT IF EXISTS chk_comparison_type;
ALTER TABLE quality_rules ADD CONSTRAINT chk_comparison_type
    CHECK (comparison_type IS NULL OR comparison_type IN (
        'GREATER_THAN', 'LESS_THAN', 'EQUAL', 'NOT_EQUAL',
        'BETWEEN', 'NOT_NULL', 'UNIQUE', 'REGEX', 'IN_LIST',
        'CUSTOM_SQL'
    ));

-- Scope check
ALTER TABLE quality_rules DROP CONSTRAINT IF EXISTS chk_scope;
ALTER TABLE quality_rules ADD CONSTRAINT chk_scope
    CHECK (scope IN ('RECORD', 'DATASET', 'CROSS_SYSTEM'));

COMMENT ON COLUMN quality_rules.rule_expression IS 'The actual check expression (SQL, regex, or logical expression)';
COMMENT ON COLUMN quality_rules.comparison_type IS 'Type of comparison: GREATER_THAN, LESS_THAN, NOT_NULL, UNIQUE, REGEX, etc.';
COMMENT ON COLUMN quality_rules.comparison_value IS 'Value to compare against (for GREATER_THAN, REGEX, etc.)';
COMMENT ON COLUMN quality_rules.scope IS 'RECORD (per-row check), DATASET (aggregate), CROSS_SYSTEM (reconciliation)';
COMMENT ON COLUMN quality_rules.check_frequency IS 'How often: REALTIME, HOURLY, DAILY, WEEKLY, MONTHLY, ON_DEMAND';

-- Auto-generate rule_code
CREATE SEQUENCE IF NOT EXISTS quality_rule_code_seq START WITH 1;

CREATE OR REPLACE FUNCTION generate_quality_rule_code()
RETURNS TRIGGER AS $$
DECLARE
    v_dim_code VARCHAR(10);
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

ALTER TABLE quality_rules ALTER COLUMN rule_code DROP NOT NULL;

DROP TRIGGER IF EXISTS trg_generate_rule_code ON quality_rules;
CREATE TRIGGER trg_generate_rule_code
    BEFORE INSERT ON quality_rules
    FOR EACH ROW
    WHEN (NEW.rule_code IS NULL)
    EXECUTE FUNCTION generate_quality_rule_code();

-- Composite unique on (rule_code, version_number) for amendments
ALTER TABLE quality_rules DROP CONSTRAINT IF EXISTS quality_rules_rule_code_key;
CREATE UNIQUE INDEX IF NOT EXISTS idx_quality_rules_code_version
    ON quality_rules (rule_code, version_number)
    WHERE deleted_at IS NULL;

-- =========================================================================
-- 2. ENHANCE QUALITY_SCORES FOR INGESTION
-- =========================================================================

ALTER TABLE quality_scores
    ADD COLUMN IF NOT EXISTS rule_id UUID REFERENCES quality_rules(rule_id),
    ADD COLUMN IF NOT EXISTS profiling_run_id VARCHAR(128),
    ADD COLUMN IF NOT EXISTS source_system_code VARCHAR(64),
    ADD COLUMN IF NOT EXISTS records_evaluated BIGINT,
    ADD COLUMN IF NOT EXISTS records_passed BIGINT,
    ADD COLUMN IF NOT EXISTS records_failed BIGINT,
    ADD COLUMN IF NOT EXISTS pass_rate NUMERIC(7,4),
    ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'PASS',
    ADD COLUMN IF NOT EXISTS details TEXT,
    ADD COLUMN IF NOT EXISTS tool_name VARCHAR(128),
    ADD COLUMN IF NOT EXISTS profiled_at TIMESTAMPTZ;

ALTER TABLE quality_scores DROP CONSTRAINT IF EXISTS chk_score_status;
ALTER TABLE quality_scores ADD CONSTRAINT chk_score_status
    CHECK (status IN ('PASS', 'FAIL', 'WARNING', 'ERROR', 'SKIPPED'));

CREATE INDEX IF NOT EXISTS idx_quality_scores_rule ON quality_scores(rule_id) WHERE rule_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_quality_scores_run ON quality_scores(profiling_run_id) WHERE profiling_run_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_quality_scores_profiled ON quality_scores(profiled_at DESC);

COMMENT ON TABLE quality_scores IS 'Quality profiling results from external tools — ingested via API';

-- =========================================================================
-- 3. UPDATE QUALITY RULE REVIEW WORKFLOW (TWO-STAGE)
-- =========================================================================

DO $$
DECLARE
    v_qr_def_id UUID;
    v_draft_id UUID;
    v_proposed_id UUID;
    v_under_review_id UUID;
    v_pending_approval_id UUID;
    v_accepted_id UUID;
    v_rejected_id UUID;
    v_revised_id UUID;
    v_deprecated_id UUID;
BEGIN
    SELECT workflow_def_id INTO v_qr_def_id
    FROM workflow_definitions WHERE workflow_name = 'Quality Rule Review';

    SELECT state_id INTO v_draft_id FROM workflow_states WHERE state_code = 'DRAFT';
    SELECT state_id INTO v_proposed_id FROM workflow_states WHERE state_code = 'PROPOSED';
    SELECT state_id INTO v_under_review_id FROM workflow_states WHERE state_code = 'UNDER_REVIEW';
    SELECT state_id INTO v_pending_approval_id FROM workflow_states WHERE state_code = 'PENDING_APPROVAL';
    SELECT state_id INTO v_accepted_id FROM workflow_states WHERE state_code = 'ACCEPTED';
    SELECT state_id INTO v_rejected_id FROM workflow_states WHERE state_code = 'REJECTED';
    SELECT state_id INTO v_revised_id FROM workflow_states WHERE state_code = 'REVISED';
    SELECT state_id INTO v_deprecated_id FROM workflow_states WHERE state_code = 'DEPRECATED';

    DELETE FROM workflow_transitions WHERE workflow_def_id = v_qr_def_id;

    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description) VALUES
        (v_qr_def_id, v_draft_id, v_under_review_id, 'SUBMIT', 'Submit for Review',
         'Submit quality rule for Data Steward review'),
        (v_qr_def_id, v_under_review_id, v_pending_approval_id, 'APPROVE', 'Approve (Steward)',
         'Data Steward approves — forwarded to Owner for final approval'),
        (v_qr_def_id, v_under_review_id, v_rejected_id, 'REJECT', 'Reject',
         'Data Steward rejects the quality rule'),
        (v_qr_def_id, v_under_review_id, v_revised_id, 'REVISE', 'Request Revision',
         'Data Steward requests revision'),
        (v_qr_def_id, v_pending_approval_id, v_accepted_id, 'APPROVE', 'Final Approval (Owner)',
         'Data Owner gives final approval'),
        (v_qr_def_id, v_pending_approval_id, v_rejected_id, 'REJECT', 'Reject',
         'Data Owner rejects the quality rule'),
        (v_qr_def_id, v_pending_approval_id, v_under_review_id, 'REVISE', 'Return to Steward',
         'Data Owner returns to Data Steward for re-review'),
        (v_qr_def_id, v_revised_id, v_under_review_id, 'SUBMIT', 'Resubmit',
         'Resubmit revised quality rule for Data Steward review'),
        (v_qr_def_id, v_accepted_id, v_deprecated_id, 'DEPRECATE', 'Deprecate',
         'Deprecate an accepted quality rule');

    UPDATE workflow_instances
    SET current_state_id = v_under_review_id, updated_at = CURRENT_TIMESTAMP
    WHERE workflow_def_id = v_qr_def_id
      AND current_state_id = v_proposed_id;

END $$;

UPDATE quality_rules
SET status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'UNDER_REVIEW'),
    updated_at = CURRENT_TIMESTAMP
WHERE status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'PROPOSED')
  AND deleted_at IS NULL;
