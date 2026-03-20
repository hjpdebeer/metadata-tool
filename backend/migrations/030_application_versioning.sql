-- Migration 030: Add version-based amendment support to applications
--
-- Mirrors the glossary term versioning pattern (migration 025):
-- - version_number tracks the version of the application record
-- - is_current_version indicates which version is the active one
-- - previous_version_id links an amendment to its original
-- - Composite unique constraint allows same application_code across versions

-- 1. Add versioning columns
ALTER TABLE applications
    ADD COLUMN IF NOT EXISTS version_number INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS is_current_version BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS previous_version_id UUID REFERENCES applications(application_id);

-- 2. Drop the old single-column unique constraint on application_code
ALTER TABLE applications DROP CONSTRAINT IF EXISTS applications_application_code_key;

-- Also drop any unique index that may exist on application_code alone
DROP INDEX IF EXISTS applications_application_code_key;

-- 3. Add composite unique constraint (application_code + version_number, soft-delete aware)
CREATE UNIQUE INDEX IF NOT EXISTS idx_applications_code_version
    ON applications (application_code, version_number)
    WHERE deleted_at IS NULL;

COMMENT ON COLUMN applications.version_number IS 'Version number of this application record (starts at 1, incremented for amendments)';
COMMENT ON COLUMN applications.is_current_version IS 'TRUE for the active version of the application, FALSE for superseded or in-progress amendments';
COMMENT ON COLUMN applications.previous_version_id IS 'References the previous version this amendment is based on (NULL for original entries)';
COMMENT ON INDEX idx_applications_code_version IS 'Application code + version number must be unique (allows same code across versions)';
