-- Data Dictionary

-- Data classification levels
CREATE TABLE data_classifications (
    classification_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    classification_code VARCHAR(50) NOT NULL UNIQUE,
    classification_name VARCHAR(128) NOT NULL,
    description TEXT,
    display_order INT NOT NULL DEFAULT 0
);

INSERT INTO data_classifications (classification_code, classification_name, description, display_order) VALUES
    ('PUBLIC',        'Public',        'Information available to the public',         10),
    ('INTERNAL',      'Internal',      'Information for internal use only',           20),
    ('CONFIDENTIAL',  'Confidential',  'Restricted access, business-sensitive',       30),
    ('RESTRICTED',    'Restricted',    'Highly restricted, regulatory/PII data',      40);

-- Data elements (business metadata)
CREATE TABLE data_elements (
    element_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    element_name        VARCHAR(512) NOT NULL,
    element_code        VARCHAR(256) NOT NULL UNIQUE,
    description         TEXT NOT NULL,
    business_definition TEXT,
    business_rules      TEXT,
    data_type           VARCHAR(64) NOT NULL,
    format_pattern      VARCHAR(256),
    allowed_values      JSONB,
    default_value       VARCHAR(512),
    is_nullable         BOOLEAN NOT NULL DEFAULT TRUE,
    is_cde              BOOLEAN NOT NULL DEFAULT FALSE,
    cde_rationale       TEXT,
    cde_designated_at   TIMESTAMPTZ,
    cde_designated_by   UUID REFERENCES users(user_id),
    glossary_term_id    UUID REFERENCES glossary_terms(term_id),
    domain_id           UUID REFERENCES glossary_domains(domain_id),
    classification_id   UUID REFERENCES data_classifications(classification_id),
    sensitivity_level   VARCHAR(50),
    status_id           UUID NOT NULL REFERENCES entity_statuses(status_id),
    owner_user_id       UUID REFERENCES users(user_id),
    steward_user_id     UUID REFERENCES users(user_id),
    search_vector       TSVECTOR,
    deleted_at          TIMESTAMPTZ,
    created_by          UUID NOT NULL REFERENCES users(user_id),
    updated_by          UUID REFERENCES users(user_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Source systems
CREATE TABLE source_systems (
    system_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    system_name        VARCHAR(256) NOT NULL,
    system_code        VARCHAR(64) NOT NULL UNIQUE,
    system_type        VARCHAR(64) NOT NULL,  -- DATABASE, API, FILE, STREAM
    description        TEXT,
    connection_details JSONB,
    deleted_at         TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Technical schemas within source systems
CREATE TABLE technical_schemas (
    schema_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    system_id    UUID NOT NULL REFERENCES source_systems(system_id),
    schema_name  VARCHAR(256) NOT NULL,
    description  TEXT,
    deleted_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(system_id, schema_name)
);

-- Technical tables/views
CREATE TABLE technical_tables (
    table_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_id    UUID NOT NULL REFERENCES technical_schemas(schema_id),
    table_name   VARCHAR(256) NOT NULL,
    table_type   VARCHAR(50) NOT NULL DEFAULT 'TABLE',  -- TABLE, VIEW, MATERIALIZED_VIEW
    description  TEXT,
    row_count    BIGINT,
    size_bytes   BIGINT,
    deleted_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(schema_id, table_name)
);

-- Technical columns
CREATE TABLE technical_columns (
    column_id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    table_id                   UUID NOT NULL REFERENCES technical_tables(table_id),
    column_name                VARCHAR(256) NOT NULL,
    ordinal_position           INT NOT NULL,
    data_type                  VARCHAR(128) NOT NULL,
    max_length                 INT,
    numeric_precision          INT,
    numeric_scale              INT,
    is_nullable                BOOLEAN NOT NULL DEFAULT TRUE,
    is_primary_key             BOOLEAN NOT NULL DEFAULT FALSE,
    is_foreign_key             BOOLEAN NOT NULL DEFAULT FALSE,
    default_expression         TEXT,
    element_id                 UUID REFERENCES data_elements(element_id),
    naming_standard_compliant  BOOLEAN,
    naming_standard_violation  TEXT,
    deleted_at                 TIMESTAMPTZ,
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(table_id, column_name)
);

-- Naming standards configuration
CREATE TABLE naming_standards (
    standard_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    standard_name VARCHAR(256) NOT NULL,
    applies_to    VARCHAR(50) NOT NULL,  -- TABLE, COLUMN, SCHEMA, API, KEY, TRIGGER
    pattern_regex VARCHAR(512) NOT NULL,
    description   TEXT NOT NULL,
    example_valid VARCHAR(256),
    example_invalid VARCHAR(256),
    is_mandatory  BOOLEAN NOT NULL DEFAULT TRUE,
    deleted_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed default naming standards
INSERT INTO naming_standards (standard_name, applies_to, pattern_regex, description, example_valid, example_invalid, is_mandatory) VALUES
    ('Table: snake_case',    'TABLE',   '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',           'Tables must use lowercase snake_case',             'customer_accounts', 'CustomerAccounts', TRUE),
    ('Column: snake_case',   'COLUMN',  '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',           'Columns must use lowercase snake_case',            'account_balance',   'AccountBalance',   TRUE),
    ('PK suffix',            'COLUMN',  '^[a-z].*_id$',                             'Primary key columns must end with _id',            'customer_id',       'customer_key',     FALSE),
    ('FK suffix',            'COLUMN',  '^[a-z].*_id$',                             'Foreign key columns must end with _id',            'account_id',        'account_ref',      FALSE),
    ('Boolean prefix',       'COLUMN',  '^(is_|has_|can_|should_|was_|did_)[a-z]',  'Boolean columns must start with is_/has_/can_/etc','is_active',         'active',           FALSE),
    ('Timestamp suffix',     'COLUMN',  '^[a-z].*_(at|date|time)$',                 'Timestamp columns must end with _at/_date/_time',  'created_at',        'creation',         FALSE),
    ('Schema: snake_case',   'SCHEMA',  '^[a-z][a-z0-9]*(_[a-z0-9]+)*$',           'Schemas must use lowercase snake_case',            'customer_data',     'CustomerData',     TRUE),
    ('API: kebab-case path', 'API',     '^/[a-z][a-z0-9]*(-[a-z0-9]+)*(/[a-z][a-z0-9]*(-[a-z0-9]+)*)*$', 'API paths must use kebab-case', '/data-elements', '/dataElements', TRUE);

-- Indexes
CREATE INDEX idx_data_elements_domain ON data_elements(domain_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_data_elements_status ON data_elements(status_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_data_elements_cde ON data_elements(is_cde) WHERE is_cde = TRUE AND deleted_at IS NULL;
CREATE INDEX idx_data_elements_glossary ON data_elements(glossary_term_id) WHERE glossary_term_id IS NOT NULL;
CREATE INDEX idx_data_elements_search ON data_elements USING GIN(search_vector);
CREATE INDEX idx_technical_columns_table ON technical_columns(table_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_technical_columns_element ON technical_columns(element_id) WHERE element_id IS NOT NULL;
