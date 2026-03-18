-- Business Glossary

-- Lookup: entity statuses (shared across domains)
CREATE TABLE entity_statuses (
    status_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status_code VARCHAR(50) NOT NULL UNIQUE,
    status_name VARCHAR(128) NOT NULL,
    description TEXT,
    display_order INT NOT NULL DEFAULT 0
);

INSERT INTO entity_statuses (status_code, status_name, description, display_order) VALUES
    ('DRAFT',       'Draft',        'Initial creation, not yet submitted',   10),
    ('PROPOSED',    'Proposed',     'Submitted for review',                  20),
    ('UNDER_REVIEW','Under Review', 'Being reviewed by steward/approver',    30),
    ('REVISED',     'Revised',      'Returned for revision',                 40),
    ('ACCEPTED',    'Accepted',     'Approved and active',                   50),
    ('REJECTED',    'Rejected',     'Rejected during review',                60),
    ('DEPRECATED',  'Deprecated',   'No longer active',                      70);

-- Business domains (hierarchical)
CREATE TABLE glossary_domains (
    domain_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain_name      VARCHAR(256) NOT NULL,
    description      TEXT,
    parent_domain_id UUID REFERENCES glossary_domains(domain_id),
    owner_user_id    UUID REFERENCES users(user_id),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Term categories
CREATE TABLE glossary_categories (
    category_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_name VARCHAR(256) NOT NULL,
    description   TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Relationship types between terms
CREATE TABLE glossary_term_relationship_types (
    relationship_type_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type_code     VARCHAR(50) NOT NULL UNIQUE,
    type_name     VARCHAR(128) NOT NULL,
    description   TEXT,
    is_symmetric  BOOLEAN NOT NULL DEFAULT FALSE,
    inverse_code  VARCHAR(50)
);

INSERT INTO glossary_term_relationship_types (type_code, type_name, is_symmetric, inverse_code) VALUES
    ('SYNONYM',    'Synonym',       TRUE,  NULL),
    ('RELATED',    'Related To',    TRUE,  NULL),
    ('PARENT',     'Parent Of',     FALSE, 'CHILD'),
    ('CHILD',      'Child Of',      FALSE, 'PARENT'),
    ('SUPERSEDES', 'Supersedes',    FALSE, 'SUPERSEDED_BY'),
    ('SUPERSEDED_BY', 'Superseded By', FALSE, 'SUPERSEDES');

-- Business glossary terms
CREATE TABLE glossary_terms (
    term_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_name            VARCHAR(512) NOT NULL,
    definition           TEXT NOT NULL,
    business_context     TEXT,
    examples             TEXT,
    abbreviation         VARCHAR(64),
    domain_id            UUID REFERENCES glossary_domains(domain_id),
    category_id          UUID REFERENCES glossary_categories(category_id),
    status_id            UUID NOT NULL REFERENCES entity_statuses(status_id),
    owner_user_id        UUID REFERENCES users(user_id),
    steward_user_id      UUID REFERENCES users(user_id),
    version_number       INT NOT NULL DEFAULT 1,
    is_current_version   BOOLEAN NOT NULL DEFAULT TRUE,
    previous_version_id  UUID REFERENCES glossary_terms(term_id),
    source_reference     TEXT,
    regulatory_reference TEXT,
    search_vector        TSVECTOR,
    deleted_at           TIMESTAMPTZ,
    created_by           UUID NOT NULL REFERENCES users(user_id),
    updated_by           UUID REFERENCES users(user_id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Term relationships
CREATE TABLE glossary_term_relationships (
    relationship_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_term_id        UUID NOT NULL REFERENCES glossary_terms(term_id),
    target_term_id        UUID NOT NULL REFERENCES glossary_terms(term_id),
    relationship_type_id  UUID NOT NULL REFERENCES glossary_term_relationship_types(relationship_type_id),
    relationship_description TEXT,
    created_by            UUID REFERENCES users(user_id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_term_id, target_term_id, relationship_type_id),
    CHECK(source_term_id != target_term_id)
);

-- Term aliases / alternate names
CREATE TABLE glossary_term_aliases (
    alias_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id    UUID NOT NULL REFERENCES glossary_terms(term_id),
    alias_name VARCHAR(512) NOT NULL,
    alias_type VARCHAR(50),   -- ABBREVIATION, ACRONYM, ALTERNATE, LEGACY
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_glossary_terms_domain ON glossary_terms(domain_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_glossary_terms_status ON glossary_terms(status_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_glossary_terms_name ON glossary_terms(term_name) WHERE deleted_at IS NULL;
CREATE INDEX idx_glossary_terms_search ON glossary_terms USING GIN(search_vector);
CREATE INDEX idx_glossary_terms_current ON glossary_terms(term_name, domain_id) WHERE is_current_version = TRUE AND deleted_at IS NULL;
