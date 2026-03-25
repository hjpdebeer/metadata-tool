-- Remove the redundant approver_user_id column from all entity tables.
-- In the two-stage workflow (Steward reviews → Owner approves), the
-- owner_user_id IS the approver. The separate approver_user_id field
-- was redundant and caused confusion.
--
-- The workflow validation (validate_ownership_before_submit) is updated
-- in application code to no longer require approver_user_id.

-- Drop approver columns from entity tables
ALTER TABLE glossary_terms DROP COLUMN IF EXISTS approver_user_id;
ALTER TABLE data_elements DROP COLUMN IF EXISTS approver_user_id;
ALTER TABLE applications DROP COLUMN IF EXISTS approver_user_id;

-- Note: quality_rules.approver_user_id was already removed in migration 040.
-- Note: workflow_instances.approver_user_id is kept — it tracks which user
-- was the actual approver for a specific workflow instance (audit trail).
