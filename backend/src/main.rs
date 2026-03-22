use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{delete, get, post, put};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
#[cfg(debug_assertions)]
use utoipa_swagger_ui::SwaggerUi;

use metadata_tool::api;
use metadata_tool::auth::middleware::require_auth;
use metadata_tool::config::AppConfig;
use metadata_tool::db::{self, AppState};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Metadata Management Tool",
        version = "0.1.0",
        description = "Enterprise metadata lifecycle management for data governance",
        license(name = "MIT OR Apache-2.0")
    ),
    paths(
        api::health::health_check,
        api::auth::dev_login,
        api::auth::login,
        api::auth::callback,
        api::auth::me,
        api::auth::me_profile,
        api::bulk_upload::download_template,
        api::bulk_upload::bulk_upload,
        api::glossary::list_terms,
        api::glossary::get_term,
        api::glossary::create_term,
        api::glossary::update_term,
        api::glossary::amend_term,
        api::glossary::discard_amendment,
        api::glossary::list_domains,
        api::glossary::list_categories,
        api::glossary::list_term_types,
        api::glossary::list_review_frequencies,
        api::glossary::list_confidence_levels,
        api::glossary::list_visibility_levels,
        api::glossary::list_units_of_measure,
        api::glossary::list_regulatory_tags,
        api::glossary::list_subject_areas,
        api::glossary::list_languages,
        api::glossary::list_organisational_units,
        api::glossary::attach_regulatory_tag,
        api::glossary::detach_regulatory_tag,
        api::glossary::attach_subject_area,
        api::glossary::detach_subject_area,
        api::glossary::attach_tag,
        api::glossary::detach_tag,
        api::glossary::ai_enrich_term,
        api::glossary::get_stats,
        api::data_dictionary::list_elements,
        api::data_dictionary::get_element,
        api::data_dictionary::create_element,
        api::data_dictionary::update_element,
        api::data_dictionary::amend_element,
        api::data_dictionary::discard_amendment,
        api::data_dictionary::list_cde,
        api::data_dictionary::designate_cde,
        api::data_dictionary::list_source_systems,
        api::data_dictionary::create_source_system,
        api::data_dictionary::list_classifications,
        api::data_dictionary::list_schemas,
        api::data_dictionary::create_schema,
        api::data_dictionary::list_tables,
        api::data_dictionary::create_table,
        api::data_dictionary::list_columns,
        api::data_dictionary::create_column,
        // Data Dictionary — technical metadata ingestion
        api::ingestion::ingest_technical,
        api::ingestion::ingest_elements,
        api::ingestion::link_columns,
        // Data Quality — score ingestion
        api::ingestion::ingest_scores,
        api::data_quality::list_dimensions,
        api::data_quality::list_rule_types,
        api::data_quality::list_rules,
        api::data_quality::get_rule,
        api::data_quality::create_rule,
        api::data_quality::update_rule,
        api::data_quality::get_assessments,
        api::data_quality::create_assessment,
        api::data_quality::get_element_scores,
        api::lineage::list_graphs,
        api::lineage::get_graph,
        api::lineage::create_graph,
        api::lineage::update_graph,
        api::lineage::add_node,
        api::lineage::update_node_position,
        api::lineage::delete_node,
        api::lineage::add_edge,
        api::lineage::delete_edge,
        api::lineage::list_node_types,
        api::lineage::impact_analysis,
        // Data Dictionary — bulk upload
        api::de_bulk_upload::download_de_template,
        api::de_bulk_upload::bulk_upload_elements,
        // Applications — bulk upload
        api::app_bulk_upload::download_app_template,
        api::app_bulk_upload::bulk_upload_apps,
        // Applications
        api::applications::list_applications,
        api::applications::get_application,
        api::applications::create_application,
        api::applications::update_application,
        api::applications::list_classifications,
        api::applications::list_dr_tiers,
        api::applications::list_lifecycle_stages,
        api::applications::list_criticality_tiers,
        api::applications::list_risk_ratings,
        api::applications::link_data_element,
        api::applications::list_app_elements,
        api::applications::list_interfaces,
        api::applications::amend_application,
        api::applications::discard_amendment,
        // Processes
        api::processes::list_processes,
        api::processes::get_process,
        api::processes::create_process,
        api::processes::update_process,
        api::processes::list_critical_processes,
        api::processes::list_categories,
        api::processes::add_step,
        api::processes::list_steps,
        api::processes::link_data_element,
        api::processes::list_process_elements,
        api::processes::link_application,
        api::processes::list_process_applications,
        // Workflow
        api::workflow::my_pending_tasks,
        api::workflow::get_instance,
        api::workflow::get_instance_by_entity,
        api::workflow::transition,
        api::workflow::complete_task,
        api::users::list_users,
        api::users::get_user,
        api::users::update_user,
        api::users::assign_role,
        api::users::remove_role,
        api::users::confirm_roles,
        api::users::list_roles,
        api::users::lookup_users,
        // Notifications
        api::notifications::list_notifications,
        api::notifications::mark_read,
        api::notifications::mark_all_read,
        api::notifications::unread_count,
        api::notifications::get_preferences,
        api::notifications::update_preferences,
        // AI
        api::ai::enrich,
        api::ai::list_suggestions,
        api::ai::accept_suggestion,
        api::ai::reject_suggestion,
        api::ai::submit_feedback,
        api::ai::suggest_quality_rules,
        // Data Quality — accept AI suggestion
        api::data_quality::accept_rule_suggestion,
        // Admin
        api::admin::list_settings,
        api::admin::update_setting,
        api::admin::reveal_setting,
        api::admin::test_connection,
        api::admin::list_lookup,
        api::admin::create_lookup,
        api::admin::update_lookup,
        api::admin::delete_lookup,
        api::admin::get_lookup_usage_count,
        // Admin — API key management
        api::admin::create_api_key,
        api::admin::list_api_keys,
        api::admin::deactivate_api_key,
    ),
    tags(
        (name = "health", description = "Health check"),
        (name = "auth", description = "Authentication & SSO"),
        (name = "glossary", description = "Business Glossary management"),
        (name = "data_dictionary", description = "Data Dictionary & CDE management"),
        (name = "data_quality", description = "Data Quality rules & assessments"),
        (name = "lineage", description = "Data Lineage graphs & impact analysis"),
        (name = "applications", description = "Business Application Registry"),
        (name = "processes", description = "Business Process Registry"),
        (name = "workflow", description = "Workflow engine & task management"),
        (name = "users", description = "User & role management"),
        (name = "notifications", description = "Notification management"),
        (name = "ai", description = "AI-powered metadata enrichment"),
        (name = "admin", description = "Admin panel — settings & lookup table management"),
    ),
    components(
        schemas(
            api::health::HealthResponse,
            api::auth::DevLoginRequest,
            api::auth::TokenResponse,
            api::auth::MeResponse,
            metadata_tool::domain::glossary::BulkUploadResult,
            metadata_tool::domain::glossary::BulkUploadError,
            metadata_tool::domain::ai::AiEnrichRequest,
            metadata_tool::domain::ai::AiEnrichResponse,
            metadata_tool::domain::ai::AiSuggestionResponse,
            metadata_tool::domain::ai::AiSuggestion,
            metadata_tool::domain::ai::AcceptSuggestionRequest,
            metadata_tool::domain::ai::RejectSuggestionRequest,
            metadata_tool::domain::ai::FeedbackRequest,
            metadata_tool::domain::ai::FeedbackResponse,
            metadata_tool::domain::ai::AiSuggestRulesRequest,
            metadata_tool::domain::ai::AiRuleSuggestion,
            metadata_tool::domain::ai::AiSuggestRulesResponse,
            metadata_tool::domain::ai::AcceptRuleSuggestionRequest,
            metadata_tool::settings::SystemSettingResponse,
            metadata_tool::settings::UpdateSettingRequest,
            metadata_tool::settings::UpdateSettingResponse,
            metadata_tool::settings::TestConnectionResponse,
            api::admin::SettingsListResponse,
            api::admin::RevealSettingResponse,
            api::admin::LookupRow,
            api::admin::LookupRowRequest,
            api::admin::LookupListResponse,
            api::admin::UsageCountResponse,
            // Ingestion
            api::ingestion::IngestTechnicalRequest,
            api::ingestion::IngestTechnicalResponse,
            api::ingestion::IngestSourceSystem,
            api::ingestion::IngestSchema,
            api::ingestion::IngestTable,
            api::ingestion::IngestColumn,
            api::ingestion::IngestRelationship,
            api::ingestion::IngestOptions,
            api::ingestion::IngestSummary,
            api::ingestion::IngestCounts,
            api::ingestion::IngestStaleCounts,
            api::ingestion::IngestError,
            api::ingestion::IngestWarning,
            // Element ingestion
            api::ingestion::IngestElementsRequest,
            api::ingestion::IngestElement,
            api::ingestion::IngestElementOptions,
            api::ingestion::IngestElementsResponse,
            api::ingestion::IngestElementSummary,
            // Column-element linking
            api::ingestion::LinkColumnsRequest,
            api::ingestion::ColumnElementLink,
            api::ingestion::LinkColumnsResponse,
            api::ingestion::LinkColumnsSummary,
            // Quality score ingestion
            api::ingestion::IngestScoresRequest,
            api::ingestion::ProfilingRun,
            api::ingestion::ScoreEntry,
            api::ingestion::IngestScoresResponse,
            // API key management
            api::admin::CreateApiKeyRequest,
            api::admin::CreateApiKeyResponse,
            api::admin::ApiKeyListItem,
            api::admin::ApiKeyListResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();
        components.add_security_scheme(
            "bearer_auth",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::HttpBuilder::new()
                    .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Load configuration
    let config = AppConfig::from_env()?;
    let addr = format!("{}:{}", config.host, config.port);

    // Connect to database and run migrations
    // Note: Docker Compose creates the database automatically via POSTGRES_DB
    let pool = db::create_pool(&config.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations applied");

    let state = AppState::new(pool.clone(), config.clone());

    // SEC-003: Warn if seeded dev accounts exist when Entra SSO is configured
    let entra_configured = !config.entra.tenant_id.is_empty()
        && config.entra.tenant_id != "your-tenant-id"
        && uuid::Uuid::parse_str(&config.entra.tenant_id).is_ok();

    if entra_configured {
        let seeded_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE email LIKE '%@example.com' AND is_active = TRUE",
        )
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

        if seeded_count > 0 {
            tracing::warn!(
                count = seeded_count,
                "SECURITY WARNING: {seeded_count} seeded development accounts (@example.com) are active \
                 while Entra SSO is configured. These accounts should be deactivated or deleted \
                 before production deployment."
            );
        }
    }

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(
            config
                .frontend_url
                .parse::<axum::http::HeaderValue>()
                .map_err(|e| {
                    anyhow::anyhow!("invalid FRONTEND_URL '{}': {e}", config.frontend_url)
                })?,
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::HeaderName::from_static("x-api-key"),
        ]);

    // TODO(SEC-006): Add rate limiting to auth endpoints.
    // tower-governor 0.4 is incompatible with axum 0.8; evaluate tower-governor 0.5+
    // or implement a custom middleware using std::sync::Arc<tokio::sync::Semaphore>.
    // Target: 2 req/s sustained, burst of 5, per IP on auth endpoints.

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
        .route("/api/v1/data-quality/rules/{rule_id}", get(api::data_quality::get_rule).put(api::data_quality::update_rule))
        .route("/api/v1/data-quality/assessments", post(api::data_quality::create_assessment))
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
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state);

    // SEC-023: Only expose Swagger UI in debug builds (disabled in production)
    #[cfg(debug_assertions)]
    let app = {
        let swagger_ui =
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());
        let swagger_router = Router::<()>::from(swagger_ui);
        app.merge(swagger_router)
    };

    let app = app.layer(TraceLayer::new_for_http()).layer(cors);

    tracing::info!("Starting server on {addr}");
    #[cfg(debug_assertions)]
    tracing::info!("Swagger UI available at http://{addr}/swagger-ui/");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
