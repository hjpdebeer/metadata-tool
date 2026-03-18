-- Seed workflow definitions, transitions, and approvers
-- Idempotent: uses ON CONFLICT DO NOTHING for all inserts

DO $$
DECLARE
    -- Entity type IDs
    v_et_glossary       UUID;
    v_et_data_element   UUID;
    v_et_quality_rule   UUID;
    v_et_application    UUID;
    v_et_process        UUID;

    -- Workflow state IDs
    v_st_draft          UUID;
    v_st_proposed       UUID;
    v_st_under_review   UUID;
    v_st_revised        UUID;
    v_st_accepted       UUID;
    v_st_rejected       UUID;
    v_st_deprecated     UUID;

    -- Role IDs
    v_role_steward      UUID;
    v_role_owner        UUID;

    -- Workflow definition IDs
    v_wf_glossary       UUID;
    v_wf_data_element   UUID;
    v_wf_quality_rule   UUID;
    v_wf_application    UUID;
    v_wf_process        UUID;
BEGIN
    -- =========================================================================
    -- Look up entity type IDs
    -- =========================================================================
    SELECT entity_type_id INTO STRICT v_et_glossary
        FROM workflow_entity_types WHERE type_code = 'GLOSSARY_TERM';
    SELECT entity_type_id INTO STRICT v_et_data_element
        FROM workflow_entity_types WHERE type_code = 'DATA_ELEMENT';
    SELECT entity_type_id INTO STRICT v_et_quality_rule
        FROM workflow_entity_types WHERE type_code = 'QUALITY_RULE';
    SELECT entity_type_id INTO STRICT v_et_application
        FROM workflow_entity_types WHERE type_code = 'APPLICATION';
    SELECT entity_type_id INTO STRICT v_et_process
        FROM workflow_entity_types WHERE type_code = 'BUSINESS_PROCESS';

    -- =========================================================================
    -- Look up workflow state IDs
    -- =========================================================================
    SELECT state_id INTO STRICT v_st_draft        FROM workflow_states WHERE state_code = 'DRAFT';
    SELECT state_id INTO STRICT v_st_proposed      FROM workflow_states WHERE state_code = 'PROPOSED';
    SELECT state_id INTO STRICT v_st_under_review  FROM workflow_states WHERE state_code = 'UNDER_REVIEW';
    SELECT state_id INTO STRICT v_st_revised       FROM workflow_states WHERE state_code = 'REVISED';
    SELECT state_id INTO STRICT v_st_accepted      FROM workflow_states WHERE state_code = 'ACCEPTED';
    SELECT state_id INTO STRICT v_st_rejected      FROM workflow_states WHERE state_code = 'REJECTED';
    SELECT state_id INTO STRICT v_st_deprecated    FROM workflow_states WHERE state_code = 'DEPRECATED';

    -- =========================================================================
    -- Look up role IDs
    -- =========================================================================
    SELECT role_id INTO STRICT v_role_steward FROM roles WHERE role_code = 'DATA_STEWARD';
    SELECT role_id INTO STRICT v_role_owner   FROM roles WHERE role_code = 'DATA_OWNER';

    -- =========================================================================
    -- 1. Workflow definitions — one per entity type, 72-hour review SLA
    -- =========================================================================
    INSERT INTO workflow_definitions (entity_type_id, workflow_name, description, is_active, review_sla_hours)
    VALUES
        (v_et_glossary,     'Glossary Term Review',     'Standard review workflow for glossary terms',     TRUE, 72),
        (v_et_data_element, 'Data Element Review',      'Standard review workflow for data elements',      TRUE, 72),
        (v_et_quality_rule, 'Quality Rule Review',      'Standard review workflow for data quality rules', TRUE, 72),
        (v_et_application,  'Application Review',       'Standard review workflow for applications',       TRUE, 72),
        (v_et_process,      'Business Process Review',  'Standard review workflow for business processes', TRUE, 72)
    ON CONFLICT (entity_type_id, workflow_name) DO NOTHING;

    -- Retrieve the workflow definition IDs (may already exist from a prior run)
    SELECT workflow_def_id INTO STRICT v_wf_glossary
        FROM workflow_definitions WHERE entity_type_id = v_et_glossary AND workflow_name = 'Glossary Term Review';
    SELECT workflow_def_id INTO STRICT v_wf_data_element
        FROM workflow_definitions WHERE entity_type_id = v_et_data_element AND workflow_name = 'Data Element Review';
    SELECT workflow_def_id INTO STRICT v_wf_quality_rule
        FROM workflow_definitions WHERE entity_type_id = v_et_quality_rule AND workflow_name = 'Quality Rule Review';
    SELECT workflow_def_id INTO STRICT v_wf_application
        FROM workflow_definitions WHERE entity_type_id = v_et_application AND workflow_name = 'Application Review';
    SELECT workflow_def_id INTO STRICT v_wf_process
        FROM workflow_definitions WHERE entity_type_id = v_et_process AND workflow_name = 'Business Process Review';

    -- =========================================================================
    -- 2. Workflow transitions — valid state transitions for each workflow
    --    Unique constraint: (workflow_def_id, from_state_id, action_code)
    -- =========================================================================

    -- Helper: insert transitions for a single workflow definition.
    -- All five workflows share the same transition pattern.

    -- --- Glossary Term Review ---
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, required_role_id, description) VALUES
        (v_wf_glossary, v_st_draft,        v_st_proposed,     'SUBMIT',        'Submit for Review',  NULL,             'Author submits the term for review'),
        (v_wf_glossary, v_st_proposed,      v_st_under_review, 'ASSIGN_REVIEW', 'Assign Reviewer',   v_role_steward,   'Data steward assigns and begins review'),
        (v_wf_glossary, v_st_under_review,  v_st_accepted,     'APPROVE',       'Approve',           v_role_steward,   'Data steward approves the term'),
        (v_wf_glossary, v_st_under_review,  v_st_revised,      'REVISE',        'Request Revision',  v_role_steward,   'Data steward sends the term back for changes'),
        (v_wf_glossary, v_st_under_review,  v_st_rejected,     'REJECT',        'Reject',            v_role_steward,   'Data steward rejects the term'),
        (v_wf_glossary, v_st_revised,       v_st_proposed,     'RESUBMIT',      'Resubmit',          NULL,             'Author fixes issues and resubmits'),
        (v_wf_glossary, v_st_accepted,      v_st_deprecated,   'DEPRECATE',     'Deprecate',         v_role_owner,     'Data owner deprecates the accepted term'),
        (v_wf_glossary, v_st_draft,         v_st_draft,        'WITHDRAW',      'Withdraw',          NULL,             'Author withdraws the draft before submission')
    ON CONFLICT (workflow_def_id, from_state_id, action_code) DO NOTHING;

    -- --- Data Element Review ---
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, required_role_id, description) VALUES
        (v_wf_data_element, v_st_draft,        v_st_proposed,     'SUBMIT',        'Submit for Review',  NULL,             'Author submits the data element for review'),
        (v_wf_data_element, v_st_proposed,      v_st_under_review, 'ASSIGN_REVIEW', 'Assign Reviewer',   v_role_steward,   'Data steward assigns and begins review'),
        (v_wf_data_element, v_st_under_review,  v_st_accepted,     'APPROVE',       'Approve',           v_role_steward,   'Data steward approves the data element'),
        (v_wf_data_element, v_st_under_review,  v_st_revised,      'REVISE',        'Request Revision',  v_role_steward,   'Data steward sends the data element back for changes'),
        (v_wf_data_element, v_st_under_review,  v_st_rejected,     'REJECT',        'Reject',            v_role_steward,   'Data steward rejects the data element'),
        (v_wf_data_element, v_st_revised,       v_st_proposed,     'RESUBMIT',      'Resubmit',          NULL,             'Author fixes issues and resubmits'),
        (v_wf_data_element, v_st_accepted,      v_st_deprecated,   'DEPRECATE',     'Deprecate',         v_role_owner,     'Data owner deprecates the accepted data element'),
        (v_wf_data_element, v_st_draft,         v_st_draft,        'WITHDRAW',      'Withdraw',          NULL,             'Author withdraws the draft before submission')
    ON CONFLICT (workflow_def_id, from_state_id, action_code) DO NOTHING;

    -- --- Quality Rule Review ---
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, required_role_id, description) VALUES
        (v_wf_quality_rule, v_st_draft,        v_st_proposed,     'SUBMIT',        'Submit for Review',  NULL,             'Author submits the quality rule for review'),
        (v_wf_quality_rule, v_st_proposed,      v_st_under_review, 'ASSIGN_REVIEW', 'Assign Reviewer',   v_role_steward,   'Data steward assigns and begins review'),
        (v_wf_quality_rule, v_st_under_review,  v_st_accepted,     'APPROVE',       'Approve',           v_role_steward,   'Data steward approves the quality rule'),
        (v_wf_quality_rule, v_st_under_review,  v_st_revised,      'REVISE',        'Request Revision',  v_role_steward,   'Data steward sends the quality rule back for changes'),
        (v_wf_quality_rule, v_st_under_review,  v_st_rejected,     'REJECT',        'Reject',            v_role_steward,   'Data steward rejects the quality rule'),
        (v_wf_quality_rule, v_st_revised,       v_st_proposed,     'RESUBMIT',      'Resubmit',          NULL,             'Author fixes issues and resubmits'),
        (v_wf_quality_rule, v_st_accepted,      v_st_deprecated,   'DEPRECATE',     'Deprecate',         v_role_owner,     'Data owner deprecates the accepted quality rule'),
        (v_wf_quality_rule, v_st_draft,         v_st_draft,        'WITHDRAW',      'Withdraw',          NULL,             'Author withdraws the draft before submission')
    ON CONFLICT (workflow_def_id, from_state_id, action_code) DO NOTHING;

    -- --- Application Review ---
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, required_role_id, description) VALUES
        (v_wf_application, v_st_draft,        v_st_proposed,     'SUBMIT',        'Submit for Review',  NULL,             'Author submits the application for review'),
        (v_wf_application, v_st_proposed,      v_st_under_review, 'ASSIGN_REVIEW', 'Assign Reviewer',   v_role_steward,   'Data steward assigns and begins review'),
        (v_wf_application, v_st_under_review,  v_st_accepted,     'APPROVE',       'Approve',           v_role_steward,   'Data steward approves the application'),
        (v_wf_application, v_st_under_review,  v_st_revised,      'REVISE',        'Request Revision',  v_role_steward,   'Data steward sends the application back for changes'),
        (v_wf_application, v_st_under_review,  v_st_rejected,     'REJECT',        'Reject',            v_role_steward,   'Data steward rejects the application'),
        (v_wf_application, v_st_revised,       v_st_proposed,     'RESUBMIT',      'Resubmit',          NULL,             'Author fixes issues and resubmits'),
        (v_wf_application, v_st_accepted,      v_st_deprecated,   'DEPRECATE',     'Deprecate',         v_role_owner,     'Data owner deprecates the accepted application'),
        (v_wf_application, v_st_draft,         v_st_draft,        'WITHDRAW',      'Withdraw',          NULL,             'Author withdraws the draft before submission')
    ON CONFLICT (workflow_def_id, from_state_id, action_code) DO NOTHING;

    -- --- Business Process Review ---
    INSERT INTO workflow_transitions (workflow_def_id, from_state_id, to_state_id, action_code, action_name, required_role_id, description) VALUES
        (v_wf_process, v_st_draft,        v_st_proposed,     'SUBMIT',        'Submit for Review',  NULL,             'Author submits the business process for review'),
        (v_wf_process, v_st_proposed,      v_st_under_review, 'ASSIGN_REVIEW', 'Assign Reviewer',   v_role_steward,   'Data steward assigns and begins review'),
        (v_wf_process, v_st_under_review,  v_st_accepted,     'APPROVE',       'Approve',           v_role_steward,   'Data steward approves the business process'),
        (v_wf_process, v_st_under_review,  v_st_revised,      'REVISE',        'Request Revision',  v_role_steward,   'Data steward sends the business process back for changes'),
        (v_wf_process, v_st_under_review,  v_st_rejected,     'REJECT',        'Reject',            v_role_steward,   'Data steward rejects the business process'),
        (v_wf_process, v_st_revised,       v_st_proposed,     'RESUBMIT',      'Resubmit',          NULL,             'Author fixes issues and resubmits'),
        (v_wf_process, v_st_accepted,      v_st_deprecated,   'DEPRECATE',     'Deprecate',         v_role_owner,     'Data owner deprecates the accepted business process'),
        (v_wf_process, v_st_draft,         v_st_draft,        'WITHDRAW',      'Withdraw',          NULL,             'Author withdraws the draft before submission')
    ON CONFLICT (workflow_def_id, from_state_id, action_code) DO NOTHING;

    -- =========================================================================
    -- 3. Workflow approvers — DATA_STEWARD as default approver for all workflows
    --    The CHECK constraint requires at least one of approver_user_id or
    --    approver_role_id to be non-null; we set approver_role_id.
    -- =========================================================================

    -- Use a sub-select guard to avoid duplicate inserts (no unique constraint
    -- on workflow_approvers, so ON CONFLICT is not available).
    INSERT INTO workflow_approvers (workflow_def_id, approver_role_id, approval_order, is_mandatory)
    SELECT v_wf_glossary, v_role_steward, 1, TRUE
    WHERE NOT EXISTS (
        SELECT 1 FROM workflow_approvers
        WHERE workflow_def_id = v_wf_glossary AND approver_role_id = v_role_steward
    );

    INSERT INTO workflow_approvers (workflow_def_id, approver_role_id, approval_order, is_mandatory)
    SELECT v_wf_data_element, v_role_steward, 1, TRUE
    WHERE NOT EXISTS (
        SELECT 1 FROM workflow_approvers
        WHERE workflow_def_id = v_wf_data_element AND approver_role_id = v_role_steward
    );

    INSERT INTO workflow_approvers (workflow_def_id, approver_role_id, approval_order, is_mandatory)
    SELECT v_wf_quality_rule, v_role_steward, 1, TRUE
    WHERE NOT EXISTS (
        SELECT 1 FROM workflow_approvers
        WHERE workflow_def_id = v_wf_quality_rule AND approver_role_id = v_role_steward
    );

    INSERT INTO workflow_approvers (workflow_def_id, approver_role_id, approval_order, is_mandatory)
    SELECT v_wf_application, v_role_steward, 1, TRUE
    WHERE NOT EXISTS (
        SELECT 1 FROM workflow_approvers
        WHERE workflow_def_id = v_wf_application AND approver_role_id = v_role_steward
    );

    INSERT INTO workflow_approvers (workflow_def_id, approver_role_id, approval_order, is_mandatory)
    SELECT v_wf_process, v_role_steward, 1, TRUE
    WHERE NOT EXISTS (
        SELECT 1 FROM workflow_approvers
        WHERE workflow_def_id = v_wf_process AND approver_role_id = v_role_steward
    );
END $$;
