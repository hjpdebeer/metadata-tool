-- Migration 025: Change term_code uniqueness to (term_code, version_number)
--
-- The term_code identifies the business term concept.
-- The version_number identifies which version of that concept.
-- Together they form the natural composite key for versioned terms.

-- Drop the old single-column unique constraint
ALTER TABLE glossary_terms DROP CONSTRAINT IF EXISTS glossary_terms_term_code_key;

-- Add composite unique constraint (term_code + version_number, soft-delete aware)
CREATE UNIQUE INDEX idx_glossary_terms_code_version
    ON glossary_terms (term_code, version_number)
    WHERE deleted_at IS NULL;

COMMENT ON INDEX idx_glossary_terms_code_version IS 'Term code + version number must be unique (allows same code across versions)';
