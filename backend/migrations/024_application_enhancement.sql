-- Migration 024: Application Register Enhancement
--
-- Enhances the applications table from 23 fields to ~45 fields with full
-- governance metadata, matching the pattern established in the Business Glossary.
-- Adds configurable lookup tables for DR tiers, lifecycle stages, criticality
-- tiers, risk ratings, and SLA tiers.
-- Adds golden_source_app_id FK on glossary_terms to link golden source to
-- a registered application.
-- Updates Application Review workflow to two-stage pattern (Steward → Owner).

-- =========================================================================
-- 1. NEW LOOKUP TABLES
-- =========================================================================

-- Disaster Recovery Tiers (configurable via Admin Panel)
CREATE TABLE IF NOT EXISTS disaster_recovery_tiers (
    dr_tier_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tier_code       VARCHAR(50)  NOT NULL UNIQUE,
    tier_name       VARCHAR(128) NOT NULL,
    rto_hours       INTEGER      NOT NULL,
    rpo_minutes     INTEGER      NOT NULL,
    description     TEXT,
    display_order   INTEGER      NOT NULL DEFAULT 0
);

INSERT INTO disaster_recovery_tiers (tier_code, tier_name, rto_hours, rpo_minutes, description, display_order) VALUES
    ('PLATINUM', 'Platinum', 4,    15,   'RTO: 4 hours, RPO: 15 minutes — mission-critical systems', 10),
    ('GOLD',     'Gold',     8,    30,   'RTO: 8 hours, RPO: 30 minutes — business-critical systems', 20),
    ('SILVER',   'Silver',   12,   60,   'RTO: 12 hours, RPO: 1 hour — important business systems',   30),
    ('BRONZE',   'Bronze',   24,   360,  'RTO: 24 hours, RPO: 6 hours — standard business systems',   40),
    ('IRON',     'Iron',     168,  1440, 'RTO: 7 days, RPO: 24 hours — non-critical systems',         50);

COMMENT ON TABLE disaster_recovery_tiers IS 'Configurable disaster recovery tier lookup with RTO/RPO definitions';

-- Application Lifecycle Stages
CREATE TABLE IF NOT EXISTS application_lifecycle_stages (
    stage_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stage_code      VARCHAR(50)  NOT NULL UNIQUE,
    stage_name      VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INTEGER      NOT NULL DEFAULT 0
);

INSERT INTO application_lifecycle_stages (stage_code, stage_name, description, display_order) VALUES
    ('PLANNING',    'Planning',    'Application is in planning/evaluation phase',                10),
    ('DEVELOPMENT', 'Development', 'Application is being developed or configured',              20),
    ('TESTING',     'Testing',     'Application is in testing/UAT phase',                       30),
    ('ACTIVE',      'Active',      'Application is live and in active use',                     40),
    ('SUNSET',      'Sunset',      'Application is scheduled for retirement, limited changes',  50),
    ('RETIRED',     'Retired',     'Application has been decommissioned',                       60);

COMMENT ON TABLE application_lifecycle_stages IS 'Application lifecycle stage lookup';

-- Application Criticality Tiers
CREATE TABLE IF NOT EXISTS application_criticality_tiers (
    tier_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tier_code       VARCHAR(50)  NOT NULL UNIQUE,
    tier_name       VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INTEGER      NOT NULL DEFAULT 0
);

INSERT INTO application_criticality_tiers (tier_code, tier_name, description, display_order) VALUES
    ('TIER_1', 'Tier 1 — Mission Critical',  'Failure causes immediate, severe business impact; regulatory exposure',  10),
    ('TIER_2', 'Tier 2 — Business Critical',  'Failure causes significant operational disruption within hours',       20),
    ('TIER_3', 'Tier 3 — Business Support',   'Failure causes inconvenience; workarounds available',                  30),
    ('TIER_4', 'Tier 4 — Administrative',      'Failure has minimal business impact; manual fallback exists',          40);

COMMENT ON TABLE application_criticality_tiers IS 'Business criticality tier lookup for applications';

-- Application Risk Ratings
CREATE TABLE IF NOT EXISTS application_risk_ratings (
    rating_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rating_code     VARCHAR(50)  NOT NULL UNIQUE,
    rating_name     VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INTEGER      NOT NULL DEFAULT 0
);

INSERT INTO application_risk_ratings (rating_code, rating_name, description, display_order) VALUES
    ('CRITICAL', 'Critical', 'Requires immediate attention and remediation',           10),
    ('HIGH',     'High',     'Significant risk requiring prioritised treatment',       20),
    ('MEDIUM',   'Medium',   'Moderate risk with planned remediation',                 30),
    ('LOW',      'Low',      'Acceptable risk level with standard controls in place',  40);

COMMENT ON TABLE application_risk_ratings IS 'Application risk rating lookup';

-- SLA Tiers
CREATE TABLE IF NOT EXISTS sla_tiers (
    sla_tier_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tier_code       VARCHAR(50)  NOT NULL UNIQUE,
    tier_name       VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INTEGER      NOT NULL DEFAULT 0
);

INSERT INTO sla_tiers (tier_code, tier_name, description, display_order) VALUES
    ('PLATINUM', 'Platinum', '99.99% availability, 15-minute response time',  10),
    ('GOLD',     'Gold',     '99.9% availability, 1-hour response time',      20),
    ('SILVER',   'Silver',   '99.5% availability, 4-hour response time',      30),
    ('BRONZE',   'Bronze',   '99% availability, 8-hour response time',        40);

COMMENT ON TABLE sla_tiers IS 'Service Level Agreement tier lookup';

-- =========================================================================
-- 2. ENHANCE APPLICATIONS TABLE
-- =========================================================================

-- Rename is_critical → is_cba (Critical Business Application)
ALTER TABLE applications RENAME COLUMN is_critical TO is_cba;
ALTER TABLE applications RENAME COLUMN criticality_rationale TO cba_rationale;

-- Drop old index and recreate with new column name
DROP INDEX IF EXISTS idx_applications_critical;
CREATE INDEX idx_applications_cba ON applications(is_cba) WHERE is_cba = TRUE AND deleted_at IS NULL;

-- New columns
ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS external_reference_id  VARCHAR(256),
    ADD COLUMN IF NOT EXISTS vendor_product_name    VARCHAR(256),
    ADD COLUMN IF NOT EXISTS abbreviation           VARCHAR(50),
    ADD COLUMN IF NOT EXISTS business_capability    TEXT,
    ADD COLUMN IF NOT EXISTS user_base              TEXT,
    ADD COLUMN IF NOT EXISTS data_classification_id UUID REFERENCES data_classifications(classification_id),
    ADD COLUMN IF NOT EXISTS regulatory_scope       TEXT,
    ADD COLUMN IF NOT EXISTS last_security_assessment DATE,
    ADD COLUMN IF NOT EXISTS risk_rating_id         UUID REFERENCES application_risk_ratings(rating_id),
    ADD COLUMN IF NOT EXISTS support_model          VARCHAR(50),
    ADD COLUMN IF NOT EXISTS sla_tier_id            UUID REFERENCES sla_tiers(sla_tier_id),
    ADD COLUMN IF NOT EXISTS dr_tier_id             UUID REFERENCES disaster_recovery_tiers(dr_tier_id),
    ADD COLUMN IF NOT EXISTS lifecycle_stage_id     UUID REFERENCES application_lifecycle_stages(stage_id),
    ADD COLUMN IF NOT EXISTS criticality_tier_id    UUID REFERENCES application_criticality_tiers(tier_id),
    ADD COLUMN IF NOT EXISTS contract_end_date      DATE,
    ADD COLUMN IF NOT EXISTS license_type           VARCHAR(100),
    ADD COLUMN IF NOT EXISTS review_frequency_id    UUID REFERENCES glossary_review_frequencies(frequency_id),
    ADD COLUMN IF NOT EXISTS next_review_date       DATE,
    ADD COLUMN IF NOT EXISTS approved_at            TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS steward_user_id        UUID REFERENCES users(user_id),
    ADD COLUMN IF NOT EXISTS approver_user_id       UUID REFERENCES users(user_id),
    ADD COLUMN IF NOT EXISTS organisational_unit    VARCHAR(256);

COMMENT ON COLUMN applications.is_cba IS 'Critical Business Application flag — propagates CDE to linked data elements';
COMMENT ON COLUMN applications.cba_rationale IS 'Rationale for CBA designation';
COMMENT ON COLUMN applications.external_reference_id IS 'External reference (CMDB ID, ServiceNow ID, asset tag)';
COMMENT ON COLUMN applications.vendor_product_name IS 'Vendor product name (e.g., Backbase Digital Banking Suite)';
COMMENT ON COLUMN applications.abbreviation IS 'Short name or acronym';
COMMENT ON COLUMN applications.business_capability IS 'Business function/capability this application supports';
COMMENT ON COLUMN applications.user_base IS 'Description of user base and estimated user count';
COMMENT ON COLUMN applications.steward_user_id IS 'Data Steward responsible for governance review';
COMMENT ON COLUMN applications.approver_user_id IS 'Approver for workflow (typically Business Owner)';

-- =========================================================================
-- 3. GOLDEN SOURCE APP LINK ON GLOSSARY TERMS
-- =========================================================================

ALTER TABLE glossary_terms
    ADD COLUMN IF NOT EXISTS golden_source_app_id UUID REFERENCES applications(application_id);

COMMENT ON COLUMN glossary_terms.golden_source_app_id IS 'FK to the application that is the golden/authoritative source for this term';

-- =========================================================================
-- 4. UPDATE APPLICATION REVIEW WORKFLOW (TWO-STAGE)
-- =========================================================================

DO $$
DECLARE
    v_app_def_id UUID;
    v_draft_id UUID;
    v_proposed_id UUID;
    v_under_review_id UUID;
    v_pending_approval_id UUID;
    v_accepted_id UUID;
    v_rejected_id UUID;
    v_revised_id UUID;
    v_deprecated_id UUID;
BEGIN
    SELECT workflow_def_id INTO v_app_def_id
    FROM workflow_definitions WHERE workflow_name = 'Application Review';

    SELECT state_id INTO v_draft_id FROM workflow_states WHERE state_code = 'DRAFT';
    SELECT state_id INTO v_proposed_id FROM workflow_states WHERE state_code = 'PROPOSED';
    SELECT state_id INTO v_under_review_id FROM workflow_states WHERE state_code = 'UNDER_REVIEW';
    SELECT state_id INTO v_pending_approval_id FROM workflow_states WHERE state_code = 'PENDING_APPROVAL';
    SELECT state_id INTO v_accepted_id FROM workflow_states WHERE state_code = 'ACCEPTED';
    SELECT state_id INTO v_rejected_id FROM workflow_states WHERE state_code = 'REJECTED';
    SELECT state_id INTO v_revised_id FROM workflow_states WHERE state_code = 'REVISED';
    SELECT state_id INTO v_deprecated_id FROM workflow_states WHERE state_code = 'DEPRECATED';

    -- Remove old transitions
    DELETE FROM workflow_transitions WHERE workflow_def_id = v_app_def_id;

    -- Insert new two-stage transitions (matching glossary pattern)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description) VALUES
        (v_app_def_id, v_draft_id, v_under_review_id, 'SUBMIT', 'Submit for Review',
         'Submit application for Data Steward review'),
        (v_app_def_id, v_under_review_id, v_pending_approval_id, 'APPROVE', 'Approve (Steward)',
         'Data Steward approves — forwarded to Business Owner for final approval'),
        (v_app_def_id, v_under_review_id, v_rejected_id, 'REJECT', 'Reject',
         'Data Steward rejects the application'),
        (v_app_def_id, v_under_review_id, v_revised_id, 'REVISE', 'Request Revision',
         'Data Steward requests revision'),
        (v_app_def_id, v_pending_approval_id, v_accepted_id, 'APPROVE', 'Final Approval (Owner)',
         'Business Owner gives final approval'),
        (v_app_def_id, v_pending_approval_id, v_rejected_id, 'REJECT', 'Reject',
         'Business Owner rejects the application'),
        (v_app_def_id, v_pending_approval_id, v_under_review_id, 'REVISE', 'Return to Steward',
         'Business Owner returns to Data Steward for re-review'),
        (v_app_def_id, v_revised_id, v_under_review_id, 'SUBMIT', 'Resubmit',
         'Resubmit revised application for Data Steward review'),
        (v_app_def_id, v_accepted_id, v_deprecated_id, 'DEPRECATE', 'Deprecate',
         'Deprecate an accepted application');

    -- Migrate any existing PROPOSED application instances → UNDER_REVIEW
    UPDATE workflow_instances
    SET current_state_id = v_under_review_id, updated_at = CURRENT_TIMESTAMP
    WHERE workflow_def_id = v_app_def_id
      AND current_state_id = v_proposed_id;

END $$;

-- Migrate any PROPOSED applications → UNDER_REVIEW entity status
UPDATE applications
SET status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'UNDER_REVIEW'),
    updated_at = CURRENT_TIMESTAMP
WHERE status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'PROPOSED')
  AND deleted_at IS NULL;
