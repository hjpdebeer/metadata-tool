use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;

/// Shared application state available to all route handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(pool: PgPool, config: AppConfig) -> Self {
        Self { pool, config }
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
/// Parses the URL and disables SSL for local development (Docker PostgreSQL
/// does not have SSL configured). In production, configure SSL via the
/// connection string or environment.
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let options: PgConnectOptions = database_url
        .parse::<PgConnectOptions>()?
        .ssl_mode(PgSslMode::Prefer);

    PgPool::connect_with(options).await
}
