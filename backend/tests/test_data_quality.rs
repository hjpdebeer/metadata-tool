mod common;

#[tokio::test]
async fn list_dimensions() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/data-quality/dimensions")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let dims: Vec<serde_json::Value> = response.json();
    assert_eq!(dims.len(), 6, "Should have 6 DAMA dimensions");
}

#[tokio::test]
async fn list_rule_types() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/data-quality/rule-types")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let types: Vec<serde_json::Value> = response.json();
    assert!(!types.is_empty());
}

#[tokio::test]
async fn list_rules_with_pagination() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/data-quality/rules?page=1&page_size=10")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn recent_assessments() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/data-quality/assessments/recent?limit=5")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
}
