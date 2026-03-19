-- Migration 022: Owner revision goes back to Steward, not Creator
--
-- When the Owner requests revision from Pending Approval, the term should
-- go back to Under Review (Data Steward) rather than Revised (Creator).
-- Steward already reviewed once — Owner feedback needs Steward to verify.
--
-- Also backfills approved_at for terms already in Accepted status.

-- 1. Update the REVISE transition from Pending Approval → Under Review (was → Revised)
UPDATE workflow_transitions
SET to_state_id = (SELECT state_id FROM workflow_states WHERE state_code = 'UNDER_REVIEW'),
    action_name = 'Return to Steward',
    description = 'Business Term Owner returns to Data Steward for re-review'
WHERE workflow_def_id = (SELECT workflow_def_id FROM workflow_definitions WHERE workflow_name = 'Glossary Term Review')
  AND from_state_id = (SELECT state_id FROM workflow_states WHERE state_code = 'PENDING_APPROVAL')
  AND action_code = 'REVISE';

-- 2. Backfill approved_at for terms already in Accepted status
UPDATE glossary_terms
SET approved_at = updated_at
WHERE status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'ACCEPTED')
  AND approved_at IS NULL
  AND deleted_at IS NULL;
