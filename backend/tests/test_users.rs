mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn admin_can_list_users() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/users?page=1&page_size=10")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn consumer_cannot_list_users() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/users?page=1&page_size=10")
        .authorization_bearer(&ctx.consumer_token)
        .await;
    response.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn list_roles_returns_system_roles() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/roles")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let roles: Vec<serde_json::Value> = response.json();
    let role_codes: Vec<&str> = roles
        .iter()
        .map(|r| r["role_code"].as_str().unwrap())
        .collect();
    assert!(role_codes.contains(&"ADMIN"));
    assert!(role_codes.contains(&"DATA_STEWARD"));
    assert!(role_codes.contains(&"DATA_CONSUMER"));
}

#[tokio::test]
async fn lookup_users_available_to_all() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/users/lookup")
        .authorization_bearer(&ctx.consumer_token)
        .await;
    response.assert_status_ok();
}
