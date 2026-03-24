mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn dev_login_with_valid_credentials() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .post("/api/v1/auth/dev-login")
        .json(&serde_json::json!({
            "email": "testadmin@test.local",
            "password": "Password"
        }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["access_token"].is_string());
}

#[tokio::test]
async fn dev_login_with_wrong_password() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .post("/api/v1/auth/dev-login")
        .json(&serde_json::json!({
            "email": "testadmin@test.local",
            "password": "WrongPassword"
        }))
        .await;
    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_returns_user_info() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/auth/me")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "testadmin@test.local");
}

#[tokio::test]
async fn me_without_token_returns_401() {
    let ctx = common::setup().await;
    let response = ctx.server.get("/api/v1/auth/me").await;
    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_profile_returns_full_profile() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/auth/me/profile")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "testadmin@test.local");
    assert!(body["roles"].is_array());
}
