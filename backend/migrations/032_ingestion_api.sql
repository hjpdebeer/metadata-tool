-- Migration 032: Ingestion API Support
--
-- Adds last_seen_at for stale detection, api_keys for service account auth,
-- and ACCEPTED status auto-assignment for trusted ingestion sources.

-- =========================================================================
-- 1. LAST_SEEN_AT FOR STALE DETECTION
-- =========================================================================

ALTER TABLE source_systems
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;

ALTER TABLE technical_schemas
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;

ALTER TABLE technical_tables
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;

ALTER TABLE technical_columns
    ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;

COMMENT ON COLUMN source_systems.last_seen_at IS 'Last time this system was reported by an ingestion crawl';
COMMENT ON COLUMN technical_schemas.last_seen_at IS 'Last time this schema was reported by an ingestion crawl';
COMMENT ON COLUMN technical_tables.last_seen_at IS 'Last time this table was reported by an ingestion crawl';
COMMENT ON COLUMN technical_columns.last_seen_at IS 'Last time this column was reported by an ingestion crawl';

-- =========================================================================
-- 2. API KEYS FOR SERVICE ACCOUNT AUTHENTICATION
-- =========================================================================

CREATE TABLE IF NOT EXISTS api_keys (
    key_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_name        VARCHAR(128) NOT NULL,
    key_hash        VARCHAR(256) NOT NULL,
    key_prefix      VARCHAR(8) NOT NULL,
    scopes          TEXT[] NOT NULL DEFAULT '{}',
    created_by      UUID NOT NULL REFERENCES users(user_id),
    expires_at      TIMESTAMPTZ,
    last_used_at    TIMESTAMPTZ,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix) WHERE is_active = TRUE;

COMMENT ON TABLE api_keys IS 'Service account API keys for external tool integration';
COMMENT ON COLUMN api_keys.key_hash IS 'bcrypt hash of the full API key';
COMMENT ON COLUMN api_keys.key_prefix IS 'First 8 chars of the key for identification (e.g., mdt_k3x9)';
COMMENT ON COLUMN api_keys.scopes IS 'Array of permission scopes: ingest:technical, ingest:elements, read:all, read:technical';

-- =========================================================================
-- 3. INGESTION LOG
-- =========================================================================

CREATE TABLE IF NOT EXISTS ingestion_log (
    log_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id      UUID REFERENCES api_keys(key_id),
    ingestion_type  VARCHAR(50) NOT NULL,
    source_system_code VARCHAR(64),
    summary         JSONB NOT NULL DEFAULT '{}',
    errors          JSONB NOT NULL DEFAULT '[]',
    warnings        JSONB NOT NULL DEFAULT '[]',
    duration_ms     INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ingestion_log_key ON ingestion_log(api_key_id);
CREATE INDEX IF NOT EXISTS idx_ingestion_log_created ON ingestion_log(created_at DESC);

COMMENT ON TABLE ingestion_log IS 'Audit trail of all metadata ingestion operations';
