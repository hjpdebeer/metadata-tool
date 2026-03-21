-- Migration 040: Simplify quality rules — remove standalone governance
--
-- Quality rules are child records of data elements. They inherit the
-- element's workflow status and don't need their own governance fields.
-- Same pattern as glossary term aliases, regulatory tags, subject areas.

-- Remove governance fields that only existed for standalone workflow
ALTER TABLE quality_rules
    DROP COLUMN IF EXISTS status_id,
    DROP COLUMN IF EXISTS steward_user_id,
    DROP COLUMN IF EXISTS approver_user_id,
    DROP COLUMN IF EXISTS organisational_unit,
    DROP COLUMN IF EXISTS review_frequency_id,
    DROP COLUMN IF EXISTS next_review_date,
    DROP COLUMN IF EXISTS approved_at,
    DROP COLUMN IF EXISTS version_number,
    DROP COLUMN IF EXISTS is_current_version,
    DROP COLUMN IF EXISTS previous_version_id;

-- Drop the status index (no longer has status_id)
DROP INDEX IF EXISTS idx_quality_rules_status;
