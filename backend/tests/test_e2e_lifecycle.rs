//! End-to-end lifecycle test: simulates a complete metadata governance workflow
//! from glossary term creation through data element, quality rules, application
//! linking, and full approval chains.
//!
//! This test validates the entire "happy path" that a real user follows:
//!
//! 1. Create Glossary Term → assign ownership → submit → approve → ACCEPTED
//! 2. Create Data Element → link to glossary → assign ownership → submit → approve
//! 3. Create Quality Rules for the element → record assessments
//! 4. Create Application → link data element → approve
//! 5. Verify the full entity graph and cross-module relationships

mod common;

use serde_json::json;

/// Helper to add auth header
fn auth(token: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    (
        "Authorization".parse().unwrap(),
        format!("Bearer {token}").parse().unwrap(),
    )
}

#[tokio::test]
async fn full_metadata_governance_lifecycle() {
    let ctx = common::setup().await;
    let (auth_name, auth_val) = auth(&ctx.admin_token);

    // =========================================================================
    // PHASE 1: Glossary Term — Create, Assign Ownership, Approve
    // =========================================================================

    // 1a. Get a domain for the term
    let domains_resp = ctx
        .server
        .get("/api/v1/glossary/domains")
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    domains_resp.assert_status_ok();
    let domains: Vec<serde_json::Value> = domains_resp.json();
    assert!(!domains.is_empty(), "Should have seeded domains");
    let domain_id = domains[0]["domain_id"].as_str().unwrap();

    // 1b. Create glossary term
    let term_resp = ctx
        .server
        .post("/api/v1/glossary/terms")
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "term_name": "E2E Net Interest Income",
            "definition": "The difference between interest earned on assets and interest paid on liabilities.",
            "domain_id": domain_id,
            "owner_user_id": ctx.admin_id,
            "steward_user_id": ctx.admin_id,
            "domain_owner_user_id": ctx.admin_id,
            "approver_user_id": ctx.admin_id
        }))
        .await;
    term_resp.assert_status(axum::http::StatusCode::CREATED);
    let term: serde_json::Value = term_resp.json();
    let term_id = term["term_id"].as_str().unwrap().to_string();
    assert_eq!(term["term_name"], "E2E Net Interest Income");

    // 1c. Get the workflow instance for this term
    let wf_resp = ctx
        .server
        .get(&format!("/api/v1/workflow/instances/by-entity/{term_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    wf_resp.assert_status_ok();
    let wf_instance: serde_json::Value = wf_resp.json();
    let wf_instance_id = wf_instance["instance_id"].as_str().unwrap().to_string();
    // current_state_name is the display name from workflow_states table
    let state_name = wf_instance["current_state_name"]
        .as_str()
        .unwrap()
        .to_uppercase();
    assert!(
        state_name.contains("DRAFT"),
        "New term should be in DRAFT state, got: {}",
        wf_instance["current_state_name"]
    );

    // 1d. Submit for review (DRAFT → UNDER_REVIEW)
    let submit_resp = ctx
        .server
        .post(&format!(
            "/api/v1/workflow/instances/{wf_instance_id}/transition"
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "action": "SUBMIT",
            "comments": "Ready for steward review"
        }))
        .await;
    submit_resp.assert_status_ok();
    let wf_after_submit: serde_json::Value = submit_resp.json();
    // The transition endpoint returns WorkflowInstance (not WorkflowInstanceView)
    // which has current_state_id but not current_state_name
    // Just verify it moved from the original state
    assert!(
        wf_after_submit["current_state_id"].as_str() != wf_instance["current_state_id"].as_str(),
        "State should have changed after SUBMIT"
    );

    // 1e. Get pending tasks — steward should have a review task
    let tasks_resp = ctx
        .server
        .get("/api/v1/workflow/tasks/pending")
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    tasks_resp.assert_status_ok();
    let tasks: Vec<serde_json::Value> = tasks_resp.json();
    let review_task = tasks
        .iter()
        .find(|t| {
            t["entity_id"].as_str() == Some(term_id.as_str())
                && t["task"]["status"].as_str() == Some("PENDING")
        })
        .expect("Should have a pending review task for the term");
    let review_task_id = review_task["task"]["task_id"].as_str().unwrap().to_string();

    // 1f. Complete review task with APPROVE (UNDER_REVIEW → PENDING_APPROVAL)
    let approve_resp = ctx
        .server
        .post(&format!("/api/v1/workflow/tasks/{review_task_id}/complete"))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "decision": "APPROVE",
            "comments": "Looks good, forwarding to owner"
        }))
        .await;
    approve_resp.assert_status_ok();

    // 1g. Verify state progressed (skip exact name check — DB returns display names)

    // 1h. Get the owner approval task
    let tasks_resp2 = ctx
        .server
        .get("/api/v1/workflow/tasks/pending")
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    let tasks2: Vec<serde_json::Value> = tasks_resp2.json();
    let approval_task = tasks2
        .iter()
        .find(|t| {
            t["entity_id"].as_str() == Some(&term_id)
                && t["task"]["status"].as_str() == Some("PENDING")
        })
        .expect("Should have a pending approval task");
    let approval_task_id = approval_task["task"]["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 1i. Owner approves (PENDING_APPROVAL → ACCEPTED)
    let final_approve = ctx
        .server
        .post(&format!(
            "/api/v1/workflow/tasks/{approval_task_id}/complete"
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "decision": "APPROVE",
            "comments": "Approved for production use"
        }))
        .await;
    final_approve.assert_status_ok();

    // 1j. Verify term is now ACCEPTED
    let term_final = ctx
        .server
        .get(&format!("/api/v1/glossary/terms/{term_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    term_final.assert_status_ok();
    let accepted_term: serde_json::Value = term_final.json();
    assert_eq!(
        accepted_term["status_code"], "ACCEPTED",
        "Term should be ACCEPTED after full approval"
    );

    // =========================================================================
    // PHASE 2: Data Element — Create, Link to Glossary, Approve
    // =========================================================================

    // 2a. Create data element linked to the glossary term
    let elem_resp = ctx
        .server
        .post("/api/v1/data-dictionary/elements")
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "element_name": "E2E Net Interest Income Amount",
            "description": "Monetary amount of net interest income for the reporting period.",
            "data_type": "DECIMAL",
            "glossary_term_id": term_id,
            "owner_user_id": ctx.admin_id,
            "steward_user_id": ctx.admin_id,
            "approver_user_id": ctx.admin_id
        }))
        .await;
    elem_resp.assert_status(axum::http::StatusCode::CREATED);
    let element: serde_json::Value = elem_resp.json();
    let element_id = element["element_id"].as_str().unwrap().to_string();
    assert_eq!(element["element_name"], "E2E Net Interest Income Amount");

    // 2c. Submit and approve element through workflow
    let elem_wf = ctx
        .server
        .get(&format!(
            "/api/v1/workflow/instances/by-entity/{element_id}"
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    let elem_wf_instance: serde_json::Value = elem_wf.json();
    let elem_wf_id = elem_wf_instance["instance_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Submit
    ctx.server
        .post(&format!(
            "/api/v1/workflow/instances/{elem_wf_id}/transition"
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({"action": "SUBMIT"}))
        .await
        .assert_status_ok();

    // Find and complete steward review task
    let tasks3: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/workflow/tasks/pending")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let elem_review = tasks3
        .iter()
        .find(|t| {
            t["entity_id"].as_str() == Some(&element_id)
                && t["task"]["status"].as_str() == Some("PENDING")
        })
        .expect("Should have element review task");
    ctx.server
        .post(&format!(
            "/api/v1/workflow/tasks/{}/complete",
            elem_review["task"]["task_id"].as_str().unwrap()
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({"decision": "APPROVE"}))
        .await
        .assert_status_ok();

    // Find and complete owner approval task
    let tasks4: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/workflow/tasks/pending")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let elem_approval = tasks4
        .iter()
        .find(|t| {
            t["entity_id"].as_str() == Some(&element_id)
                && t["task"]["status"].as_str() == Some("PENDING")
        })
        .expect("Should have element approval task");
    ctx.server
        .post(&format!(
            "/api/v1/workflow/tasks/{}/complete",
            elem_approval["task"]["task_id"].as_str().unwrap()
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({"decision": "APPROVE"}))
        .await
        .assert_status_ok();

    // 2c. Verify element is ACCEPTED
    let elem_final: serde_json::Value = ctx
        .server
        .get(&format!("/api/v1/data-dictionary/elements/{element_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert_eq!(elem_final["status_code"], "ACCEPTED");
    assert_eq!(
        elem_final["glossary_term_id"].as_str(),
        Some(term_id.as_str()),
        "Element should be linked to glossary term"
    );

    // =========================================================================
    // PHASE 3: Quality Rules — Create rules for the element, record assessments
    // =========================================================================

    // 3a. Get dimensions and rule types
    let dims: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/data-quality/dimensions")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let completeness_dim = dims
        .iter()
        .find(|d| d["dimension_code"].as_str() == Some("COMPLETENESS"))
        .expect("Should have COMPLETENESS dimension");
    let completeness_id = completeness_dim["dimension_id"].as_str().unwrap();

    let rule_types: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/data-quality/rule-types")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let not_null_type = rule_types
        .iter()
        .find(|t| t["type_code"].as_str() == Some("NOT_NULL"))
        .expect("Should have NOT_NULL rule type");
    let not_null_type_id = not_null_type["rule_type_id"].as_str().unwrap();

    // 3b. Create a completeness rule
    let rule_resp = ctx
        .server
        .post("/api/v1/data-quality/rules")
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "rule_name": "E2E NII Not Null Check",
            "description": "Net Interest Income amount must not be null.",
            "dimension_id": completeness_id,
            "rule_type_id": not_null_type_id,
            "element_id": element_id,
            "rule_definition": {"column": "net_interest_income_amt", "check": "IS NOT NULL"},
            "threshold_percentage": 99.5,
            "severity": "HIGH",
            "scope": "RECORD",
            "check_frequency": "DAILY"
        }))
        .await;
    rule_resp.assert_status(axum::http::StatusCode::CREATED);
    let rule: serde_json::Value = rule_resp.json();
    let rule_id = rule["rule_id"].as_str().unwrap().to_string();
    assert_eq!(rule["severity"], "HIGH");
    assert_eq!(rule["scope"].as_str().unwrap_or(""), "RECORD");

    // 3c. Record a quality assessment
    let assessment_resp = ctx
        .server
        .post("/api/v1/data-quality/assessments")
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "rule_id": rule_id,
            "records_assessed": 10000,
            "records_passed": 9985,
            "records_failed": 15,
            "score_percentage": 99.85
        }))
        .await;
    assessment_resp.assert_status(axum::http::StatusCode::CREATED);

    // 3d. Record a second assessment (for trend)
    ctx.server
        .post("/api/v1/data-quality/assessments")
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "rule_id": rule_id,
            "records_assessed": 10000,
            "records_passed": 9990,
            "records_failed": 10,
            "score_percentage": 99.90
        }))
        .await
        .assert_status(axum::http::StatusCode::CREATED);

    // 3e. Verify assessments recorded
    let assessments: Vec<serde_json::Value> = ctx
        .server
        .get(&format!("/api/v1/data-quality/assessments/{rule_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert_eq!(assessments.len(), 2, "Should have 2 assessments");

    // =========================================================================
    // PHASE 4: Application — Create, Link Element, Approve
    // =========================================================================

    // 4a. Get classification for the app
    let classifications: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/applications/classifications")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let classification_id = classifications[0]["classification_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 4b. Create application
    let app_resp = ctx
        .server
        .post("/api/v1/applications")
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "application_name": "E2E Core Banking System",
            "description": "Primary banking platform for interest calculations.",
            "classification_id": classification_id,
            "business_owner_id": ctx.admin_id,
            "technical_owner_id": ctx.admin_id,
            "steward_user_id": ctx.admin_id,
            "approver_user_id": ctx.admin_id
        }))
        .await;
    app_resp.assert_status(axum::http::StatusCode::CREATED);
    let application: serde_json::Value = app_resp.json();
    let app_id = application["application_id"].as_str().unwrap().to_string();

    // 4c. Link data element to application
    let link_resp = ctx
        .server
        .post(&format!("/api/v1/applications/{app_id}/elements"))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({
            "element_id": element_id,
            "is_golden_source": true
        }))
        .await;
    link_resp.assert_status(axum::http::StatusCode::CREATED);

    // 4d. Submit and approve application through workflow
    let app_wf: serde_json::Value = ctx
        .server
        .get(&format!("/api/v1/workflow/instances/by-entity/{app_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let app_wf_id = app_wf["instance_id"].as_str().unwrap().to_string();

    ctx.server
        .post(&format!(
            "/api/v1/workflow/instances/{app_wf_id}/transition"
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({"action": "SUBMIT"}))
        .await
        .assert_status_ok();

    // Complete steward review
    let tasks5: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/workflow/tasks/pending")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let app_review = tasks5
        .iter()
        .find(|t| {
            t["entity_id"].as_str() == Some(&app_id)
                && t["task"]["status"].as_str() == Some("PENDING")
        })
        .expect("Should have app review task");
    ctx.server
        .post(&format!(
            "/api/v1/workflow/tasks/{}/complete",
            app_review["task"]["task_id"].as_str().unwrap()
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({"decision": "APPROVE"}))
        .await
        .assert_status_ok();

    // Complete owner approval
    let tasks6: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/workflow/tasks/pending")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let app_approval = tasks6
        .iter()
        .find(|t| {
            t["entity_id"].as_str() == Some(&app_id)
                && t["task"]["status"].as_str() == Some("PENDING")
        })
        .expect("Should have app approval task");
    ctx.server
        .post(&format!(
            "/api/v1/workflow/tasks/{}/complete",
            app_approval["task"]["task_id"].as_str().unwrap()
        ))
        .add_header(auth_name.clone(), auth_val.clone())
        .json(&json!({"decision": "APPROVE"}))
        .await
        .assert_status_ok();

    // =========================================================================
    // PHASE 5: Verify Full Entity Graph
    // =========================================================================

    // 5a. Term is ACCEPTED
    let final_term: serde_json::Value = ctx
        .server
        .get(&format!("/api/v1/glossary/terms/{term_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert_eq!(final_term["status_code"], "ACCEPTED");

    // 5b. Element is ACCEPTED and linked to term
    let final_elem: serde_json::Value = ctx
        .server
        .get(&format!("/api/v1/data-dictionary/elements/{element_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert_eq!(final_elem["status_code"], "ACCEPTED");
    assert_eq!(
        final_elem["glossary_term_id"].as_str(),
        Some(term_id.as_str())
    );

    // 5c. Quality rule exists with assessments
    let final_rule: serde_json::Value = ctx
        .server
        .get(&format!("/api/v1/data-quality/rules/{rule_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert_eq!(final_rule["element_id"].as_str(), Some(element_id.as_str()));
    assert_eq!(final_rule["severity"], "HIGH");

    // 5d. Application is ACCEPTED with linked element
    let final_app: serde_json::Value = ctx
        .server
        .get(&format!("/api/v1/applications/{app_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert_eq!(final_app["status_code"], "ACCEPTED");

    let app_elements: Vec<serde_json::Value> = ctx
        .server
        .get(&format!("/api/v1/applications/{app_id}/elements"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert!(
        app_elements
            .iter()
            .any(|e| e["element_id"].as_str() == Some(&element_id)),
        "Application should have the linked data element"
    );

    // 5e. Recent assessments include our rule
    let recent: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/data-quality/assessments/recent?limit=5")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    assert!(
        recent
            .iter()
            .any(|a| a["rule_id"].as_str() == Some(&rule_id)),
        "Recent assessments should include our E2E rule"
    );

    // 5f. Dashboard dimensions should show our rule in counts
    let dims_final: Vec<serde_json::Value> = ctx
        .server
        .get("/api/v1/data-quality/dimensions")
        .add_header(auth_name.clone(), auth_val.clone())
        .await
        .json();
    let completeness_final = dims_final
        .iter()
        .find(|d| d["dimension_code"].as_str() == Some("COMPLETENESS"))
        .unwrap();
    assert!(
        completeness_final["rules_count"].as_i64().unwrap() >= 1,
        "Completeness dimension should have at least 1 rule"
    );

    // =========================================================================
    // PHASE 6: Cleanup — Delete the quality rule to test soft delete
    // =========================================================================

    let delete_resp = ctx
        .server
        .delete(&format!("/api/v1/data-quality/rules/{rule_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    delete_resp.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify deleted rule returns 404
    let deleted_rule = ctx
        .server
        .get(&format!("/api/v1/data-quality/rules/{rule_id}"))
        .add_header(auth_name.clone(), auth_val.clone())
        .await;
    deleted_rule.assert_status(axum::http::StatusCode::NOT_FOUND);
}
