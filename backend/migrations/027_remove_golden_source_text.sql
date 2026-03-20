-- Migration 027: Remove golden_source text field
--
-- The golden_source_app_id FK to the Application Register replaces the
-- free-text golden_source field. The FK provides proper governance
-- traceability — the text field is redundant and less governed.

ALTER TABLE glossary_terms DROP COLUMN IF EXISTS golden_source;
