-- Migration 023: Rename is_cde → is_cbt on glossary_terms
--
-- At the glossary level, the correct term is Critical Business Term (CBT).
-- CDE (Critical Data Element) applies at the Data Dictionary level.
-- CBT designation propagates to CDE on linked data elements (ADR-0005).

-- 1. Rename the column
ALTER TABLE glossary_terms RENAME COLUMN is_cde TO is_cbt;

-- 2. Drop old index and create new one
DROP INDEX IF EXISTS idx_glossary_terms_cde;
CREATE INDEX idx_glossary_terms_cbt ON glossary_terms(is_cbt) WHERE is_cbt = TRUE AND deleted_at IS NULL;

-- 3. Update column comment
COMMENT ON COLUMN glossary_terms.is_cbt IS 'Critical Business Term flag — propagates CDE to linked data elements';
