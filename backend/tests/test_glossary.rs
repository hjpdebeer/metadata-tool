mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn create_and_get_term() {
    let ctx = common::setup().await;

    // Get a domain_id first
    let domains_response = ctx
        .server
        .get("/api/v1/glossary/domains")
        .authorization_bearer(&ctx.admin_token)
        .await;
    domains_response.assert_status_ok();
    let domains: Vec<serde_json::Value> = domains_response.json();
    assert!(!domains.is_empty(), "Should have seeded domains");
    let domain_id = domains[0]["domain_id"].as_str().unwrap();

    // Create term with a unique name
    let unique_name = format!(
        "Test Term {}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );
    let create_response = ctx
        .server
        .post("/api/v1/glossary/terms")
        .authorization_bearer(&ctx.admin_token)
        .json(&serde_json::json!({
            "term_name": unique_name,
            "definition": "A test glossary term for integration testing.",
            "domain_id": domain_id
        }))
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let term_id = created["term_id"].as_str().unwrap();

    // Get term by ID
    let get_response = ctx
        .server
        .get(&format!("/api/v1/glossary/terms/{term_id}"))
        .authorization_bearer(&ctx.admin_token)
        .await;
    get_response.assert_status_ok();
    let term: serde_json::Value = get_response.json();
    assert_eq!(term["term_name"], unique_name);
}

#[tokio::test]
async fn list_terms_with_pagination() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/glossary/terms?page=1&page_size=10")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["data"].is_array());
    assert!(body["total_count"].is_number());
    assert_eq!(body["page"], 1);
}

#[tokio::test]
async fn create_term_without_auth_returns_401() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .post("/api/v1/glossary/terms")
        .json(&serde_json::json!({
            "term_name": "Unauthorized Term",
            "definition": "Should fail"
        }))
        .await;
    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_domains() {
    let ctx = common::setup().await;
    let response = ctx
        .server
        .get("/api/v1/glossary/domains")
        .authorization_bearer(&ctx.admin_token)
        .await;
    response.assert_status_ok();
    let domains: Vec<serde_json::Value> = response.json();
    assert!(!domains.is_empty(), "Should have seeded domains");
}
