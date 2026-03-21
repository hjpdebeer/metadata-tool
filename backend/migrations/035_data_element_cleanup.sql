-- Migration 035: Data Element Cleanup
--
-- 1. Remove sensitivity_level (duplicate of classification_id)
-- 2. Default review_frequency_id to ANNUAL
-- 3. Data type is now a clean enum value — precision/scale are separate fields

-- 1. Remove sensitivity_level (redundant with classification_id)
ALTER TABLE data_elements DROP COLUMN IF EXISTS sensitivity_level;

-- 2. Default review_frequency_id to ANNUAL for new elements
-- (The create handler will look up the ANNUAL frequency_id, same as glossary)

-- 3. No DB change needed for data_type — it's already VARCHAR.
--    The frontend will enforce clean enum values via dropdown.
--    AI will suggest clean type names (VARCHAR, DECIMAL, INTEGER, etc.)
