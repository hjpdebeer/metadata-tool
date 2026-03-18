-- Identity & Access Management

CREATE TABLE roles (
    role_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_code     VARCHAR(50) NOT NULL UNIQUE,
    role_name     VARCHAR(128) NOT NULL,
    description   TEXT,
    is_system_role BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE users (
    user_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        VARCHAR(128) NOT NULL UNIQUE,
    email           VARCHAR(256) NOT NULL UNIQUE,
    display_name    VARCHAR(256) NOT NULL,
    first_name      VARCHAR(128),
    last_name       VARCHAR(128),
    department      VARCHAR(256),
    job_title       VARCHAR(256),
    entra_object_id VARCHAR(128) UNIQUE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    last_login_at   TIMESTAMPTZ,
    deleted_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      UUID REFERENCES users(user_id),
    updated_by      UUID REFERENCES users(user_id)
);

CREATE TABLE user_roles (
    user_role_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(user_id),
    role_id       UUID NOT NULL REFERENCES roles(role_id),
    scope_type    VARCHAR(50),    -- e.g. DOMAIN, APPLICATION
    scope_id      UUID,           -- ID of the scoped entity
    effective_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    effective_to   TIMESTAMPTZ,
    granted_by    UUID REFERENCES users(user_id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, role_id, scope_type, scope_id)
);

CREATE TABLE sso_sessions (
    session_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID NOT NULL REFERENCES users(user_id),
    entra_session_id VARCHAR(256),
    ip_address     INET,
    user_agent     TEXT,
    started_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at     TIMESTAMPTZ NOT NULL,
    ended_at       TIMESTAMPTZ
);

-- Seed system roles
INSERT INTO roles (role_code, role_name, description, is_system_role) VALUES
    ('ADMIN',                 'System Administrator',       'Full system access',                          TRUE),
    ('DATA_OWNER',            'Data Owner',                 'Accountable for data within a domain',        TRUE),
    ('DATA_STEWARD',          'Data Steward',               'Responsible for data quality and governance',  TRUE),
    ('DATA_PRODUCER',         'Data Producer',              'Creates and maintains data',                   TRUE),
    ('DATA_CONSUMER',         'Data Consumer',              'Uses data for analysis and reporting',         TRUE),
    ('APP_BUSINESS_OWNER',    'Application Business Owner', 'Business owner of an application',            TRUE),
    ('APP_TECHNICAL_OWNER',   'Application Technical Owner','Technical owner of an application',           TRUE),
    ('BUSINESS_PROCESS_OWNER','Business Process Owner',     'Owner of a business process',                 TRUE),
    ('VIEWER',                'Viewer',                     'Read-only access to metadata',                TRUE);

-- Indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_entra ON users(entra_object_id) WHERE entra_object_id IS NOT NULL;
CREATE INDEX idx_users_active ON users(is_active) WHERE deleted_at IS NULL;
CREATE INDEX idx_user_roles_user ON user_roles(user_id);
CREATE INDEX idx_user_roles_role ON user_roles(role_id);
