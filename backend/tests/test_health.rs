mod common;

#[tokio::test]
async fn health_check_returns_ok() {
    let ctx = common::setup().await;
    let response = ctx.server.get("/api/v1/health").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["database"], "connected");
}
