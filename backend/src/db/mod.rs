use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::settings::SettingsCache;

/// Shared application state available to all route handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
    pub settings_cache: Option<SettingsCache>,
}

impl AppState {
    pub fn new(pool: PgPool, config: AppConfig) -> Self {
        // SEC-019: In production (Entra SSO configured), require a separate encryption key.
        // In development (placeholder tenant ID), allow fallback to jwt_secret for convenience.
        let entra_configured = !config.entra.tenant_id.is_empty()
            && config.entra.tenant_id != "your-tenant-id"
            && uuid::Uuid::parse_str(&config.entra.tenant_id).is_ok();

        let encryption_secret = match std::env::var("SETTINGS_ENCRYPTION_KEY") {
            Ok(key) if !key.is_empty() => key,
            _ if entra_configured => {
                tracing::error!(
                    "SETTINGS_ENCRYPTION_KEY is required in production (Entra SSO is configured). \
                     Set a unique 32+ character key, different from JWT_SECRET."
                );
                panic!(
                    "SETTINGS_ENCRYPTION_KEY must be set when Entra SSO is configured — \
                     refusing to reuse JWT_SECRET as encryption key in production"
                );
            }
            _ => {
                tracing::warn!(
                    "SETTINGS_ENCRYPTION_KEY not set — falling back to JWT_SECRET (dev mode only)"
                );
                config.jwt_secret.clone()
            }
        };

        let settings_cache = Some(SettingsCache::new(encryption_secret));
        Self {
            pool,
            config,
            settings_cache,
        }
    }
}

// ---------------------------------------------------------------------------
// Lookup resolution (ADR-0006 Pattern 2)
// ---------------------------------------------------------------------------

/// Resolve a lookup field value. Accepts either a UUID string (from UI dropdown)
/// or a display name (from AI suggestion). Tries UUID parse first, falls back
/// to ILIKE name match via the provided lookup query.
///
/// ADR-0006 Pattern 2: Unified write path for UI and AI inputs.
pub async fn resolve_lookup(
    pool: &PgPool,
    value: &str,
    lookup_query: &str,
) -> Option<Uuid> {
    // Try parsing as UUID first (UI dropdowns send IDs)
    if let Ok(id) = Uuid::parse_str(value) {
        return Some(id);
    }
    // Fall back to display name resolution (AI suggestions send names)
    sqlx::query_scalar::<_, Uuid>(lookup_query)
        .bind(value)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

// ---------------------------------------------------------------------------
// Connection pool
// ---------------------------------------------------------------------------

/// Create a connection pool from a DATABASE_URL string.
///
/// Configures health checks, timeouts, and automatic reconnection so the
/// application recovers gracefully from database restarts without requiring
/// a backend restart.
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let options: PgConnectOptions = database_url
        .parse::<PgConnectOptions>()?
        .ssl_mode(PgSslMode::Prefer);

    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        // Test connections before handing them out — detects stale connections
        // after a database restart without requiring an application restart.
        .test_before_acquire(true)
        .connect_with(options)
        .await
}
