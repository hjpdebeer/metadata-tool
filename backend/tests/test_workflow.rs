mod common;

#[tokio::test]
async fn list_pending_tasks() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/workflow/tasks/pending")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let tasks: serde_json::Value = response.json();
    assert!(tasks.is_array());
}
