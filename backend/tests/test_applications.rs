mod common;

#[tokio::test]
async fn list_applications() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/applications?page=1&page_size=10")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn list_classifications() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/applications/classifications")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
}

#[tokio::test]
async fn list_dr_tiers() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/applications/dr-tiers")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
}
