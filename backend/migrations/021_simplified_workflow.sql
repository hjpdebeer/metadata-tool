-- Migration 021: Simplified two-stage workflow
--
-- Before: Draft → SUBMIT → Proposed → ASSIGN_REVIEW → Under Review → APPROVE → Accepted
-- After:  Draft → SUBMIT → Under Review (Steward) → APPROVE → Pending Approval (Owner) → APPROVE → Accepted
--
-- Removes the Proposed intermediate state.
-- Adds Pending Approval state for Owner final sign-off.

-- 1. Add PENDING_APPROVAL workflow state
INSERT INTO workflow_states (state_id, state_code, state_name, description, is_terminal, display_order)
VALUES (
    'a1b2c3d4-e5f6-7890-abcd-ef1234567890',
    'PENDING_APPROVAL',
    'Pending Approval',
    'Reviewed by Data Steward, awaiting final approval from Business Term Owner',
    FALSE,
    4  -- between Under Review (3) and Revised (5)
) ON CONFLICT (state_code) DO NOTHING;

-- 2. Add PENDING_APPROVAL entity status
INSERT INTO entity_statuses (status_id, status_code, status_name, description, display_order)
VALUES (
    'b2c3d4e5-f6a7-8901-bcde-f12345678901',
    'PENDING_APPROVAL',
    'Pending Approval',
    'Reviewed by Data Steward, awaiting final approval from Business Term Owner',
    4
) ON CONFLICT (status_code) DO NOTHING;

-- 3. Get IDs we need for transition updates
DO $$
DECLARE
    v_glossary_def_id UUID;
    v_draft_id UUID;
    v_proposed_id UUID;
    v_under_review_id UUID;
    v_pending_approval_id UUID;
    v_accepted_id UUID;
    v_rejected_id UUID;
    v_revised_id UUID;
BEGIN
    SELECT workflow_def_id INTO v_glossary_def_id
    FROM workflow_definitions WHERE workflow_name = 'Glossary Term Review';

    SELECT state_id INTO v_draft_id FROM workflow_states WHERE state_code = 'DRAFT';
    SELECT state_id INTO v_proposed_id FROM workflow_states WHERE state_code = 'PROPOSED';
    SELECT state_id INTO v_under_review_id FROM workflow_states WHERE state_code = 'UNDER_REVIEW';
    SELECT state_id INTO v_pending_approval_id FROM workflow_states WHERE state_code = 'PENDING_APPROVAL';
    SELECT state_id INTO v_accepted_id FROM workflow_states WHERE state_code = 'ACCEPTED';
    SELECT state_id INTO v_rejected_id FROM workflow_states WHERE state_code = 'REJECTED';
    SELECT state_id INTO v_revised_id FROM workflow_states WHERE state_code = 'REVISED';

    -- 4. Remove old transitions for Glossary Term Review
    DELETE FROM workflow_transitions WHERE workflow_def_id = v_glossary_def_id;

    -- 5. Insert new simplified transitions
    --    Draft → SUBMIT → Under Review (Data Steward reviews)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_draft_id, v_under_review_id, 'SUBMIT', 'Submit for Review',
            'Submit term for Data Steward review');

    --    Under Review → APPROVE → Pending Approval (Steward approves, goes to Owner)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_under_review_id, v_pending_approval_id, 'APPROVE', 'Approve (Steward)',
            'Data Steward approves — forwarded to Business Term Owner for final approval');

    --    Under Review → REJECT → Rejected
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_under_review_id, v_rejected_id, 'REJECT', 'Reject',
            'Data Steward rejects the term');

    --    Under Review → REVISE → Revised (back for rework)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_under_review_id, v_revised_id, 'REVISE', 'Request Revision',
            'Data Steward requests revision');

    --    Pending Approval → APPROVE → Accepted (Owner final approval)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_pending_approval_id, v_accepted_id, 'APPROVE', 'Final Approval (Owner)',
            'Business Term Owner gives final approval');

    --    Pending Approval → REJECT → Rejected
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_pending_approval_id, v_rejected_id, 'REJECT', 'Reject',
            'Business Term Owner rejects the term');

    --    Pending Approval → REVISE → Revised (back for rework)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_pending_approval_id, v_revised_id, 'REVISE', 'Request Revision',
            'Business Term Owner requests revision');

    --    Revised → SUBMIT → Under Review (resubmit goes back to steward)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_revised_id, v_under_review_id, 'SUBMIT', 'Resubmit',
            'Resubmit revised term for Data Steward review');

    --    Accepted → DEPRECATE → Deprecated (retire accepted terms)
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, description)
    VALUES (v_glossary_def_id, v_accepted_id, (SELECT state_id FROM workflow_states WHERE state_code = 'DEPRECATED'), 'DEPRECATE', 'Deprecate',
            'Deprecate an accepted term');

    -- 6. Migrate any existing PROPOSED workflow instances → UNDER_REVIEW
    UPDATE workflow_instances
    SET current_state_id = v_under_review_id, updated_at = CURRENT_TIMESTAMP
    WHERE workflow_def_id = v_glossary_def_id
      AND current_state_id = v_proposed_id;

    -- 7. Migrate any PROPOSED glossary terms → UNDER_REVIEW entity status
    UPDATE glossary_terms
    SET status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'UNDER_REVIEW'),
        updated_at = CURRENT_TIMESTAMP
    WHERE status_id = (SELECT status_id FROM entity_statuses WHERE status_code = 'PROPOSED')
      AND deleted_at IS NULL;

END $$;
