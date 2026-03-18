use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::PgPool;

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
