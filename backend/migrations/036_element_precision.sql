-- Migration 036: Add precision fields to data_elements
--
-- Logical data type specification at the data element level.
-- max_length for VARCHAR/CHAR, numeric_precision/numeric_scale for DECIMAL/NUMERIC.
-- AI suggests these alongside data_type.

ALTER TABLE data_elements
    ADD COLUMN IF NOT EXISTS max_length INTEGER,
    ADD COLUMN IF NOT EXISTS numeric_precision INTEGER,
    ADD COLUMN IF NOT EXISTS numeric_scale INTEGER;

COMMENT ON COLUMN data_elements.max_length IS 'Maximum length for VARCHAR/CHAR types';
COMMENT ON COLUMN data_elements.numeric_precision IS 'Total number of digits for DECIMAL/NUMERIC types';
COMMENT ON COLUMN data_elements.numeric_scale IS 'Number of decimal places for DECIMAL/NUMERIC types';
