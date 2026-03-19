-- ============================================================================
-- Migration: 017_glossary_enhancement.sql
-- Purpose: Enhance Business Glossary with 45-field metadata specification
-- Author: Hendrik de Beer
-- Date: 2026-03-19
-- ============================================================================
-- This migration adds new lookup tables, columns, and junction tables to support
-- the complete 45-field business glossary specification while preserving all
-- existing data and relationships.
-- ============================================================================

-- ============================================================================
-- PART 1: NEW LOOKUP TABLES
-- ============================================================================

-- 1.1 Term Types (what kind of term this is)
CREATE TABLE glossary_term_types (
    term_type_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type_code       VARCHAR(50) NOT NULL UNIQUE,
    type_name       VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_term_types (type_code, type_name, description, display_order) VALUES
    ('KPI',              'KPI / Financial Metric',  'Key performance indicator or financial measurement',           10),
    ('BUSINESS_CONCEPT', 'Business Concept',        'Core business domain concept or entity',                       20),
    ('REGULATORY_TERM',  'Regulatory Term',         'Term defined by regulation or supervisory guidance',           30),
    ('TECHNICAL_TERM',   'Technical Term',          'Data architecture or system-related term',                     40),
    ('PROCESS_TERM',     'Process Term',            'Business process or workflow related term',                    50),
    ('PRODUCT_TERM',     'Product Term',            'Financial product or service related term',                    60),
    ('RISK_TERM',        'Risk Term',               'Risk management and assessment related term',                  70),
    ('COMPLIANCE_TERM',  'Compliance Term',         'Compliance, AML, or regulatory adherence term',                80);

-- 1.2 Review Frequencies (how often a term should be reviewed)
CREATE TABLE glossary_review_frequencies (
    frequency_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    frequency_code  VARCHAR(50) NOT NULL UNIQUE,
    frequency_name  VARCHAR(128) NOT NULL,
    months_interval INT NOT NULL,
    display_order   INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_review_frequencies (frequency_code, frequency_name, months_interval, display_order) VALUES
    ('MONTHLY',      'Monthly',       1,  10),
    ('QUARTERLY',    'Quarterly',     3,  20),
    ('SEMI_ANNUAL',  'Semi-Annual',   6,  30),
    ('ANNUAL',       'Annual',        12, 40),
    ('BIENNIAL',     'Biennial',      24, 50);

-- 1.3 Confidence Levels (how confident we are in the definition)
CREATE TABLE glossary_confidence_levels (
    confidence_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    level_code      VARCHAR(50) NOT NULL UNIQUE,
    level_name      VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_confidence_levels (level_code, level_name, description, display_order) VALUES
    ('HIGH',   'High',   'Definition is well-established, authoritative, and widely accepted',     10),
    ('MEDIUM', 'Medium', 'Definition is generally accepted but may require further validation',    20),
    ('LOW',    'Low',    'Definition is provisional, under review, or sources conflict',           30);

-- 1.4 Visibility Levels (who can see this term)
CREATE TABLE glossary_visibility_levels (
    visibility_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    visibility_code VARCHAR(50) NOT NULL UNIQUE,
    visibility_name VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_visibility_levels (visibility_code, visibility_name, description, display_order) VALUES
    ('ENTERPRISE_WIDE',  'Enterprise-Wide',  'Visible to all users across the organization',               10),
    ('DOMAIN_SPECIFIC',  'Domain-Specific',  'Visible only to users within the data domain',               20),
    ('RESTRICTED',       'Restricted',       'Visible only to designated users with explicit access',      30);

-- 1.5 Units of Measure (for KPIs and metrics)
CREATE TABLE glossary_units_of_measure (
    unit_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    unit_code     VARCHAR(50) NOT NULL UNIQUE,
    unit_name     VARCHAR(128) NOT NULL,
    unit_symbol   VARCHAR(20),
    description   TEXT,
    display_order INT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_units_of_measure (unit_code, unit_name, unit_symbol, description, display_order) VALUES
    ('PERCENTAGE',  'Percentage',       '%',    'Value expressed as a percentage',                   10),
    ('CURRENCY',    'Currency',         NULL,   'Monetary value (currency varies by context)',       20),
    ('COUNT',       'Count',            '#',    'Discrete count of items or occurrences',            30),
    ('RATIO',       'Ratio',            NULL,   'Ratio between two quantities',                      40),
    ('DAYS',        'Days',             'd',    'Duration measured in days',                         50),
    ('MONTHS',      'Months',           'mo',   'Duration measured in months',                       60),
    ('YEARS',       'Years',            'yr',   'Duration measured in years',                        70),
    ('BOOLEAN',     'Boolean',          NULL,   'True/False indicator',                              80),
    ('TEXT',        'Text',             NULL,   'Free-form text value',                              90),
    ('DATE',        'Date',             NULL,   'Calendar date',                                     100),
    ('DATETIME',    'Date and Time',    NULL,   'Timestamp with date and time',                      110),
    ('RATE',        'Rate',             NULL,   'Rate per unit (e.g., interest rate, growth rate)', 120),
    ('SCORE',       'Score',            NULL,   'Numeric score (e.g., credit score, risk score)',   130),
    ('INDEX',       'Index',            NULL,   'Index value (e.g., benchmark index)',              140);

-- 1.6 Regulatory Tags (regulatory frameworks that apply to a term)
CREATE TABLE glossary_regulatory_tags (
    tag_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tag_code      VARCHAR(50) NOT NULL UNIQUE,
    tag_name      VARCHAR(128) NOT NULL,
    description   TEXT,
    jurisdiction  VARCHAR(128),
    display_order INT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_regulatory_tags (tag_code, tag_name, description, jurisdiction, display_order) VALUES
    ('BCBS_239',   'BCBS 239',           'Principles for effective risk data aggregation and reporting',        'International', 10),
    ('IFRS_9',     'IFRS 9',             'Financial instruments standard (classification, impairment, hedging)','International', 20),
    ('IFRS_17',    'IFRS 17',            'Insurance contracts standard',                                        'International', 30),
    ('BASEL_III',  'Basel III',          'Global regulatory framework for banks (capital, leverage, liquidity)','International', 40),
    ('FATCA',      'FATCA',              'US Foreign Account Tax Compliance Act',                               'United States', 50),
    ('CRS',        'CRS',                'Common Reporting Standard for automatic exchange of financial info',  'International', 60),
    ('GDPR',       'GDPR',               'General Data Protection Regulation',                                  'European Union',70),
    ('PCI_DSS',    'PCI DSS',            'Payment Card Industry Data Security Standard',                        'International', 80),
    ('SOX',        'SOX',                'Sarbanes-Oxley Act financial reporting requirements',                 'United States', 90),
    ('AML_CFT',    'AML/CFT',            'Anti-Money Laundering / Counter Financing of Terrorism',              'International', 100),
    ('MiFID_II',   'MiFID II',           'Markets in Financial Instruments Directive',                          'European Union',110),
    ('DORA',       'DORA',               'Digital Operational Resilience Act',                                  'European Union',120),
    ('LOCAL_REG',  'Local Regulation',   'Jurisdiction-specific regulatory requirement',                        'Varies',        130);

-- 1.7 Subject Areas (business areas where the term is used)
CREATE TABLE glossary_subject_areas (
    subject_area_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    area_code       VARCHAR(50) NOT NULL UNIQUE,
    area_name       VARCHAR(128) NOT NULL,
    description     TEXT,
    display_order   INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_subject_areas (area_code, area_name, description, display_order) VALUES
    ('RETAIL_BANKING',    'Retail Banking',       'Consumer banking products and services',                    10),
    ('CORPORATE_BANKING', 'Corporate Banking',    'Business and commercial banking services',                  20),
    ('INVESTMENT_BANKING','Investment Banking',   'Capital markets, M&A, and securities services',             30),
    ('WEALTH_MANAGEMENT', 'Wealth Management',    'Private banking and wealth advisory services',              40),
    ('TREASURY',          'Treasury',             'Liquidity management, funding, and ALM',                    50),
    ('RISK_MANAGEMENT',   'Risk Management',      'Credit, market, operational, and enterprise risk',          60),
    ('COMPLIANCE',        'Compliance',           'Regulatory compliance and financial crime prevention',      70),
    ('FINANCE',           'Finance',              'Accounting, financial reporting, and planning',             80),
    ('OPERATIONS',        'Operations',           'Back-office processing and operational functions',          90),
    ('TECHNOLOGY',        'Technology',           'IT infrastructure, applications, and data management',      100),
    ('HUMAN_RESOURCES',   'Human Resources',      'Workforce management and employee services',                110),
    ('LEGAL',             'Legal',                'Legal affairs, contracts, and regulatory interpretation',   120),
    ('AUDIT',             'Audit',                'Internal and external audit functions',                     130);

-- 1.8 Tags (freeform keywords for discoverability)
CREATE TABLE glossary_tags (
    tag_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tag_name    VARCHAR(128) NOT NULL UNIQUE,
    description TEXT,
    created_by  UUID REFERENCES users(user_id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 1.9 Languages (for multi-language support)
CREATE TABLE glossary_languages (
    language_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    language_code VARCHAR(10) NOT NULL UNIQUE,
    language_name VARCHAR(128) NOT NULL,
    is_default    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO glossary_languages (language_code, language_name, is_default) VALUES
    ('en', 'English',  TRUE),
    ('ar', 'Arabic',   FALSE),
    ('fr', 'French',   FALSE),
    ('de', 'German',   FALSE),
    ('es', 'Spanish',  FALSE),
    ('zh', 'Chinese',  FALSE),
    ('hi', 'Hindi',    FALSE),
    ('pt', 'Portuguese', FALSE);

-- ============================================================================
-- PART 2: ADD NEW RELATIONSHIP TYPES
-- ============================================================================

-- Add CONFLICTING and IS_PART_OF relationship types
INSERT INTO glossary_term_relationship_types (type_code, type_name, description, is_symmetric, inverse_code)
SELECT type_code, type_name, description, is_symmetric, inverse_code
FROM (VALUES
    ('CONFLICTING', 'Conflicts With', 'Terms that have incompatible or contradictory definitions', TRUE,  NULL),
    ('IS_PART_OF',  'Is Part Of',     'Term is a component or subset of another term',             FALSE, 'HAS_PART'),
    ('HAS_PART',    'Has Part',       'Term contains or is composed of other terms',               FALSE, 'IS_PART_OF')
) AS new_types(type_code, type_name, description, is_symmetric, inverse_code)
WHERE NOT EXISTS (
    SELECT 1 FROM glossary_term_relationship_types
    WHERE type_code = new_types.type_code
);

-- ============================================================================
-- PART 3: ADD DOMAIN CODE TO glossary_domains
-- ============================================================================

-- Add domain_code for term_code generation (GLO-{DOMAIN_CODE}-{SEQ})
ALTER TABLE glossary_domains
    ADD COLUMN IF NOT EXISTS domain_code VARCHAR(10);

-- Populate domain codes for existing domains
UPDATE glossary_domains SET domain_code = 'CUS' WHERE domain_name = 'Customer' AND domain_code IS NULL;
UPDATE glossary_domains SET domain_code = 'ACC' WHERE domain_name = 'Account' AND domain_code IS NULL;
UPDATE glossary_domains SET domain_code = 'TXN' WHERE domain_name = 'Transaction' AND domain_code IS NULL;
UPDATE glossary_domains SET domain_code = 'PRD' WHERE domain_name = 'Product' AND domain_code IS NULL;
UPDATE glossary_domains SET domain_code = 'RSK' WHERE domain_name = 'Risk' AND domain_code IS NULL;
UPDATE glossary_domains SET domain_code = 'CMP' WHERE domain_name = 'Compliance' AND domain_code IS NULL;
UPDATE glossary_domains SET domain_code = 'OPS' WHERE domain_name = 'Operations' AND domain_code IS NULL;
UPDATE glossary_domains SET domain_code = 'FIN' WHERE domain_name = 'Financial Reporting' AND domain_code IS NULL;

-- Make domain_code required and unique after population
ALTER TABLE glossary_domains
    ALTER COLUMN domain_code SET NOT NULL;
ALTER TABLE glossary_domains
    ADD CONSTRAINT uq_glossary_domains_code UNIQUE (domain_code);

-- ============================================================================
-- PART 4: SEQUENCE FOR TERM CODE GENERATION
-- ============================================================================

-- Sequence per domain would be complex; use a global sequence with domain prefix
CREATE SEQUENCE IF NOT EXISTS glossary_term_code_seq START WITH 1;

-- ============================================================================
-- PART 5: ADD NEW COLUMNS TO glossary_terms
-- ============================================================================

ALTER TABLE glossary_terms

    -- Section 1: Core Identity
    ADD COLUMN IF NOT EXISTS term_code VARCHAR(32) UNIQUE,

    -- Section 2: Definition & Semantics
    ADD COLUMN IF NOT EXISTS definition_notes TEXT,
    ADD COLUMN IF NOT EXISTS counter_examples TEXT,
    ADD COLUMN IF NOT EXISTS formula TEXT,
    ADD COLUMN IF NOT EXISTS unit_of_measure_id UUID REFERENCES glossary_units_of_measure(unit_id),

    -- Section 3: Classification
    ADD COLUMN IF NOT EXISTS term_type_id UUID REFERENCES glossary_term_types(term_type_id),
    ADD COLUMN IF NOT EXISTS classification_id UUID REFERENCES data_classifications(classification_id),

    -- Section 4: Ownership
    ADD COLUMN IF NOT EXISTS domain_owner_user_id UUID REFERENCES users(user_id),
    ADD COLUMN IF NOT EXISTS approver_user_id UUID REFERENCES users(user_id),
    ADD COLUMN IF NOT EXISTS organisational_unit VARCHAR(256),

    -- Section 5: Lifecycle
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS review_frequency_id UUID REFERENCES glossary_review_frequencies(frequency_id),
    ADD COLUMN IF NOT EXISTS next_review_date DATE,

    -- Section 6: Relationships (direct parent for common hierarchical case)
    ADD COLUMN IF NOT EXISTS parent_term_id UUID REFERENCES glossary_terms(term_id),

    -- Section 7: Usage & Context
    -- Note: business_context exists but semantically becomes "business_rules"
    ADD COLUMN IF NOT EXISTS used_in_reports TEXT,
    ADD COLUMN IF NOT EXISTS used_in_policies TEXT,
    ADD COLUMN IF NOT EXISTS regulatory_reporting_usage TEXT,

    -- Section 8: Quality
    ADD COLUMN IF NOT EXISTS is_cde BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS golden_source VARCHAR(256),
    ADD COLUMN IF NOT EXISTS confidence_level_id UUID REFERENCES glossary_confidence_levels(confidence_id),

    -- Section 9: Discoverability
    ADD COLUMN IF NOT EXISTS visibility_id UUID REFERENCES glossary_visibility_levels(visibility_id),
    ADD COLUMN IF NOT EXISTS language_id UUID REFERENCES glossary_languages(language_id),
    ADD COLUMN IF NOT EXISTS external_reference TEXT;

-- Add comment explaining semantic shift for business_context
COMMENT ON COLUMN glossary_terms.business_context IS 'Business rules and context for the term (formerly generic business context)';

-- ============================================================================
-- PART 6: JUNCTION TABLES FOR MANY-TO-MANY RELATIONSHIPS
-- ============================================================================

-- 6.1 Terms <-> Regulatory Tags
CREATE TABLE glossary_term_regulatory_tags (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id         UUID NOT NULL REFERENCES glossary_terms(term_id) ON DELETE CASCADE,
    tag_id          UUID NOT NULL REFERENCES glossary_regulatory_tags(tag_id) ON DELETE CASCADE,
    notes           TEXT,
    created_by      UUID REFERENCES users(user_id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(term_id, tag_id)
);

-- 6.2 Terms <-> Subject Areas
CREATE TABLE glossary_term_subject_areas (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id         UUID NOT NULL REFERENCES glossary_terms(term_id) ON DELETE CASCADE,
    subject_area_id UUID NOT NULL REFERENCES glossary_subject_areas(subject_area_id) ON DELETE CASCADE,
    is_primary      BOOLEAN NOT NULL DEFAULT FALSE,
    created_by      UUID REFERENCES users(user_id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(term_id, subject_area_id)
);

-- 6.3 Terms <-> Tags (keywords)
CREATE TABLE glossary_term_tags (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id         UUID NOT NULL REFERENCES glossary_terms(term_id) ON DELETE CASCADE,
    tag_id          UUID NOT NULL REFERENCES glossary_tags(tag_id) ON DELETE CASCADE,
    created_by      UUID REFERENCES users(user_id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(term_id, tag_id)
);

-- 6.4 Terms <-> Business Processes
CREATE TABLE glossary_term_processes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id         UUID NOT NULL REFERENCES glossary_terms(term_id) ON DELETE CASCADE,
    process_id      UUID NOT NULL REFERENCES business_processes(process_id) ON DELETE CASCADE,
    usage_context   TEXT,
    created_by      UUID REFERENCES users(user_id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(term_id, process_id)
);

-- ============================================================================
-- PART 7: TRIGGERS FOR AUTO-GENERATION
-- ============================================================================

-- 7.1 Auto-generate term_code on INSERT
CREATE OR REPLACE FUNCTION generate_glossary_term_code()
RETURNS TRIGGER AS $$
DECLARE
    v_domain_code VARCHAR(10);
    v_seq_num BIGINT;
BEGIN
    -- Get domain code
    SELECT domain_code INTO v_domain_code
    FROM glossary_domains
    WHERE domain_id = NEW.domain_id;

    -- If no domain, use 'GEN' for general
    IF v_domain_code IS NULL THEN
        v_domain_code := 'GEN';
    END IF;

    -- Get next sequence number
    v_seq_num := nextval('glossary_term_code_seq');

    -- Generate code: GLO-{DOMAIN}-{SEQ with leading zeros}
    NEW.term_code := 'GLO-' || v_domain_code || '-' || LPAD(v_seq_num::TEXT, 5, '0');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Only generate if term_code is NULL (allows manual override)
CREATE TRIGGER trg_generate_term_code
    BEFORE INSERT ON glossary_terms
    FOR EACH ROW
    WHEN (NEW.term_code IS NULL)
    EXECUTE FUNCTION generate_glossary_term_code();

-- 7.2 Calculate next_review_date when review_frequency changes
CREATE OR REPLACE FUNCTION calculate_next_review_date()
RETURNS TRIGGER AS $$
DECLARE
    v_months INT;
    v_base_date TIMESTAMPTZ;
BEGIN
    -- Only recalculate if review_frequency_id changed or approved_at changed
    IF (TG_OP = 'INSERT') OR
       (NEW.review_frequency_id IS DISTINCT FROM OLD.review_frequency_id) OR
       (NEW.approved_at IS DISTINCT FROM OLD.approved_at) THEN

        IF NEW.review_frequency_id IS NOT NULL THEN
            -- Get months interval
            SELECT months_interval INTO v_months
            FROM glossary_review_frequencies
            WHERE frequency_id = NEW.review_frequency_id;

            -- Base date is approved_at or updated_at
            v_base_date := COALESCE(NEW.approved_at, NEW.updated_at, CURRENT_TIMESTAMP);

            -- Calculate next review date
            NEW.next_review_date := (v_base_date + (v_months || ' months')::INTERVAL)::DATE;
        ELSE
            NEW.next_review_date := NULL;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_calculate_next_review
    BEFORE INSERT OR UPDATE ON glossary_terms
    FOR EACH ROW
    EXECUTE FUNCTION calculate_next_review_date();

-- 7.3 Set default language to English if not specified
CREATE OR REPLACE FUNCTION set_default_language()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.language_id IS NULL THEN
        SELECT language_id INTO NEW.language_id
        FROM glossary_languages
        WHERE is_default = TRUE
        LIMIT 1;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_set_default_language
    BEFORE INSERT ON glossary_terms
    FOR EACH ROW
    WHEN (NEW.language_id IS NULL)
    EXECUTE FUNCTION set_default_language();

-- 7.4 Set default visibility to ENTERPRISE_WIDE if not specified
CREATE OR REPLACE FUNCTION set_default_visibility()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.visibility_id IS NULL THEN
        SELECT visibility_id INTO NEW.visibility_id
        FROM glossary_visibility_levels
        WHERE visibility_code = 'ENTERPRISE_WIDE'
        LIMIT 1;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_set_default_visibility
    BEFORE INSERT ON glossary_terms
    FOR EACH ROW
    WHEN (NEW.visibility_id IS NULL)
    EXECUTE FUNCTION set_default_visibility();

-- ============================================================================
-- PART 8: INDEXES FOR QUERY PERFORMANCE
-- ============================================================================

-- New column indexes on glossary_terms
CREATE INDEX IF NOT EXISTS idx_glossary_terms_term_type
    ON glossary_terms(term_type_id) WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_classification
    ON glossary_terms(classification_id) WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_confidence
    ON glossary_terms(confidence_level_id) WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_visibility
    ON glossary_terms(visibility_id) WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_cde
    ON glossary_terms(is_cde) WHERE is_cde = TRUE AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_next_review
    ON glossary_terms(next_review_date) WHERE next_review_date IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_parent
    ON glossary_terms(parent_term_id) WHERE parent_term_id IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_language
    ON glossary_terms(language_id) WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_glossary_terms_approved
    ON glossary_terms(approved_at) WHERE approved_at IS NOT NULL AND deleted_at IS NULL;

-- Junction table indexes
CREATE INDEX IF NOT EXISTS idx_term_regulatory_tags_term
    ON glossary_term_regulatory_tags(term_id);

CREATE INDEX IF NOT EXISTS idx_term_regulatory_tags_tag
    ON glossary_term_regulatory_tags(tag_id);

CREATE INDEX IF NOT EXISTS idx_term_subject_areas_term
    ON glossary_term_subject_areas(term_id);

CREATE INDEX IF NOT EXISTS idx_term_subject_areas_area
    ON glossary_term_subject_areas(subject_area_id);

CREATE INDEX IF NOT EXISTS idx_term_tags_term
    ON glossary_term_tags(term_id);

CREATE INDEX IF NOT EXISTS idx_term_tags_tag
    ON glossary_term_tags(tag_id);

CREATE INDEX IF NOT EXISTS idx_term_processes_term
    ON glossary_term_processes(term_id);

CREATE INDEX IF NOT EXISTS idx_term_processes_process
    ON glossary_term_processes(process_id);

-- Lookup table indexes (for code lookups)
CREATE INDEX IF NOT EXISTS idx_glossary_tags_name
    ON glossary_tags(tag_name);

-- ============================================================================
-- PART 9: UPDATE SEARCH VECTOR TO INCLUDE NEW FIELDS
-- ============================================================================

-- Drop existing trigger first (if exists)
DROP TRIGGER IF EXISTS trg_glossary_terms_search_vector ON glossary_terms;

-- Update function to include new searchable fields
CREATE OR REPLACE FUNCTION update_glossary_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.term_name, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.abbreviation, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.term_code, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.definition, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.definition_notes, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.business_context, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.examples, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.counter_examples, '')), 'C') ||
        setweight(to_tsvector('english', COALESCE(NEW.formula, '')), 'D') ||
        setweight(to_tsvector('english', COALESCE(NEW.regulatory_reporting_usage, '')), 'D') ||
        setweight(to_tsvector('english', COALESCE(NEW.golden_source, '')), 'D') ||
        setweight(to_tsvector('english', COALESCE(NEW.external_reference, '')), 'D');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_glossary_terms_search_vector
    BEFORE INSERT OR UPDATE ON glossary_terms
    FOR EACH ROW
    EXECUTE FUNCTION update_glossary_search_vector();

-- ============================================================================
-- PART 10: META-DATA DOCUMENTATION
-- ============================================================================

-- Add comments documenting the 45-field structure
COMMENT ON TABLE glossary_term_types IS 'Lookup: Classification of term types (KPI, Business Concept, Regulatory, etc.)';
COMMENT ON TABLE glossary_review_frequencies IS 'Lookup: How often terms should be reviewed';
COMMENT ON TABLE glossary_confidence_levels IS 'Lookup: Confidence in term definition accuracy';
COMMENT ON TABLE glossary_visibility_levels IS 'Lookup: Who can view the term';
COMMENT ON TABLE glossary_units_of_measure IS 'Lookup: Units for KPIs and metrics';
COMMENT ON TABLE glossary_regulatory_tags IS 'Lookup: Regulatory frameworks (BCBS 239, IFRS, etc.)';
COMMENT ON TABLE glossary_subject_areas IS 'Lookup: Business areas where terms are used';
COMMENT ON TABLE glossary_tags IS 'User-defined keywords for term discoverability';
COMMENT ON TABLE glossary_languages IS 'Lookup: Supported languages for term definitions';
COMMENT ON TABLE glossary_term_regulatory_tags IS 'Junction: Links terms to regulatory frameworks';
COMMENT ON TABLE glossary_term_subject_areas IS 'Junction: Links terms to business subject areas';
COMMENT ON TABLE glossary_term_tags IS 'Junction: Links terms to user-defined tags';
COMMENT ON TABLE glossary_term_processes IS 'Junction: Links terms to business processes';

-- Column comments for new glossary_terms fields
COMMENT ON COLUMN glossary_terms.term_code IS 'System-generated unique identifier (GLO-{DOMAIN}-{SEQ})';
COMMENT ON COLUMN glossary_terms.definition_notes IS 'Additional notes clarifying the definition';
COMMENT ON COLUMN glossary_terms.counter_examples IS 'Examples of what this term does NOT mean';
COMMENT ON COLUMN glossary_terms.formula IS 'Calculation formula for KPIs/metrics';
COMMENT ON COLUMN glossary_terms.unit_of_measure_id IS 'FK: Unit of measure for quantitative terms';
COMMENT ON COLUMN glossary_terms.term_type_id IS 'FK: Type classification of the term';
COMMENT ON COLUMN glossary_terms.classification_id IS 'FK: Data sensitivity classification';
COMMENT ON COLUMN glossary_terms.domain_owner_user_id IS 'FK: User who owns the data domain';
COMMENT ON COLUMN glossary_terms.approver_user_id IS 'FK: User who approved this term version';
COMMENT ON COLUMN glossary_terms.organisational_unit IS 'Business unit that owns this term';
COMMENT ON COLUMN glossary_terms.approved_at IS 'Timestamp of last approval';
COMMENT ON COLUMN glossary_terms.review_frequency_id IS 'FK: How often this term should be reviewed';
COMMENT ON COLUMN glossary_terms.next_review_date IS 'System-calculated next review date';
COMMENT ON COLUMN glossary_terms.parent_term_id IS 'FK: Direct hierarchical parent term';
COMMENT ON COLUMN glossary_terms.used_in_reports IS 'Reports where this term is used';
COMMENT ON COLUMN glossary_terms.used_in_policies IS 'Policies referencing this term';
COMMENT ON COLUMN glossary_terms.regulatory_reporting_usage IS 'Context for regulatory reporting';
COMMENT ON COLUMN glossary_terms.is_cde IS 'Critical Data Element flag';
COMMENT ON COLUMN glossary_terms.golden_source IS 'Authoritative source system for this term''s data';
COMMENT ON COLUMN glossary_terms.confidence_level_id IS 'FK: Confidence in definition accuracy';
COMMENT ON COLUMN glossary_terms.visibility_id IS 'FK: Who can view this term';
COMMENT ON COLUMN glossary_terms.language_id IS 'FK: Language of the term definition';
COMMENT ON COLUMN glossary_terms.external_reference IS 'External documentation or standard reference';

-- ============================================================================
-- END OF MIGRATION
-- ============================================================================
