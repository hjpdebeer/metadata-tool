use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
#[cfg(debug_assertions)]
use utoipa_swagger_ui::SwaggerUi;

use metadata_tool::api;
use metadata_tool::config::AppConfig;
use metadata_tool::db::{self, AppState};
use metadata_tool::notifications;

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
        api::data_quality::delete_rule,
        api::data_quality::get_recent_assessments,
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

// ---------------------------------------------------------------------------
// Security headers middleware (TLS 1.3 + OWASP recommended headers)
// ---------------------------------------------------------------------------

async fn security_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // HSTS — enforce HTTPS for 1 year, including subdomains
    headers.insert(
        axum::http::header::STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains; preload"
            .parse()
            .unwrap(),
    );
    // Prevent MIME-type sniffing
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().unwrap(),
    );
    // Deny framing (clickjacking protection)
    headers.insert(axum::http::header::X_FRAME_OPTIONS, "DENY".parse().unwrap());
    // Control referrer information
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    // Restrict permissions (camera, microphone, geolocation, etc.)
    headers.insert(
        axum::http::HeaderName::from_static("permissions-policy"),
        "camera=(), microphone=(), geolocation=(), payment=()"
            .parse()
            .unwrap(),
    );
    // Prevent caching of API responses containing sensitive data
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        "no-store".parse().unwrap(),
    );

    response
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

    // Start notification email processor
    {
        let provider: std::sync::Arc<dyn notifications::provider::NotificationProvider> =
            match config.notification_email.provider.as_str() {
                "ses" => {
                    let ses = notifications::ses::SesProvider::new(
                        &config.notification_email.ses_region,
                        config.notification_email.ses_sender_email.clone(),
                    )
                    .await;
                    std::sync::Arc::new(ses)
                }
                "graph" => std::sync::Arc::new(notifications::graph::GraphProvider::new(
                    config.graph.tenant_id.clone(),
                    config.graph.client_id.clone(),
                    config.graph.client_secret.clone(),
                    config.graph.sender_email.clone(),
                )),
                _ => {
                    tracing::info!("Email notifications disabled (NOTIFICATION_PROVIDER=disabled)");
                    std::sync::Arc::new(notifications::disabled::DisabledProvider)
                }
            };
        notifications::processor::spawn(pool.clone(), provider);
    }

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

    let app = metadata_tool::build_router(state.clone());

    // SEC-023: Only expose Swagger UI in debug builds (disabled in production)
    #[cfg(debug_assertions)]
    let app = {
        let swagger_ui =
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());
        let swagger_router = Router::<()>::from(swagger_ui);
        app.merge(swagger_router)
    };

    // Serve frontend static files as fallback for non-API routes (SPA support)
    let frontend_dir = std::path::PathBuf::from(
        std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "./frontend/dist".into()),
    );
    let app = if frontend_dir.exists() {
        tracing::info!("Serving frontend from {}", frontend_dir.display());
        // Serve static files, falling back to index.html for SPA client-side routing
        let serve_dir = tower_http::services::ServeDir::new(&frontend_dir).not_found_service(
            tower_http::services::ServeFile::new(frontend_dir.join("index.html")),
        );
        app.fallback_service(serve_dir)
    } else {
        tracing::info!(
            "No frontend directory found at {}, API-only mode",
            frontend_dir.display()
        );
        app
    };

    let app = app
        .layer(axum::middleware::from_fn(security_headers))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    tracing::info!("Starting server on {addr}");
    #[cfg(debug_assertions)]
    tracing::info!("Swagger UI available at http://{addr}/swagger-ui/");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
