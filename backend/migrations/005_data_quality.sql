-- Data Quality

CREATE TABLE quality_dimensions (
    dimension_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dimension_code VARCHAR(50) NOT NULL UNIQUE,
    dimension_name VARCHAR(128) NOT NULL,
    description    TEXT NOT NULL,
    display_order  INT NOT NULL DEFAULT 0
);

INSERT INTO quality_dimensions (dimension_code, dimension_name, description, display_order) VALUES
    ('COMPLETENESS', 'Completeness', 'The degree to which all required data is present',                        10),
    ('UNIQUENESS',   'Uniqueness',   'The degree to which there are no duplicate records',                      20),
    ('VALIDITY',     'Validity',     'The degree to which data conforms to defined business rules and formats',  30),
    ('TIMELINESS',   'Timeliness',   'The degree to which data is available when needed',                       40),
    ('ACCURACY',     'Accuracy',     'The degree to which data correctly represents the real-world entity',     50),
    ('CONSISTENCY',  'Consistency',  'The degree to which data is consistent across systems and over time',     60);

CREATE TABLE quality_rule_types (
    rule_type_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type_code    VARCHAR(50) NOT NULL UNIQUE,
    type_name    VARCHAR(128) NOT NULL,
    description  TEXT,
    sql_template TEXT
);

INSERT INTO quality_rule_types (type_code, type_name, description) VALUES
    ('NOT_NULL',     'Not Null Check',      'Verifies the field is not null or empty'),
    ('UNIQUE',       'Uniqueness Check',    'Verifies no duplicate values exist'),
    ('RANGE',        'Range Check',         'Verifies value falls within a defined range'),
    ('PATTERN',      'Pattern Match',       'Verifies value matches a regex pattern'),
    ('REFERENTIAL',  'Referential Check',   'Verifies value exists in a reference dataset'),
    ('FRESHNESS',    'Freshness Check',     'Verifies data has been updated within SLA'),
    ('CROSS_FIELD',  'Cross-Field Check',   'Validates relationships between fields'),
    ('AGGREGATE',    'Aggregate Check',     'Validates aggregate metrics like row counts'),
    ('CUSTOM_SQL',   'Custom SQL',          'Custom SQL-based quality check');

CREATE TABLE quality_rules (
    rule_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_name            VARCHAR(256) NOT NULL,
    rule_code            VARCHAR(128) NOT NULL UNIQUE,
    description          TEXT NOT NULL,
    dimension_id         UUID NOT NULL REFERENCES quality_dimensions(dimension_id),
    rule_type_id         UUID NOT NULL REFERENCES quality_rule_types(rule_type_id),
    element_id           UUID REFERENCES data_elements(element_id),
    column_id            UUID REFERENCES technical_columns(column_id),
    rule_definition      JSONB NOT NULL,
    threshold_percentage NUMERIC(5,2) DEFAULT 100.00,
    severity             VARCHAR(20) NOT NULL DEFAULT 'MEDIUM' CHECK(severity IN ('LOW','MEDIUM','HIGH','CRITICAL')),
    is_active            BOOLEAN NOT NULL DEFAULT TRUE,
    status_id            UUID NOT NULL REFERENCES entity_statuses(status_id),
    owner_user_id        UUID REFERENCES users(user_id),
    deleted_at           TIMESTAMPTZ,
    created_by           UUID NOT NULL REFERENCES users(user_id),
    updated_by           UUID REFERENCES users(user_id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE quality_assessments (
    assessment_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id          UUID NOT NULL REFERENCES quality_rules(rule_id),
    assessed_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    records_assessed BIGINT NOT NULL DEFAULT 0,
    records_passed   BIGINT NOT NULL DEFAULT 0,
    records_failed   BIGINT NOT NULL DEFAULT 0,
    score_percentage NUMERIC(5,2) NOT NULL,
    status           VARCHAR(20) NOT NULL DEFAULT 'COMPLETED' CHECK(status IN ('RUNNING','COMPLETED','FAILED','CANCELLED')),
    error_message    TEXT,
    details          JSONB,
    executed_by      UUID REFERENCES users(user_id),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE quality_scores (
    score_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    element_id    UUID REFERENCES data_elements(element_id),
    table_id      UUID REFERENCES technical_tables(table_id),
    dimension_id  UUID REFERENCES quality_dimensions(dimension_id),
    overall_score NUMERIC(5,2) NOT NULL,
    period_start  TIMESTAMPTZ NOT NULL,
    period_end    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_quality_rules_dimension ON quality_rules(dimension_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_quality_rules_element ON quality_rules(element_id) WHERE element_id IS NOT NULL;
CREATE INDEX idx_quality_rules_active ON quality_rules(is_active) WHERE is_active = TRUE AND deleted_at IS NULL;
CREATE INDEX idx_quality_assessments_rule ON quality_assessments(rule_id);
CREATE INDEX idx_quality_assessments_date ON quality_assessments(assessed_at DESC);
CREATE INDEX idx_quality_scores_element ON quality_scores(element_id) WHERE element_id IS NOT NULL;
