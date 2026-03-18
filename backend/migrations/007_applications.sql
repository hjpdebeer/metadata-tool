-- Business Application Registry

CREATE TABLE application_classifications (
    classification_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    classification_code VARCHAR(50) NOT NULL UNIQUE,
    classification_name VARCHAR(128) NOT NULL,
    description         TEXT,
    display_order       INT NOT NULL DEFAULT 0
);

INSERT INTO application_classifications (classification_code, classification_name, description, display_order) VALUES
    ('CORE_BANKING',    'Core Banking',      'Core banking and transaction systems',        10),
    ('PAYMENTS',        'Payments',          'Payment processing systems',                   20),
    ('LENDING',         'Lending',           'Lending and credit systems',                   30),
    ('TREASURY',        'Treasury',          'Treasury and trading systems',                 40),
    ('RISK',            'Risk Management',   'Risk management and compliance systems',       50),
    ('CRM',             'CRM',              'Customer relationship management',              60),
    ('HR',              'Human Resources',   'HR and payroll systems',                       70),
    ('FINANCE',         'Finance',           'Financial accounting and reporting',            80),
    ('DATA_ANALYTICS',  'Data & Analytics',  'Data warehouse, BI, and analytics platforms',  90),
    ('INFRASTRUCTURE',  'Infrastructure',    'Infrastructure and middleware services',       100),
    ('DIGITAL',         'Digital Channels',  'Online banking, mobile, and digital channels', 110),
    ('OTHER',           'Other',             'Other applications',                           999);

CREATE TABLE applications (
    application_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_name     VARCHAR(256) NOT NULL,
    application_code     VARCHAR(64) NOT NULL UNIQUE,
    description          TEXT NOT NULL,
    classification_id    UUID REFERENCES application_classifications(classification_id),
    status_id            UUID NOT NULL REFERENCES entity_statuses(status_id),
    business_owner_id    UUID REFERENCES users(user_id),
    technical_owner_id   UUID REFERENCES users(user_id),
    vendor               VARCHAR(256),
    version              VARCHAR(64),
    deployment_type      VARCHAR(50),  -- ON_PREMISE, CLOUD, HYBRID, SAAS
    technology_stack     JSONB,
    is_critical          BOOLEAN NOT NULL DEFAULT FALSE,
    criticality_rationale TEXT,
    go_live_date         TIMESTAMPTZ,
    retirement_date      TIMESTAMPTZ,
    documentation_url    TEXT,
    deleted_at           TIMESTAMPTZ,
    created_by           UUID NOT NULL REFERENCES users(user_id),
    updated_by           UUID REFERENCES users(user_id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Link applications to data elements
CREATE TABLE application_data_elements (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id          UUID NOT NULL REFERENCES applications(application_id),
    element_id              UUID NOT NULL REFERENCES data_elements(element_id),
    usage_type              VARCHAR(50) NOT NULL DEFAULT 'BOTH' CHECK(usage_type IN ('PRODUCER','CONSUMER','BOTH')),
    is_authoritative_source BOOLEAN NOT NULL DEFAULT FALSE,
    description             TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(application_id, element_id)
);

-- Interfaces between applications
CREATE TABLE application_interfaces (
    interface_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_app_id  UUID NOT NULL REFERENCES applications(application_id),
    target_app_id  UUID NOT NULL REFERENCES applications(application_id),
    interface_name VARCHAR(256) NOT NULL,
    interface_type VARCHAR(50) NOT NULL,  -- API, FILE, DB_LINK, MESSAGE_QUEUE, BATCH
    protocol       VARCHAR(64),
    frequency      VARCHAR(64),
    description    TEXT,
    deleted_at     TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK(source_app_id != target_app_id)
);

-- Add FK from lineage_nodes to applications
ALTER TABLE lineage_nodes
    ADD CONSTRAINT fk_lineage_nodes_application
    FOREIGN KEY (application_id) REFERENCES applications(application_id);

-- Indexes
CREATE INDEX idx_applications_classification ON applications(classification_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_applications_status ON applications(status_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_applications_critical ON applications(is_critical) WHERE is_critical = TRUE AND deleted_at IS NULL;
CREATE INDEX idx_app_data_elements_app ON application_data_elements(application_id);
CREATE INDEX idx_app_data_elements_element ON application_data_elements(element_id);
