//! # Metadata Tool Backend
//!
//! Enterprise metadata lifecycle management for data governance in financial institutions.
//! Provides REST API, domain models, workflow engine, AI enrichment, and notification
//! services for managing business glossary terms, data dictionaries, data quality rules,
//! data lineage, business applications, and business processes.
//!
//! ## Architecture (ADR-0001: Modular Monolith)
//!
//! - [`api`] — Axum HTTP route handlers with OpenAPI annotations
//! - [`domain`] — Rust structs for database entities, request/response DTOs
//! - [`auth`] — JWT authentication and RBAC middleware
//! - [`workflow`] — Generic state machine for entity lifecycle (Principle 5)
//! - [`ai`] — Claude/OpenAI integration for metadata enrichment (Principle 6)
//! - [`notifications`] — Email queue and in-app notifications
//! - [`naming`] — Technical metadata naming standards validation (Principle 8)
//! - [`db`] — PostgreSQL connection pool and shared application state
//!
//! ## Key Principles
//!
//! All code must comply with the 14 foundational principles in
//! `METADATA_TOOL_PRINCIPLES.md` and coding standards in `CODING_STANDARDS.md`.

pub mod ai;
pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod naming;
pub mod notifications;
pub mod settings;
pub mod workflow;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{delete, get, post, put};

use auth::middleware::require_auth;
use db::AppState;

/// Build the application router with all API routes.
///
/// Contains all public and protected routes merged together with shared state.
/// Does **not** include Swagger UI, frontend static file serving, security
/// headers, TraceLayer, or CORS layers — those are deployment concerns added
/// in `main`.
pub fn build_router(state: AppState) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/api/v1/health", get(api::health::health_check))
        .route("/api/v1/auth/dev-login", post(api::auth::dev_login))
        .route("/api/v1/auth/login", get(api::auth::login))
        .route("/api/v1/auth/callback", get(api::auth::callback));

    // Protected routes (require valid JWT)
    let protected_routes = Router::new()
        // Auth
        .route("/api/v1/auth/me", get(api::auth::me))
        .route("/api/v1/auth/me/profile", get(api::auth::me_profile))
        // Business Glossary — bulk upload routes BEFORE {term_id} to avoid path conflicts
        .route("/api/v1/glossary/terms/bulk-upload/template", get(api::bulk_upload::download_template))
        .route("/api/v1/glossary/terms/bulk-upload",
            post(api::bulk_upload::bulk_upload)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        )
        .route("/api/v1/glossary/terms", get(api::glossary::list_terms).post(api::glossary::create_term))
        .route("/api/v1/glossary/terms/{term_id}", get(api::glossary::get_term).put(api::glossary::update_term))
        .route("/api/v1/glossary/terms/{term_id}/amend", post(api::glossary::amend_term))
        .route("/api/v1/glossary/terms/{term_id}/discard", delete(api::glossary::discard_amendment))
        .route("/api/v1/glossary/terms/{term_id}/ai-enrich", post(api::glossary::ai_enrich_term))
        .route("/api/v1/glossary/terms/{term_id}/regulatory-tags", post(api::glossary::attach_regulatory_tag))
        .route("/api/v1/glossary/terms/{term_id}/regulatory-tags/{tag_id}", delete(api::glossary::detach_regulatory_tag))
        .route("/api/v1/glossary/terms/{term_id}/subject-areas", post(api::glossary::attach_subject_area))
        .route("/api/v1/glossary/terms/{term_id}/subject-areas/{area_id}", delete(api::glossary::detach_subject_area))
        .route("/api/v1/glossary/terms/{term_id}/tags", post(api::glossary::attach_tag))
        .route("/api/v1/glossary/terms/{term_id}/tags/{tag_id}", delete(api::glossary::detach_tag))
        .route("/api/v1/glossary/terms/{term_id}/aliases", post(api::glossary::add_alias))
        .route("/api/v1/glossary/terms/{term_id}/aliases/{alias_id}", delete(api::glossary::remove_alias))
        .route("/api/v1/glossary/domains", get(api::glossary::list_domains))
        .route("/api/v1/glossary/categories", get(api::glossary::list_categories))
        .route("/api/v1/glossary/term-types", get(api::glossary::list_term_types))
        .route("/api/v1/glossary/review-frequencies", get(api::glossary::list_review_frequencies))
        .route("/api/v1/glossary/confidence-levels", get(api::glossary::list_confidence_levels))
        .route("/api/v1/glossary/visibility-levels", get(api::glossary::list_visibility_levels))
        .route("/api/v1/glossary/units-of-measure", get(api::glossary::list_units_of_measure))
        .route("/api/v1/glossary/regulatory-tags", get(api::glossary::list_regulatory_tags))
        .route("/api/v1/glossary/subject-areas", get(api::glossary::list_subject_areas))
        .route("/api/v1/glossary/languages", get(api::glossary::list_languages))
        .route("/api/v1/glossary/organisational-units", get(api::glossary::list_organisational_units))
        // Dashboard
        .route("/api/v1/stats", get(api::glossary::get_stats))
        // Data Dictionary — bulk upload routes BEFORE {element_id} to avoid path conflicts
        .route("/api/v1/data-dictionary/elements/bulk-upload/template", get(api::de_bulk_upload::download_de_template))
        .route("/api/v1/data-dictionary/elements/bulk-upload",
            post(api::de_bulk_upload::bulk_upload_elements)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        )
        // Data Dictionary
        .route("/api/v1/data-dictionary/elements", get(api::data_dictionary::list_elements).post(api::data_dictionary::create_element))
        .route("/api/v1/data-dictionary/elements/cde", get(api::data_dictionary::list_cde))
        .route("/api/v1/data-dictionary/elements/{element_id}/amend", post(api::data_dictionary::amend_element))
        .route("/api/v1/data-dictionary/elements/{element_id}/discard", delete(api::data_dictionary::discard_amendment))
        .route("/api/v1/data-dictionary/elements/{element_id}", get(api::data_dictionary::get_element).put(api::data_dictionary::update_element))
        .route("/api/v1/data-dictionary/elements/{element_id}/cde", post(api::data_dictionary::designate_cde))
        .route("/api/v1/data-dictionary/classifications", get(api::data_dictionary::list_classifications))
        .route("/api/v1/data-dictionary/source-systems", get(api::data_dictionary::list_source_systems).post(api::data_dictionary::create_source_system))
        .route("/api/v1/data-dictionary/source-systems/{system_id}/schemas", get(api::data_dictionary::list_schemas).post(api::data_dictionary::create_schema))
        .route("/api/v1/data-dictionary/schemas/{schema_id}/tables", get(api::data_dictionary::list_tables).post(api::data_dictionary::create_table))
        .route("/api/v1/data-dictionary/tables/{table_id}/columns", get(api::data_dictionary::list_columns).post(api::data_dictionary::create_column))
        // Data Dictionary — technical metadata ingestion
        .route("/api/v1/data-dictionary/ingest/technical",
            post(api::ingestion::ingest_technical)
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        )
        .route("/api/v1/data-dictionary/ingest/elements",
            post(api::ingestion::ingest_elements)
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        )
        .route("/api/v1/data-dictionary/ingest/link-columns",
            post(api::ingestion::link_columns)
        )
        // Data Quality — score ingestion
        .route("/api/v1/data-quality/ingest/scores",
            post(api::ingestion::ingest_scores)
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
        )
        // Data Quality
        .route("/api/v1/data-quality/dimensions", get(api::data_quality::list_dimensions))
        .route("/api/v1/data-quality/rule-types", get(api::data_quality::list_rule_types))
        .route("/api/v1/data-quality/rules", get(api::data_quality::list_rules).post(api::data_quality::create_rule))
        .route("/api/v1/data-quality/rules/{rule_id}", get(api::data_quality::get_rule).put(api::data_quality::update_rule).delete(api::data_quality::delete_rule))
        .route("/api/v1/data-quality/assessments", post(api::data_quality::create_assessment))
        .route("/api/v1/data-quality/assessments/recent", get(api::data_quality::get_recent_assessments))
        .route("/api/v1/data-quality/assessments/{rule_id}", get(api::data_quality::get_assessments))
        .route("/api/v1/data-quality/scores/element/{element_id}", get(api::data_quality::get_element_scores))
        // Data Lineage
        .route("/api/v1/lineage/graphs", get(api::lineage::list_graphs).post(api::lineage::create_graph))
        .route("/api/v1/lineage/graphs/{graph_id}", get(api::lineage::get_graph).put(api::lineage::update_graph))
        .route("/api/v1/lineage/graphs/{graph_id}/nodes", post(api::lineage::add_node))
        .route("/api/v1/lineage/graphs/{graph_id}/edges", post(api::lineage::add_edge))
        .route("/api/v1/lineage/node-types", get(api::lineage::list_node_types))
        .route("/api/v1/lineage/nodes/{node_id}/position", put(api::lineage::update_node_position))
        .route("/api/v1/lineage/nodes/{node_id}", delete(api::lineage::delete_node))
        .route("/api/v1/lineage/edges/{edge_id}", delete(api::lineage::delete_edge))
        .route("/api/v1/lineage/impact/{node_id}", get(api::lineage::impact_analysis))
        // Applications — bulk upload routes BEFORE {app_id} to avoid path conflicts
        .route("/api/v1/applications/bulk-upload/template", get(api::app_bulk_upload::download_app_template))
        .route("/api/v1/applications/bulk-upload",
            post(api::app_bulk_upload::bulk_upload_apps)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        )
        .route("/api/v1/applications", get(api::applications::list_applications).post(api::applications::create_application))
        .route("/api/v1/applications/classifications", get(api::applications::list_classifications))
        .route("/api/v1/applications/dr-tiers", get(api::applications::list_dr_tiers))
        .route("/api/v1/applications/lifecycle-stages", get(api::applications::list_lifecycle_stages))
        .route("/api/v1/applications/criticality-tiers", get(api::applications::list_criticality_tiers))
        .route("/api/v1/applications/risk-ratings", get(api::applications::list_risk_ratings))
        .route("/api/v1/applications/{app_id}/amend", post(api::applications::amend_application))
        .route("/api/v1/applications/{app_id}/discard", delete(api::applications::discard_amendment))
        .route("/api/v1/applications/{app_id}", get(api::applications::get_application).put(api::applications::update_application))
        .route("/api/v1/applications/{app_id}/elements", get(api::applications::list_app_elements).post(api::applications::link_data_element))
        .route("/api/v1/applications/{app_id}/interfaces", get(api::applications::list_interfaces))
        // Processes
        .route("/api/v1/processes", get(api::processes::list_processes).post(api::processes::create_process))
        .route("/api/v1/processes/critical", get(api::processes::list_critical_processes))
        .route("/api/v1/processes/categories", get(api::processes::list_categories))
        .route("/api/v1/processes/{process_id}", get(api::processes::get_process).put(api::processes::update_process))
        .route("/api/v1/processes/{process_id}/steps", get(api::processes::list_steps).post(api::processes::add_step))
        .route("/api/v1/processes/{process_id}/elements", get(api::processes::list_process_elements).post(api::processes::link_data_element))
        .route("/api/v1/processes/{process_id}/applications", get(api::processes::list_process_applications).post(api::processes::link_application))
        // Workflow
        .route("/api/v1/workflow/tasks/pending", get(api::workflow::my_pending_tasks))
        .route("/api/v1/workflow/instances/by-entity/{entity_id}", get(api::workflow::get_instance_by_entity))
        .route("/api/v1/workflow/instances/{instance_id}", get(api::workflow::get_instance))
        .route("/api/v1/workflow/instances/{instance_id}/transition", post(api::workflow::transition))
        .route("/api/v1/workflow/tasks/{task_id}/complete", post(api::workflow::complete_task))
        // Users
        .route("/api/v1/users", get(api::users::list_users))
        .route("/api/v1/users/lookup", get(api::users::lookup_users))
        .route("/api/v1/users/{user_id}", get(api::users::get_user).put(api::users::update_user))
        .route("/api/v1/users/{user_id}/roles", post(api::users::assign_role))
        .route("/api/v1/users/{user_id}/roles/{role_id}", delete(api::users::remove_role))
        .route("/api/v1/users/{user_id}/confirm-roles", post(api::users::confirm_roles))
        .route("/api/v1/roles", get(api::users::list_roles))
        // Notifications
        .route("/api/v1/notifications", get(api::notifications::list_notifications))
        .route("/api/v1/notifications/read-all", post(api::notifications::mark_all_read))
        .route("/api/v1/notifications/unread-count", get(api::notifications::unread_count))
        .route("/api/v1/notifications/preferences", get(api::notifications::get_preferences).put(api::notifications::update_preferences))
        .route("/api/v1/notifications/{notification_id}/read", post(api::notifications::mark_read))
        // AI
        .route("/api/v1/ai/enrich", post(api::ai::enrich))
        .route("/api/v1/ai/suggest-quality-rules", post(api::ai::suggest_quality_rules))
        .route("/api/v1/ai/suggestions/{entity_type}/{entity_id}", get(api::ai::list_suggestions))
        .route("/api/v1/ai/suggestions/{suggestion_id}/accept", post(api::ai::accept_suggestion))
        .route("/api/v1/ai/suggestions/{suggestion_id}/reject", post(api::ai::reject_suggestion))
        .route("/api/v1/ai/suggestions/{suggestion_id}/feedback", post(api::ai::submit_feedback))
        // Data Quality — accept AI-suggested rule
        .route("/api/v1/data-quality/rules/from-suggestion", post(api::data_quality::accept_rule_suggestion))
        // Admin — settings
        .route("/api/v1/admin/settings", get(api::admin::list_settings))
        .route("/api/v1/admin/settings/{key}", put(api::admin::update_setting))
        .route("/api/v1/admin/settings/{key}/reveal", get(api::admin::reveal_setting))
        .route("/api/v1/admin/settings/test-connection/{key}", post(api::admin::test_connection))
        // Admin — API key management
        .route("/api/v1/admin/api-keys", get(api::admin::list_api_keys).post(api::admin::create_api_key))
        .route("/api/v1/admin/api-keys/{key_id}", delete(api::admin::deactivate_api_key))
        // Admin — lookup table CRUD
        .route("/api/v1/admin/lookups/{table_name}", get(api::admin::list_lookup).post(api::admin::create_lookup))
        .route("/api/v1/admin/lookups/{table_name}/{id}", put(api::admin::update_lookup).delete(api::admin::delete_lookup))
        .route("/api/v1/admin/lookups/{table_name}/{id}/usage-count", get(api::admin::get_lookup_usage_count))
        // Apply auth middleware to all protected routes
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
}
