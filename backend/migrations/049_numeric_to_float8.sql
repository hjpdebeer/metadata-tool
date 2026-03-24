-- Change NUMERIC columns to DOUBLE PRECISION to eliminate ::FLOAT8 casts.
-- SQLx maps DOUBLE PRECISION directly to Rust f64.

ALTER TABLE quality_rules
    ALTER COLUMN threshold_percentage TYPE DOUBLE PRECISION;

ALTER TABLE quality_assessments
    ALTER COLUMN score_percentage TYPE DOUBLE PRECISION;

ALTER TABLE quality_scores
    ALTER COLUMN overall_score TYPE DOUBLE PRECISION;

ALTER TABLE quality_scores
    ALTER COLUMN pass_rate TYPE DOUBLE PRECISION;
