-- Workflow Engine

CREATE TABLE workflow_entity_types (
    entity_type_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type_code      VARCHAR(50) NOT NULL UNIQUE,
    type_name      VARCHAR(128) NOT NULL,
    table_name     VARCHAR(128) NOT NULL,
    description    TEXT
);

INSERT INTO workflow_entity_types (type_code, type_name, table_name) VALUES
    ('GLOSSARY_TERM',     'Glossary Term',     'glossary_terms'),
    ('DATA_ELEMENT',      'Data Element',      'data_elements'),
    ('QUALITY_RULE',      'Quality Rule',      'quality_rules'),
    ('APPLICATION',       'Application',       'applications'),
    ('BUSINESS_PROCESS',  'Business Process',  'business_processes');

CREATE TABLE workflow_states (
    state_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    state_code  VARCHAR(50) NOT NULL UNIQUE,
    state_name  VARCHAR(128) NOT NULL,
    description TEXT,
    is_initial  BOOLEAN NOT NULL DEFAULT FALSE,
    is_terminal BOOLEAN NOT NULL DEFAULT FALSE,
    display_order INT NOT NULL DEFAULT 0
);

INSERT INTO workflow_states (state_code, state_name, is_initial, is_terminal, display_order) VALUES
    ('DRAFT',        'Draft',        TRUE,  FALSE, 10),
    ('PROPOSED',     'Proposed',     FALSE, FALSE, 20),
    ('UNDER_REVIEW', 'Under Review', FALSE, FALSE, 30),
    ('REVISED',      'Revised',      FALSE, FALSE, 40),
    ('ACCEPTED',     'Accepted',     FALSE, TRUE,  50),
    ('REJECTED',     'Rejected',     FALSE, TRUE,  60),
    ('DEPRECATED',   'Deprecated',   FALSE, TRUE,  70);

CREATE TABLE workflow_definitions (
    workflow_def_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type_id   UUID NOT NULL REFERENCES workflow_entity_types(entity_type_id),
    workflow_name    VARCHAR(256) NOT NULL,
    description      TEXT,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    review_sla_hours INT DEFAULT 72,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(entity_type_id, workflow_name)
);

CREATE TABLE workflow_transitions (
    transition_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_def_id UUID NOT NULL REFERENCES workflow_definitions(workflow_def_id),
    from_state_id   UUID NOT NULL REFERENCES workflow_states(state_id),
    to_state_id     UUID NOT NULL REFERENCES workflow_states(state_id),
    action_code     VARCHAR(50) NOT NULL,
    action_name     VARCHAR(128) NOT NULL,
    required_role_id UUID REFERENCES roles(role_id),
    description     TEXT,
    UNIQUE(workflow_def_id, from_state_id, action_code)
);

CREATE TABLE workflow_approvers (
    approver_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_def_id  UUID NOT NULL REFERENCES workflow_definitions(workflow_def_id),
    approver_user_id UUID REFERENCES users(user_id),
    approver_role_id UUID REFERENCES roles(role_id),
    approval_order   INT NOT NULL DEFAULT 1,
    is_mandatory     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK(approver_user_id IS NOT NULL OR approver_role_id IS NOT NULL)
);

CREATE TABLE workflow_instances (
    instance_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_def_id  UUID NOT NULL REFERENCES workflow_definitions(workflow_def_id),
    entity_type_id   UUID NOT NULL REFERENCES workflow_entity_types(entity_type_id),
    entity_id        UUID NOT NULL,
    current_state_id UUID NOT NULL REFERENCES workflow_states(state_id),
    initiated_by     UUID NOT NULL REFERENCES users(user_id),
    initiated_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at     TIMESTAMPTZ,
    completion_notes TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE workflow_tasks (
    task_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id          UUID NOT NULL REFERENCES workflow_instances(instance_id),
    task_type            VARCHAR(50) NOT NULL DEFAULT 'APPROVE',
    task_name            VARCHAR(256) NOT NULL,
    description          TEXT,
    assigned_to_user_id  UUID REFERENCES users(user_id),
    assigned_to_role_id  UUID REFERENCES roles(role_id),
    status               VARCHAR(20) NOT NULL DEFAULT 'PENDING' CHECK(status IN ('PENDING','IN_PROGRESS','COMPLETED','CANCELLED')),
    due_date             TIMESTAMPTZ,
    completed_at         TIMESTAMPTZ,
    completed_by         UUID REFERENCES users(user_id),
    decision             VARCHAR(50),
    comments             TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK(assigned_to_user_id IS NOT NULL OR assigned_to_role_id IS NOT NULL)
);

CREATE TABLE workflow_history (
    history_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id  UUID NOT NULL REFERENCES workflow_instances(instance_id),
    from_state_id UUID NOT NULL REFERENCES workflow_states(state_id),
    to_state_id  UUID NOT NULL REFERENCES workflow_states(state_id),
    action       VARCHAR(50) NOT NULL,
    performed_by UUID NOT NULL REFERENCES users(user_id),
    performed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    comments     TEXT
);

-- Indexes
CREATE INDEX idx_workflow_instances_entity ON workflow_instances(entity_type_id, entity_id);
CREATE INDEX idx_workflow_instances_active ON workflow_instances(entity_type_id, entity_id) WHERE completed_at IS NULL;
CREATE INDEX idx_workflow_tasks_user ON workflow_tasks(assigned_to_user_id) WHERE status = 'PENDING';
CREATE INDEX idx_workflow_tasks_role ON workflow_tasks(assigned_to_role_id) WHERE status = 'PENDING';
CREATE INDEX idx_workflow_tasks_due ON workflow_tasks(due_date) WHERE status = 'PENDING';
CREATE INDEX idx_workflow_history_instance ON workflow_history(instance_id);
