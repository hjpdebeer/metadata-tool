-- ============================================================================
-- Migration: 020_system_settings.sql
-- Purpose: System settings table for admin-configurable options
-- ============================================================================

CREATE TABLE system_settings (
    setting_key     VARCHAR(128) PRIMARY KEY,
    setting_value   TEXT NOT NULL,
    is_encrypted    BOOLEAN NOT NULL DEFAULT FALSE,
    category        VARCHAR(64) NOT NULL,
    display_name    VARCHAR(256) NOT NULL,
    description     TEXT,
    validation_regex VARCHAR(512),
    updated_by      UUID REFERENCES users(user_id),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_system_settings_category ON system_settings(category);

-- Seed with default structure (values will be empty until set by admin)
INSERT INTO system_settings (setting_key, setting_value, is_encrypted, category, display_name, description, validation_regex)
VALUES
    -- AI
    ('anthropic_api_key', '', TRUE, 'AI', 'Anthropic API Key', 'API key for Claude AI enrichment (primary provider)', '^sk-ant-'),
    ('anthropic_model', 'claude-3-5-sonnet-latest', FALSE, 'AI', 'Anthropic Model', 'Model to use for AI enrichment', NULL),
    ('openai_api_key', '', TRUE, 'AI', 'OpenAI API Key', 'API key for OpenAI (fallback provider)', '^sk-'),
    ('openai_model', 'gpt-4o', FALSE, 'AI', 'OpenAI Model', 'Model to use for AI enrichment fallback', NULL),

    -- Auth
    ('jwt_secret', '', TRUE, 'Auth', 'JWT Secret', 'Secret key for signing JWT tokens (min 32 chars)', '.{32,}'),
    ('entra_tenant_id', '', FALSE, 'Auth', 'Entra Tenant ID', 'Microsoft Entra ID tenant', '^[0-9a-f-]{36}$'),
    ('entra_client_id', '', FALSE, 'Auth', 'Entra Client ID', 'OAuth application client ID', '^[0-9a-f-]{36}$'),
    ('entra_client_secret', '', TRUE, 'Auth', 'Entra Client Secret', 'OAuth application client secret', NULL),
    ('entra_redirect_uri', '', FALSE, 'Auth', 'Entra Redirect URI', 'OAuth redirect URI after login', '^https?://'),

    -- Email
    ('graph_tenant_id', '', FALSE, 'Email', 'Graph Tenant ID', 'Microsoft Graph API tenant', '^[0-9a-f-]{36}$'),
    ('graph_client_id', '', FALSE, 'Email', 'Graph Client ID', 'Microsoft Graph application ID', '^[0-9a-f-]{36}$'),
    ('graph_client_secret', '', TRUE, 'Email', 'Graph Client Secret', 'Microsoft Graph client secret', NULL),
    ('notification_sender_email', '', FALSE, 'Email', 'Notification Sender', 'Email address for sending notifications', '^[^@]+@[^@]+$'),

    -- App
    ('frontend_url', 'http://localhost:5173', FALSE, 'App', 'Frontend URL', 'Base URL for the frontend application', '^https?://'),
    ('default_review_frequency', 'ANNUAL', FALSE, 'App', 'Default Review Frequency', 'Default review frequency for new terms', NULL);

COMMENT ON TABLE system_settings IS 'Admin-configurable system settings with optional encryption for sensitive values';
