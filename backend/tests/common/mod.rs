//! Shared test utilities for integration tests.
//!
//! Provides database connection, test server, authenticated test users, and
//! helper functions for building integration test scenarios against a real
//! PostgreSQL database.

use axum_test::TestServer;
use metadata_tool::auth::create_token;
use metadata_tool::config::AppConfig;
use metadata_tool::db::AppState;
use sqlx::PgPool;
use uuid::Uuid;

/// Everything an integration test needs: a running test server, database pool,
/// and pre-authenticated tokens for users with different roles.
#[allow(dead_code)]
pub struct TestContext {
    pub server: TestServer,
    pub pool: PgPool,
    pub admin_id: Uuid,
    pub admin_token: String,
    pub steward_id: Uuid,
    pub steward_token: String,
    pub consumer_id: Uuid,
    pub consumer_token: String,
}

/// Set up a test context with a real database, migrations, seeded users, and
/// a test server running the full application router.
///
/// Attempts to auto-create the test database if it does not exist.
pub async fn setup() -> TestContext {
    let config = AppConfig::test_default();

    // Try connecting to test DB; if it doesn't exist, create it.
    // Multiple test binaries may race to create the DB — handle gracefully.
    let pool = match PgPool::connect(&config.database_url).await {
        Ok(pool) => pool,
        Err(_) => {
            let base_url = config
                .database_url
                .rsplitn(2, '/')
                .last()
                .expect("DATABASE_URL must contain a '/'");
            if let Ok(admin_pool) = PgPool::connect(&format!("{base_url}/postgres")).await {
                // Ignore errors (DB may already exist from parallel test)
                let _ = sqlx::query("CREATE DATABASE metadata_tool_test")
                    .execute(&admin_pool)
                    .await;
                admin_pool.close().await;
            }
            // Retry connection (may need a brief delay for DB to become ready)
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            PgPool::connect(&config.database_url)
                .await
                .expect("Failed to connect to test database")
        }
    };

    // Run migrations (idempotent)
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations on test database");

    // Seed test users with distinct roles
    let admin_id = seed_user(
        &pool,
        "testadmin",
        "testadmin@test.local",
        "Test Admin",
        "Password",
        &["ADMIN", "DATA_STEWARD", "DATA_OWNER"],
    )
    .await;
    let steward_id = seed_user(
        &pool,
        "teststeward",
        "teststeward@test.local",
        "Test Steward",
        "Password",
        &["DATA_STEWARD"],
    )
    .await;
    let consumer_id = seed_user(
        &pool,
        "testconsumer",
        "testconsumer@test.local",
        "Test Consumer",
        "Password",
        &["DATA_CONSUMER"],
    )
    .await;

    // Create JWT tokens for each test user
    let jwt_secret = &config.jwt_secret;
    let admin_token = create_token(
        admin_id,
        "testadmin@test.local",
        "Test Admin",
        vec!["ADMIN".into(), "DATA_STEWARD".into(), "DATA_OWNER".into()],
        jwt_secret,
        8,
    )
    .expect("Failed to create admin test token");

    let steward_token = create_token(
        steward_id,
        "teststeward@test.local",
        "Test Steward",
        vec!["DATA_STEWARD".into()],
        jwt_secret,
        8,
    )
    .expect("Failed to create steward test token");

    let consumer_token = create_token(
        consumer_id,
        "testconsumer@test.local",
        "Test Consumer",
        vec!["DATA_CONSUMER".into()],
        jwt_secret,
        8,
    )
    .expect("Failed to create consumer test token");

    // Build the app state and test server
    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
        settings_cache: None,
    };

    let app = metadata_tool::build_router(state);
    let server = TestServer::new(app);

    TestContext {
        server,
        pool,
        admin_id,
        admin_token,
        steward_id,
        steward_token,
        consumer_id,
        consumer_token,
    }
}

/// Upsert a user with the given credentials and role assignments.
async fn seed_user(
    pool: &PgPool,
    username: &str,
    email: &str,
    display_name: &str,
    password: &str,
    roles: &[&str],
) -> Uuid {
    // Upsert user
    let user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (username, email, display_name, is_active, roles_reviewed)
        VALUES ($1, $2, $3, TRUE, TRUE)
        ON CONFLICT (email) DO UPDATE SET is_active = TRUE
        RETURNING user_id
        "#,
    )
    .bind(username)
    .bind(email)
    .bind(display_name)
    .fetch_one(pool)
    .await
    .expect("Failed to seed user");

    // Set password (uses pgcrypto crypt/gen_salt)
    sqlx::query("UPDATE users SET password_hash = crypt($1, gen_salt('bf')) WHERE user_id = $2")
        .bind(password)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("Failed to set password");

    // Assign roles
    for role in roles {
        sqlx::query(
            r#"
            INSERT INTO user_roles (user_id, role_id, granted_by)
            SELECT $1, role_id, $1 FROM roles WHERE role_code = $2
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await
        .expect("Failed to assign role");
    }

    user_id
}
