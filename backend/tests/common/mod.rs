//! Shared test utilities for integration tests.
//!
//! Provides database setup, authenticated test clients, and fixture builders.

use metadata_tool::auth;
use uuid::Uuid;

/// Default test JWT secret (must match .env.example or test config).
pub const TEST_JWT_SECRET: &str = "test-secret-key-for-development-only-change-in-prod";

/// Default test admin user credentials (matches migration 013 seed).
pub const ADMIN_EMAIL: &str = "admin@example.com";
pub const ADMIN_PASSWORD: &str = "metadata123";

/// Create a JWT token for testing with the given user details.
pub fn create_test_token(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    roles: Vec<String>,
) -> String {
    auth::create_token(user_id, email, display_name, roles, TEST_JWT_SECRET, 8)
        .expect("test token creation should succeed")
}

/// Create a JWT token for the seeded admin user.
pub fn admin_token() -> String {
    create_test_token(
        Uuid::new_v4(),
        ADMIN_EMAIL,
        "System Administrator",
        vec![
            "ADMIN".into(),
            "DATA_STEWARD".into(),
            "DATA_PRODUCER".into(),
            "DATA_CONSUMER".into(),
            "DATA_OWNER".into(),
        ],
    )
}

/// Create a JWT token for a data steward user.
pub fn steward_token() -> String {
    create_test_token(
        Uuid::new_v4(),
        "steward@example.com",
        "Dana Steward",
        vec!["DATA_STEWARD".into()],
    )
}

/// Create a JWT token for a data consumer (read-only) user.
pub fn consumer_token() -> String {
    create_test_token(
        Uuid::new_v4(),
        "consumer@example.com",
        "Chris Consumer",
        vec!["DATA_CONSUMER".into()],
    )
}

/// Create an Authorization header value for Bearer token auth.
pub fn bearer_header(token: &str) -> String {
    format!("Bearer {token}")
}
