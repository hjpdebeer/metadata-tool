use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::applications::*;
use crate::domain::glossary::PaginatedResponse;
use crate::error::{AppError, AppResult};
use crate::workflow;

// ---------------------------------------------------------------------------
// list_applications — GET /api/v1/applications
// ---------------------------------------------------------------------------

/// List applications with optional filtering and pagination.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/applications",
    params(SearchApplicationsRequest),
    responses(
        (status = 200, description = "Paginated list of applications",
         body = PaginatedApplications)
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_applications(
    State(state): State<AppState>,
    Query(params): Query<SearchApplicationsRequest>,
) -> AppResult<Json<PaginatedResponse<ApplicationListItem>>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Count query — mirrors the same WHERE conditions as the data query
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM applications a
        JOIN entity_statuses es ON es.status_id = a.status_id
        WHERE a.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR a.application_name ILIKE '%' || $1 || '%'
               OR a.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR a.classification_id = $2)
          AND ($3::TEXT IS NULL OR es.status_code = $3)
          AND ($4::BOOL IS NULL OR a.is_critical = $4)
          AND ($5::TEXT IS NULL OR a.deployment_type = $5)
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.classification_id)
    .bind(params.status.as_deref())
    .bind(params.is_critical)
    .bind(params.deployment_type.as_deref())
    .fetch_one(&state.pool)
    .await?;

    // Data query with joins for display fields
    let items = sqlx::query_as::<_, ApplicationListItem>(
        r#"
        SELECT
            a.application_id,
            a.application_name,
            a.application_code,
            a.description,
            ac.classification_name        AS classification_name,
            es.status_code                AS status_code,
            es.status_name                AS status_name,
            ubo.display_name              AS business_owner_name,
            uto.display_name              AS technical_owner_name,
            a.vendor,
            a.is_critical,
            a.deployment_type,
            a.created_at,
            a.updated_at
        FROM applications a
        JOIN entity_statuses es ON es.status_id = a.status_id
        LEFT JOIN application_classifications ac ON ac.classification_id = a.classification_id
        LEFT JOIN users ubo ON ubo.user_id = a.business_owner_id
        LEFT JOIN users uto ON uto.user_id = a.technical_owner_id
        WHERE a.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR a.application_name ILIKE '%' || $1 || '%'
               OR a.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR a.classification_id = $2)
          AND ($3::TEXT IS NULL OR es.status_code = $3)
          AND ($4::BOOL IS NULL OR a.is_critical = $4)
          AND ($5::TEXT IS NULL OR a.deployment_type = $5)
        ORDER BY a.application_name ASC
        LIMIT $6
        OFFSET $7
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.classification_id)
    .bind(params.status.as_deref())
    .bind(params.is_critical)
    .bind(params.deployment_type.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(PaginatedResponse {
        data: items,
        total_count,
        page,
        page_size,
    }))
}

// ---------------------------------------------------------------------------
// get_application — GET /api/v1/applications/:app_id
// ---------------------------------------------------------------------------

/// Retrieve a single application with full detail including linked processes and interfaces.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/applications/{app_id}",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    responses(
        (status = 200, description = "Application details", body = ApplicationFullView),
        (status = 404, description = "Application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn get_application(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<ApplicationFullView>> {
    // Single JOIN query resolving all FK lookups (ADR-0006 Pattern 1)
    let row = sqlx::query_as::<_, ApplicationDetailRow>(
        r#"
        SELECT
            a.application_id, a.application_name, a.application_code, a.description,
            a.classification_id, a.status_id, a.business_owner_id, a.technical_owner_id,
            a.vendor, a.version, a.deployment_type, a.technology_stack,
            a.is_critical, a.criticality_rationale, a.go_live_date, a.retirement_date,
            a.documentation_url, a.created_by, a.updated_by, a.created_at, a.updated_at,
            ac.classification_name,
            es.status_code,
            es.status_name,
            ubo.display_name              AS business_owner_name,
            uto.display_name              AS technical_owner_name,
            ucb.display_name              AS created_by_name,
            uub.display_name              AS updated_by_name,
            wi.instance_id                AS workflow_instance_id
        FROM applications a
        LEFT JOIN application_classifications ac ON ac.classification_id = a.classification_id
        LEFT JOIN entity_statuses es ON es.status_id = a.status_id
        LEFT JOIN users ubo ON ubo.user_id = a.business_owner_id
        LEFT JOIN users uto ON uto.user_id = a.technical_owner_id
        LEFT JOIN users ucb ON ucb.user_id = a.created_by
        LEFT JOIN users uub ON uub.user_id = a.updated_by
        LEFT JOIN workflow_instances wi ON wi.entity_id = a.application_id
            AND wi.completed_at IS NULL
        WHERE a.application_id = $1 AND a.deleted_at IS NULL
        "#,
    )
    .bind(app_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("application not found: {app_id}")))?;

    // Separate queries for junction/aggregate data only
    let data_elements_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM application_data_elements WHERE application_id = $1",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    let interfaces_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM application_interfaces
        WHERE (source_app_id = $1 OR target_app_id = $1) AND deleted_at IS NULL
        "#,
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    let linked_processes = sqlx::query_scalar::<_, String>(
        r#"
        SELECT bp.process_name
        FROM process_applications pa
        JOIN business_processes bp ON bp.process_id = pa.process_id
        WHERE pa.application_id = $1 AND bp.deleted_at IS NULL
        ORDER BY bp.process_name ASC
        "#,
    )
    .bind(app_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ApplicationFullView::from_row_and_junctions(
        row,
        data_elements_count,
        interfaces_count,
        linked_processes,
    )))
}

// ---------------------------------------------------------------------------
// create_application — POST /api/v1/applications
// ---------------------------------------------------------------------------

/// Create a new application in DRAFT status with an associated workflow instance.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/applications",
    request_body = CreateApplicationRequest,
    responses(
        (status = 201, description = "Application created", body = Application),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn create_application(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateApplicationRequest>,
) -> AppResult<(StatusCode, Json<Application>)> {
    // Validate required fields
    let application_name = body.application_name.trim().to_string();
    if application_name.is_empty() {
        return Err(AppError::Validation("application_name is required".into()));
    }
    let application_code = body.application_code.trim().to_string();
    if application_code.is_empty() {
        return Err(AppError::Validation("application_code is required".into()));
    }
    let description = body.description.trim().to_string();
    if description.is_empty() {
        return Err(AppError::Validation("description is required".into()));
    }

    // Validate deployment_type if provided
    if let Some(ref dt) = body.deployment_type
        && !["ON_PREMISE", "CLOUD", "HYBRID", "SAAS"].contains(&dt.as_str())
    {
        return Err(AppError::Validation(
            "deployment_type must be ON_PREMISE, CLOUD, HYBRID, or SAAS".into(),
        ));
    }

    // Look up DRAFT status_id from entity_statuses
    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    // Insert the new application
    let application = sqlx::query_as::<_, Application>(
        r#"
        INSERT INTO applications (
            application_name, application_code, description,
            classification_id, status_id, vendor, version,
            deployment_type, technology_stack, is_critical,
            criticality_rationale, go_live_date, documentation_url,
            created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING
            application_id, application_name, application_code, description,
            classification_id, status_id, business_owner_id, technical_owner_id,
            vendor, version, deployment_type, technology_stack,
            is_critical, criticality_rationale, go_live_date, retirement_date,
            documentation_url, created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(&application_name)
    .bind(&application_code)
    .bind(&description)
    .bind(body.classification_id)
    .bind(draft_status_id)
    .bind(body.vendor.as_deref())
    .bind(body.version.as_deref())
    .bind(body.deployment_type.as_deref())
    .bind(&body.technology_stack)
    .bind(body.is_critical.unwrap_or(false))
    .bind(body.criticality_rationale.as_deref())
    .bind(body.go_live_date)
    .bind(body.documentation_url.as_deref())
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    // Initiate the workflow instance for this new application
    workflow::service::initiate_workflow(
        &state.pool,
        workflow::ENTITY_APPLICATION,
        application.application_id,
        claims.sub,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(application)))
}

// ---------------------------------------------------------------------------
// update_application — PUT /api/v1/applications/:app_id
// ---------------------------------------------------------------------------

/// Update an existing application. Only provided fields are changed.
/// Requires authentication.
#[utoipa::path(
    put,
    path = "/api/v1/applications/{app_id}",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    request_body = UpdateApplicationRequest,
    responses(
        (status = 200, description = "Application updated", body = Application),
        (status = 404, description = "Application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn update_application(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateApplicationRequest>,
) -> AppResult<Json<Application>> {
    // Verify the application exists and is not deleted
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "application not found: {app_id}"
        )));
    }

    // Validate deployment_type if provided
    if let Some(ref dt) = body.deployment_type
        && !["ON_PREMISE", "CLOUD", "HYBRID", "SAAS"].contains(&dt.as_str())
    {
        return Err(AppError::Validation(
            "deployment_type must be ON_PREMISE, CLOUD, HYBRID, or SAAS".into(),
        ));
    }

    // Update using COALESCE to only change provided fields
    let application = sqlx::query_as::<_, Application>(
        r#"
        UPDATE applications
        SET application_name     = COALESCE($1, application_name),
            description          = COALESCE($2, description),
            classification_id    = COALESCE($3, classification_id),
            vendor               = COALESCE($4, vendor),
            version              = COALESCE($5, version),
            deployment_type      = COALESCE($6, deployment_type),
            technology_stack     = COALESCE($7, technology_stack),
            is_critical          = COALESCE($8, is_critical),
            criticality_rationale = COALESCE($9, criticality_rationale),
            retirement_date      = COALESCE($10, retirement_date),
            documentation_url    = COALESCE($11, documentation_url),
            updated_by           = $12,
            updated_at           = CURRENT_TIMESTAMP
        WHERE application_id = $13 AND deleted_at IS NULL
        RETURNING
            application_id, application_name, application_code, description,
            classification_id, status_id, business_owner_id, technical_owner_id,
            vendor, version, deployment_type, technology_stack,
            is_critical, criticality_rationale, go_live_date, retirement_date,
            documentation_url, created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(body.application_name.as_deref())
    .bind(body.description.as_deref())
    .bind(body.classification_id)
    .bind(body.vendor.as_deref())
    .bind(body.version.as_deref())
    .bind(body.deployment_type.as_deref())
    .bind(&body.technology_stack)
    .bind(body.is_critical)
    .bind(body.criticality_rationale.as_deref())
    .bind(body.retirement_date)
    .bind(body.documentation_url.as_deref())
    .bind(claims.sub)
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(application))
}

// ---------------------------------------------------------------------------
// list_classifications — GET /api/v1/applications/classifications
// ---------------------------------------------------------------------------

/// List application classification categories.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/applications/classifications",
    responses(
        (status = 200, description = "List application classifications",
         body = Vec<ApplicationClassification>)
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_classifications(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApplicationClassification>>> {
    let classifications = sqlx::query_as::<_, ApplicationClassification>(
        r#"
        SELECT classification_id, classification_code, classification_name, description
        FROM application_classifications
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(classifications))
}

// ---------------------------------------------------------------------------
// link_data_element — POST /api/v1/applications/:app_id/elements
// ---------------------------------------------------------------------------

/// Link a data element to an application with usage type and authoritative source flag.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/applications/{app_id}/elements",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    request_body = LinkDataElementRequest,
    responses(
        (status = 201, description = "Data element linked to application",
         body = ApplicationDataElementLink),
        (status = 404, description = "Application or element not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn link_data_element(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<LinkDataElementRequest>,
) -> AppResult<(StatusCode, Json<ApplicationDataElementLink>)> {
    // Verify the application exists
    let app_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    if !app_exists {
        return Err(AppError::NotFound(format!(
            "application not found: {app_id}"
        )));
    }

    // Verify the data element exists
    let element_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM data_elements WHERE element_id = $1 AND deleted_at IS NULL)",
    )
    .bind(body.element_id)
    .fetch_one(&state.pool)
    .await?;

    if !element_exists {
        return Err(AppError::NotFound(format!(
            "data element not found: {}",
            body.element_id
        )));
    }

    // Validate usage_type if provided
    let usage_type = body.usage_type.as_deref().unwrap_or("BOTH").to_string();
    if !["PRODUCER", "CONSUMER", "BOTH"].contains(&usage_type.as_str()) {
        return Err(AppError::Validation(
            "usage_type must be PRODUCER, CONSUMER, or BOTH".into(),
        ));
    }

    // Insert the link
    let link = sqlx::query_as::<_, ApplicationDataElementLink>(
        r#"
        INSERT INTO application_data_elements (
            application_id, element_id, usage_type, is_authoritative_source, description
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id, application_id, element_id,
            (SELECT element_name FROM data_elements WHERE element_id = $2) AS element_name,
            (SELECT element_code FROM data_elements WHERE element_id = $2) AS element_code,
            usage_type, is_authoritative_source, description, created_at
        "#,
    )
    .bind(app_id)
    .bind(body.element_id)
    .bind(&usage_type)
    .bind(body.is_authoritative_source.unwrap_or(false))
    .bind(body.description.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(link)))
}

// ---------------------------------------------------------------------------
// list_app_elements — GET /api/v1/applications/:app_id/elements
// ---------------------------------------------------------------------------

/// List data elements linked to an application.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/applications/{app_id}/elements",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    responses(
        (status = 200, description = "List data elements linked to application",
         body = Vec<ApplicationDataElementLink>),
        (status = 404, description = "Application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_app_elements(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<Vec<ApplicationDataElementLink>>> {
    // Verify the application exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "application not found: {app_id}"
        )));
    }

    let links = sqlx::query_as::<_, ApplicationDataElementLink>(
        r#"
        SELECT
            ade.id,
            ade.application_id,
            ade.element_id,
            de.element_name,
            de.element_code,
            ade.usage_type,
            ade.is_authoritative_source,
            ade.description,
            ade.created_at
        FROM application_data_elements ade
        JOIN data_elements de ON de.element_id = ade.element_id
        WHERE ade.application_id = $1
        ORDER BY de.element_name ASC
        "#,
    )
    .bind(app_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(links))
}

// ---------------------------------------------------------------------------
// list_interfaces — GET /api/v1/applications/:app_id/interfaces
// ---------------------------------------------------------------------------

/// List interfaces (data exchanges) for an application.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/applications/{app_id}/interfaces",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    responses(
        (status = 200, description = "List interfaces for application",
         body = Vec<ApplicationInterface>),
        (status = 404, description = "Application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_interfaces(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<Vec<ApplicationInterface>>> {
    // Verify the application exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "application not found: {app_id}"
        )));
    }

    let interfaces = sqlx::query_as::<_, ApplicationInterface>(
        r#"
        SELECT
            interface_id, source_app_id, target_app_id,
            interface_name, interface_type, protocol,
            frequency, description
        FROM application_interfaces
        WHERE (source_app_id = $1 OR target_app_id = $1) AND deleted_at IS NULL
        ORDER BY interface_name ASC
        "#,
    )
    .bind(app_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(interfaces))
}
