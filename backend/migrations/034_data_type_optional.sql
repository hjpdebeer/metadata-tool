-- Migration 034: Make data_type optional on data_elements
--
-- Data type is now AI-suggestible. Users create elements with just
-- name + description, and AI suggests the data type for review.

ALTER TABLE data_elements ALTER COLUMN data_type DROP NOT NULL;

COMMENT ON COLUMN data_elements.data_type IS 'Physical data type (VARCHAR, INTEGER, etc.) — can be AI-suggested';
