use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
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
        api::glossary::list_terms,
        api::glossary::get_term,
        api::glossary::create_term,
        api::glossary::update_term,
        api::glossary::list_domains,
        api::glossary::list_categories,
        api::glossary::ai_enrich_term,
        api::glossary::get_stats,
        api::data_dictionary::list_elements,
        api::data_dictionary::get_element,
        api::data_dictionary::create_element,
        api::data_dictionary::update_element,
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
        api::lineage::add_node,
        api::lineage::add_edge,
        api::lineage::impact_analysis,
        api::applications::list_applications,
        api::applications::get_application,
        api::applications::create_application,
        api::applications::update_application,
        api::processes::list_processes,
        api::processes::get_process,
        api::processes::create_process,
        api::processes::list_critical_processes,
        api::workflow::my_pending_tasks,
        api::workflow::get_instance,
        api::workflow::transition,
        api::workflow::complete_task,
        api::users::list_users,
        api::users::get_user,
        api::users::list_roles,
        api::ai::enrich,
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
        (name = "ai", description = "AI-powered metadata enrichment"),
    ),
    components(
        schemas(
            api::health::HealthResponse,
            api::auth::DevLoginRequest,
            api::auth::TokenResponse,
            api::auth::MeResponse,
            api::ai::AiEnrichRequest,
            api::ai::AiEnrichResponse,
            api::ai::AiSuggestion,
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

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(
            config
                .frontend_url
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_methods(Any)
        .allow_headers(Any);

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
        // Business Glossary
        .route("/api/v1/glossary/terms", get(api::glossary::list_terms).post(api::glossary::create_term))
        .route("/api/v1/glossary/terms/{term_id}", get(api::glossary::get_term).put(api::glossary::update_term))
        .route("/api/v1/glossary/terms/{term_id}/ai-enrich", post(api::glossary::ai_enrich_term))
        .route("/api/v1/glossary/domains", get(api::glossary::list_domains))
        .route("/api/v1/glossary/categories", get(api::glossary::list_categories))
        // Dashboard
        .route("/api/v1/stats", get(api::glossary::get_stats))
        // Data Dictionary
        .route("/api/v1/data-dictionary/elements", get(api::data_dictionary::list_elements).post(api::data_dictionary::create_element))
        .route("/api/v1/data-dictionary/elements/cde", get(api::data_dictionary::list_cde))
        .route("/api/v1/data-dictionary/elements/{element_id}", get(api::data_dictionary::get_element).put(api::data_dictionary::update_element))
        .route("/api/v1/data-dictionary/elements/{element_id}/cde", post(api::data_dictionary::designate_cde))
        .route("/api/v1/data-dictionary/classifications", get(api::data_dictionary::list_classifications))
        .route("/api/v1/data-dictionary/source-systems", get(api::data_dictionary::list_source_systems).post(api::data_dictionary::create_source_system))
        .route("/api/v1/data-dictionary/source-systems/{system_id}/schemas", get(api::data_dictionary::list_schemas).post(api::data_dictionary::create_schema))
        .route("/api/v1/data-dictionary/schemas/{schema_id}/tables", get(api::data_dictionary::list_tables).post(api::data_dictionary::create_table))
        .route("/api/v1/data-dictionary/tables/{table_id}/columns", get(api::data_dictionary::list_columns).post(api::data_dictionary::create_column))
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
        .route("/api/v1/lineage/graphs/{graph_id}", get(api::lineage::get_graph))
        .route("/api/v1/lineage/graphs/{graph_id}/nodes", post(api::lineage::add_node))
        .route("/api/v1/lineage/graphs/{graph_id}/edges", post(api::lineage::add_edge))
        .route("/api/v1/lineage/impact/{node_id}", get(api::lineage::impact_analysis))
        // Applications
        .route("/api/v1/applications", get(api::applications::list_applications).post(api::applications::create_application))
        .route("/api/v1/applications/{app_id}", get(api::applications::get_application).put(api::applications::update_application))
        // Processes
        .route("/api/v1/processes", get(api::processes::list_processes).post(api::processes::create_process))
        .route("/api/v1/processes/{process_id}", get(api::processes::get_process))
        .route("/api/v1/processes/critical", get(api::processes::list_critical_processes))
        // Workflow
        .route("/api/v1/workflow/tasks/pending", get(api::workflow::my_pending_tasks))
        .route("/api/v1/workflow/instances/{instance_id}", get(api::workflow::get_instance))
        .route("/api/v1/workflow/instances/{instance_id}/transition", post(api::workflow::transition))
        .route("/api/v1/workflow/tasks/{task_id}/complete", post(api::workflow::complete_task))
        // Users
        .route("/api/v1/users", get(api::users::list_users))
        .route("/api/v1/users/{user_id}", get(api::users::get_user))
        .route("/api/v1/roles", get(api::users::list_roles))
        // AI
        .route("/api/v1/ai/enrich", post(api::ai::enrich))
        // Apply auth middleware to all protected routes
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // Swagger UI router
    let swagger_ui = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi());
    let swagger_router = Router::<()>::from(swagger_ui);

    // Combine all routes
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
        .merge(swagger_router)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    tracing::info!("Starting server on {addr}");
    tracing::info!("Swagger UI available at http://{addr}/swagger-ui/");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
