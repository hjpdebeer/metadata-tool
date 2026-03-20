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

/// List applications with optional filtering, pagination, and visibility filtering.
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
    Extension(claims): Extension<Claims>,
    Query(params): Query<SearchApplicationsRequest>,
) -> AppResult<Json<PaginatedResponse<ApplicationListItem>>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let is_admin = claims.roles.iter().any(|r| r == "ADMIN" || r == "admin");

    let visibility_clause = r#"
          AND (
              (a.is_current_version = TRUE AND (
                  es.status_code IN ('ACCEPTED', 'DEPRECATED')
                  OR a.created_by = $6
                  OR a.business_owner_id = $6
                  OR a.technical_owner_id = $6
                  OR a.steward_user_id = $6
                  OR a.approver_user_id = $6
                  OR $7::BOOLEAN = TRUE
              ))
              OR (a.is_current_version = FALSE AND es.status_code NOT IN ('SUPERSEDED', 'REJECTED') AND (
                  a.created_by = $6
                  OR a.business_owner_id = $6
                  OR a.technical_owner_id = $6
                  OR a.steward_user_id = $6
                  OR a.approver_user_id = $6
                  OR $7::BOOLEAN = TRUE
              ))
          )
    "#;

    let count_query = format!(
        r#"
        SELECT COUNT(*)
        FROM applications a
        JOIN entity_statuses es ON es.status_id = a.status_id
        WHERE a.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR a.application_name ILIKE '%' || $1 || '%'
               OR a.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR a.classification_id = $2)
          AND ($3::TEXT IS NULL OR es.status_code = $3)
          AND ($4::BOOL IS NULL OR a.is_cba = $4)
          AND ($5::TEXT IS NULL OR a.deployment_type = $5)
          {visibility}
        "#,
        visibility = visibility_clause,
    );

    let total_count = sqlx::query_scalar::<_, i64>(&count_query)
    .bind(params.query.as_deref())
    .bind(params.classification_id)
    .bind(params.status.as_deref())
    .bind(params.is_cba)
    .bind(params.deployment_type.as_deref())
    .bind(claims.sub)
    .bind(is_admin)
    .fetch_one(&state.pool)
    .await?;

    let data_query = format!(
        r#"
        SELECT
            a.application_id,
            a.application_name,
            a.application_code,
            a.description,
            a.abbreviation,
            ac.classification_name,
            es.status_code,
            es.status_name,
            ubo.display_name              AS business_owner_name,
            uto.display_name              AS technical_owner_name,
            a.vendor,
            a.is_cba,
            a.deployment_type,
            als.stage_name                AS lifecycle_stage_name,
            a.created_at,
            a.updated_at
        FROM applications a
        JOIN entity_statuses es ON es.status_id = a.status_id
        LEFT JOIN application_classifications ac ON ac.classification_id = a.classification_id
        LEFT JOIN users ubo ON ubo.user_id = a.business_owner_id
        LEFT JOIN users uto ON uto.user_id = a.technical_owner_id
        LEFT JOIN application_lifecycle_stages als ON als.stage_id = a.lifecycle_stage_id
        WHERE a.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR a.application_name ILIKE '%' || $1 || '%'
               OR a.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR a.classification_id = $2)
          AND ($3::TEXT IS NULL OR es.status_code = $3)
          AND ($4::BOOL IS NULL OR a.is_cba = $4)
          AND ($5::TEXT IS NULL OR a.deployment_type = $5)
          {visibility}
        ORDER BY a.application_name ASC
        LIMIT $8
        OFFSET $9
        "#,
        visibility = visibility_clause,
    );

    let items = sqlx::query_as::<_, ApplicationListItem>(&data_query)
    .bind(params.query.as_deref())
    .bind(params.classification_id)
    .bind(params.status.as_deref())
    .bind(params.is_cba)
    .bind(params.deployment_type.as_deref())
    .bind(claims.sub)
    .bind(is_admin)
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

/// Retrieve a single application with full detail including resolved lookups and junction data.
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
    Extension(claims): Extension<Claims>,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<ApplicationFullView>> {
    // Single JOIN query resolving all FK lookups (ADR-0006 Pattern 1)
    let row = sqlx::query_as::<_, ApplicationDetailRow>(
        r#"
        SELECT
            a.application_id, a.application_name, a.application_code, a.description,
            a.classification_id, a.deployment_type, a.technology_stack,
            a.status_id,
            a.business_owner_id, a.technical_owner_id,
            a.steward_user_id, a.approver_user_id, a.organisational_unit,
            a.vendor, a.vendor_product_name, a.version, a.license_type,
            a.abbreviation, a.external_reference_id,
            a.business_capability, a.user_base,
            a.is_cba, a.cba_rationale,
            a.criticality_tier_id, a.risk_rating_id,
            a.data_classification_id, a.regulatory_scope, a.last_security_assessment,
            a.support_model, a.dr_tier_id,
            a.lifecycle_stage_id,
            a.go_live_date, a.retirement_date, a.contract_end_date,
            a.review_frequency_id, a.next_review_date, a.approved_at,
            a.documentation_url,
            a.version_number, a.is_current_version, a.previous_version_id,
            a.created_by, a.updated_by, a.created_at, a.updated_at,
            -- Resolved lookup names
            ac.classification_name,
            es.status_code,
            es.status_name,
            ubo.display_name              AS business_owner_name,
            uto.display_name              AS technical_owner_name,
            ust.display_name              AS steward_name,
            uap.display_name              AS approver_name,
            act.tier_name                 AS criticality_tier_name,
            arr.rating_name               AS risk_rating_name,
            dc.classification_name        AS data_classification_name,
            drt.tier_name                 AS dr_tier_name,
            drt.rto_hours                 AS dr_tier_rto_hours,
            drt.rpo_minutes               AS dr_tier_rpo_minutes,
            als.stage_name                AS lifecycle_stage_name,
            grf.frequency_name            AS review_frequency_name,
            ucb.display_name              AS created_by_name,
            uub.display_name              AS updated_by_name
        FROM applications a
        LEFT JOIN application_classifications ac ON ac.classification_id = a.classification_id
        LEFT JOIN entity_statuses es ON es.status_id = a.status_id
        LEFT JOIN users ubo ON ubo.user_id = a.business_owner_id
        LEFT JOIN users uto ON uto.user_id = a.technical_owner_id
        LEFT JOIN users ust ON ust.user_id = a.steward_user_id
        LEFT JOIN users uap ON uap.user_id = a.approver_user_id
        LEFT JOIN application_criticality_tiers act ON act.tier_id = a.criticality_tier_id
        LEFT JOIN application_risk_ratings arr ON arr.rating_id = a.risk_rating_id
        LEFT JOIN data_classifications dc ON dc.classification_id = a.data_classification_id
        LEFT JOIN disaster_recovery_tiers drt ON drt.dr_tier_id = a.dr_tier_id
        LEFT JOIN application_lifecycle_stages als ON als.stage_id = a.lifecycle_stage_id
        LEFT JOIN glossary_review_frequencies grf ON grf.frequency_id = a.review_frequency_id
        LEFT JOIN users ucb ON ucb.user_id = a.created_by
        LEFT JOIN users uub ON uub.user_id = a.updated_by
        WHERE a.application_id = $1 AND a.deleted_at IS NULL
        "#,
    )
    .bind(app_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("application not found: {app_id}")))?;

    // Visibility check: non-public apps visible only to involved users or admins
    let status_code = row.status_code.as_deref().unwrap_or("DRAFT");
    if !matches!(status_code, "ACCEPTED" | "DEPRECATED") {
        let is_admin = claims.roles.iter().any(|r| r == "ADMIN" || r == "admin");
        let is_involved = row.created_by == claims.sub
            || row.business_owner_id == Some(claims.sub)
            || row.technical_owner_id == Some(claims.sub)
            || row.steward_user_id == Some(claims.sub)
            || row.approver_user_id == Some(claims.sub);
        if !is_admin && !is_involved {
            return Err(AppError::NotFound(format!("application not found: {app_id}")));
        }
    }

    // Separate queries for junction/aggregate data
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
    let application_name = body.application_name.trim().to_string();
    if application_name.is_empty() {
        return Err(AppError::Validation("application_name is required".into()));
    }
    let description = body.description.trim().to_string();
    if description.is_empty() {
        return Err(AppError::Validation("description is required".into()));
    }

    if let Some(ref dt) = body.deployment_type
        && !["ON_PREMISE", "CLOUD", "HYBRID", "SAAS"].contains(&dt.as_str())
    {
        return Err(AppError::Validation(
            "deployment_type must be ON_PREMISE, CLOUD, HYBRID, or SAAS".into(),
        ));
    }

    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    // application_code is NULL → DB trigger auto-generates it
    let application = sqlx::query_as::<_, Application>(
        r#"
        INSERT INTO applications (
            application_name, description,
            classification_id, status_id, vendor, vendor_product_name,
            version, deployment_type, technology_stack,
            is_cba, cba_rationale, go_live_date, documentation_url,
            abbreviation, external_reference_id, license_type,
            lifecycle_stage_id, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        RETURNING *
        "#,
    )
    .bind(&application_name)
    .bind(&description)
    .bind(body.classification_id)
    .bind(draft_status_id)
    .bind(body.vendor.as_deref())
    .bind(body.vendor_product_name.as_deref())
    .bind(body.version.as_deref())
    .bind(body.deployment_type.as_deref())
    .bind(&body.technology_stack)
    .bind(body.is_cba.unwrap_or(false))
    .bind(body.cba_rationale.as_deref())
    .bind(body.go_live_date)
    .bind(body.documentation_url.as_deref())
    .bind(body.abbreviation.as_deref())
    .bind(body.external_reference_id.as_deref())
    .bind(body.license_type.as_deref())
    .bind(body.lifecycle_stage_id)
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

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
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!("application not found: {app_id}")));
    }

    if let Some(ref dt) = body.deployment_type
        && !["ON_PREMISE", "CLOUD", "HYBRID", "SAAS"].contains(&dt.as_str())
    {
        return Err(AppError::Validation(
            "deployment_type must be ON_PREMISE, CLOUD, HYBRID, or SAAS".into(),
        ));
    }

    let application = sqlx::query_as::<_, Application>(
        r#"
        UPDATE applications
        SET application_name       = COALESCE($1, application_name),
            description            = COALESCE($2, description),
            classification_id      = COALESCE($3, classification_id),
            vendor                 = COALESCE($4, vendor),
            vendor_product_name    = COALESCE($5, vendor_product_name),
            version                = COALESCE($6, version),
            deployment_type        = COALESCE($7, deployment_type),
            technology_stack       = COALESCE($8, technology_stack),
            is_cba                 = COALESCE($9, is_cba),
            cba_rationale          = COALESCE($10, cba_rationale),
            go_live_date           = COALESCE($11, go_live_date),
            retirement_date        = COALESCE($12, retirement_date),
            documentation_url      = COALESCE($13, documentation_url),
            abbreviation           = COALESCE($14, abbreviation),
            external_reference_id  = COALESCE($15, external_reference_id),
            business_capability    = COALESCE($16, business_capability),
            user_base              = COALESCE($17, user_base),
            license_type           = COALESCE($18, license_type),
            lifecycle_stage_id     = COALESCE($19, lifecycle_stage_id),
            criticality_tier_id    = COALESCE($20, criticality_tier_id),
            risk_rating_id         = COALESCE($21, risk_rating_id),
            data_classification_id = COALESCE($22, data_classification_id),
            regulatory_scope       = COALESCE($23, regulatory_scope),
            last_security_assessment = COALESCE($24, last_security_assessment),
            support_model          = COALESCE($25, support_model),
            dr_tier_id             = COALESCE($26, dr_tier_id),
            contract_end_date      = COALESCE($27, contract_end_date),
            review_frequency_id    = COALESCE($28, review_frequency_id),
            business_owner_id      = COALESCE($29, business_owner_id),
            technical_owner_id     = COALESCE($30, technical_owner_id),
            steward_user_id        = COALESCE($31, steward_user_id),
            approver_user_id       = COALESCE($32, approver_user_id),
            organisational_unit    = COALESCE($33, organisational_unit),
            updated_by             = $34,
            updated_at             = CURRENT_TIMESTAMP
        WHERE application_id = $35 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(body.application_name.as_deref())
    .bind(body.description.as_deref())
    .bind(body.classification_id)
    .bind(body.vendor.as_deref())
    .bind(body.vendor_product_name.as_deref())
    .bind(body.version.as_deref())
    .bind(body.deployment_type.as_deref())
    .bind(&body.technology_stack)
    .bind(body.is_cba)
    .bind(body.cba_rationale.as_deref())
    .bind(body.go_live_date)
    .bind(body.retirement_date)
    .bind(body.documentation_url.as_deref())
    .bind(body.abbreviation.as_deref())
    .bind(body.external_reference_id.as_deref())
    .bind(body.business_capability.as_deref())
    .bind(body.user_base.as_deref())
    .bind(body.license_type.as_deref())
    .bind(body.lifecycle_stage_id)
    .bind(body.criticality_tier_id)
    .bind(body.risk_rating_id)
    .bind(body.data_classification_id)
    .bind(body.regulatory_scope.as_deref())
    .bind(body.last_security_assessment)
    .bind(body.support_model.as_deref())
    .bind(body.dr_tier_id)
    .bind(body.contract_end_date)
    .bind(body.review_frequency_id)
    .bind(body.business_owner_id)
    .bind(body.technical_owner_id)
    .bind(body.steward_user_id)
    .bind(body.approver_user_id)
    .bind(body.organisational_unit.as_deref())
    .bind(claims.sub)
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(application))
}

// ===========================================================================
// LOOKUP ENDPOINTS
// ===========================================================================

/// List application classification categories.
#[utoipa::path(
    get,
    path = "/api/v1/applications/classifications",
    responses((status = 200, body = Vec<ApplicationClassification>)),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_classifications(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApplicationClassification>>> {
    let rows = sqlx::query_as::<_, ApplicationClassification>(
        "SELECT classification_id, classification_code, classification_name, description FROM application_classifications ORDER BY display_order ASC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// List disaster recovery tiers.
#[utoipa::path(
    get,
    path = "/api/v1/applications/dr-tiers",
    responses((status = 200, body = Vec<DisasterRecoveryTier>)),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_dr_tiers(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DisasterRecoveryTier>>> {
    let rows = sqlx::query_as::<_, DisasterRecoveryTier>(
        "SELECT dr_tier_id, tier_code, tier_name, rto_hours, rpo_minutes, description FROM disaster_recovery_tiers ORDER BY display_order ASC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// List application lifecycle stages.
#[utoipa::path(
    get,
    path = "/api/v1/applications/lifecycle-stages",
    responses((status = 200, body = Vec<ApplicationLifecycleStage>)),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_lifecycle_stages(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApplicationLifecycleStage>>> {
    let rows = sqlx::query_as::<_, ApplicationLifecycleStage>(
        "SELECT stage_id, stage_code, stage_name, description FROM application_lifecycle_stages ORDER BY display_order ASC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// List application criticality tiers.
#[utoipa::path(
    get,
    path = "/api/v1/applications/criticality-tiers",
    responses((status = 200, body = Vec<ApplicationCriticalityTier>)),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_criticality_tiers(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApplicationCriticalityTier>>> {
    let rows = sqlx::query_as::<_, ApplicationCriticalityTier>(
        "SELECT tier_id, tier_code, tier_name, description FROM application_criticality_tiers ORDER BY display_order ASC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// List application risk ratings.
#[utoipa::path(
    get,
    path = "/api/v1/applications/risk-ratings",
    responses((status = 200, body = Vec<ApplicationRiskRating>)),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_risk_ratings(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApplicationRiskRating>>> {
    let rows = sqlx::query_as::<_, ApplicationRiskRating>(
        "SELECT rating_id, rating_code, rating_name, description FROM application_risk_ratings ORDER BY display_order ASC",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

// ===========================================================================
// JUNCTION ENDPOINTS
// ===========================================================================

/// Link a data element to an application.
#[utoipa::path(
    post,
    path = "/api/v1/applications/{app_id}/elements",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    request_body = LinkDataElementRequest,
    responses(
        (status = 201, body = ApplicationDataElementLink),
        (status = 404, description = "Application or element not found")
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
    let app_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;
    if !app_exists {
        return Err(AppError::NotFound(format!("application not found: {app_id}")));
    }

    let element_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM data_elements WHERE element_id = $1 AND deleted_at IS NULL)",
    )
    .bind(body.element_id)
    .fetch_one(&state.pool)
    .await?;
    if !element_exists {
        return Err(AppError::NotFound(format!("data element not found: {}", body.element_id)));
    }

    let usage_type = body.usage_type.as_deref().unwrap_or("BOTH").to_string();
    if !["PRODUCER", "CONSUMER", "BOTH"].contains(&usage_type.as_str()) {
        return Err(AppError::Validation(
            "usage_type must be PRODUCER, CONSUMER, or BOTH".into(),
        ));
    }

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

/// List data elements linked to an application.
#[utoipa::path(
    get,
    path = "/api/v1/applications/{app_id}/elements",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    responses(
        (status = 200, body = Vec<ApplicationDataElementLink>),
        (status = 404, description = "Application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_app_elements(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<Vec<ApplicationDataElementLink>>> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;
    if !exists {
        return Err(AppError::NotFound(format!("application not found: {app_id}")));
    }

    let links = sqlx::query_as::<_, ApplicationDataElementLink>(
        r#"
        SELECT ade.id, ade.application_id, ade.element_id,
               de.element_name, de.element_code,
               ade.usage_type, ade.is_authoritative_source,
               ade.description, ade.created_at
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

/// List interfaces (data exchanges) for an application.
#[utoipa::path(
    get,
    path = "/api/v1/applications/{app_id}/interfaces",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    responses(
        (status = 200, body = Vec<ApplicationInterface>),
        (status = 404, description = "Application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_interfaces(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
) -> AppResult<Json<Vec<ApplicationInterface>>> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(app_id)
    .fetch_one(&state.pool)
    .await?;
    if !exists {
        return Err(AppError::NotFound(format!("application not found: {app_id}")));
    }

    let interfaces = sqlx::query_as::<_, ApplicationInterface>(
        r#"
        SELECT interface_id, source_app_id, target_app_id,
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

// ---------------------------------------------------------------------------
// amend_application — POST /api/v1/applications/:app_id/amend
// ---------------------------------------------------------------------------

/// Create a new draft amendment of an accepted application.
/// Copies all fields to a new row with incremented version_number.
#[utoipa::path(
    post,
    path = "/api/v1/applications/{app_id}/amend",
    params(("app_id" = Uuid, Path, description = "Application ID of the accepted application to amend")),
    responses(
        (status = 201, description = "Amendment created in DRAFT status", body = Application),
        (status = 200, description = "Existing amendment returned", body = Application),
        (status = 422, description = "Application is not in ACCEPTED status")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn amend_application(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(app_id): Path<Uuid>,
) -> AppResult<(StatusCode, Json<Application>)> {
    // Verify the application exists
    let original = sqlx::query_as::<_, Application>(
        "SELECT * FROM applications WHERE application_id = $1 AND deleted_at IS NULL",
    )
    .bind(app_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("application not found: {app_id}")))?;

    // Check status is ACCEPTED
    let status_code = sqlx::query_scalar::<_, String>(
        "SELECT status_code FROM entity_statuses WHERE status_id = $1",
    )
    .bind(original.status_id)
    .fetch_one(&state.pool)
    .await?;

    if status_code != "ACCEPTED" {
        return Err(AppError::Validation(
            "only accepted applications can be amended".into(),
        ));
    }

    // If an amendment already exists, return it instead of creating a new one
    let existing_amendment = sqlx::query_as::<_, Application>(
        "SELECT * FROM applications WHERE previous_version_id = $1 AND deleted_at IS NULL AND is_current_version = FALSE LIMIT 1",
    )
    .bind(app_id)
    .fetch_optional(&state.pool)
    .await?;

    if let Some(existing) = existing_amendment {
        return Ok((StatusCode::OK, Json(existing)));
    }

    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    let new_version = original.version_number + 1;

    // Insert new version with all fields copied, new application_id, DRAFT status
    let amendment = sqlx::query_as::<_, Application>(
        r#"
        INSERT INTO applications (
            application_name, application_code, description,
            classification_id, deployment_type, technology_stack,
            status_id,
            business_owner_id, technical_owner_id,
            steward_user_id, approver_user_id, organisational_unit,
            vendor, vendor_product_name, version, license_type,
            abbreviation, external_reference_id,
            business_capability, user_base,
            is_cba, cba_rationale,
            criticality_tier_id, risk_rating_id,
            data_classification_id, regulatory_scope, last_security_assessment,
            support_model, dr_tier_id,
            lifecycle_stage_id,
            go_live_date, retirement_date, contract_end_date,
            review_frequency_id,
            documentation_url,
            version_number, is_current_version, previous_version_id,
            created_by
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27, $28, $29, $30,
            $31, $32, $33, $34, $35, $36, FALSE, $37, $38
        )
        RETURNING *
        "#,
    )
    .bind(&original.application_name)           // $1
    .bind(&original.application_code)           // $2 — same code, new version
    .bind(&original.description)                // $3
    .bind(original.classification_id)           // $4
    .bind(original.deployment_type.as_deref())  // $5
    .bind(&original.technology_stack)           // $6
    .bind(draft_status_id)                      // $7
    .bind(original.business_owner_id)           // $8
    .bind(original.technical_owner_id)          // $9
    .bind(original.steward_user_id)             // $10
    .bind(original.approver_user_id)            // $11
    .bind(original.organisational_unit.as_deref()) // $12
    .bind(original.vendor.as_deref())           // $13
    .bind(original.vendor_product_name.as_deref()) // $14
    .bind(original.version.as_deref())          // $15
    .bind(original.license_type.as_deref())     // $16
    .bind(original.abbreviation.as_deref())     // $17
    .bind(original.external_reference_id.as_deref()) // $18
    .bind(original.business_capability.as_deref()) // $19
    .bind(original.user_base.as_deref())        // $20
    .bind(original.is_cba)                      // $21
    .bind(original.cba_rationale.as_deref())    // $22
    .bind(original.criticality_tier_id)         // $23
    .bind(original.risk_rating_id)              // $24
    .bind(original.data_classification_id)      // $25
    .bind(original.regulatory_scope.as_deref()) // $26
    .bind(original.last_security_assessment)    // $27
    .bind(original.support_model.as_deref())    // $28
    .bind(original.dr_tier_id)                  // $29
    .bind(original.lifecycle_stage_id)          // $30
    .bind(original.go_live_date)                // $31
    .bind(original.retirement_date)             // $32
    .bind(original.contract_end_date)           // $33
    .bind(original.review_frequency_id)         // $34
    .bind(original.documentation_url.as_deref()) // $35
    .bind(new_version)                          // $36
    .bind(app_id)                               // $37 = previous_version_id
    .bind(claims.sub)                           // $38 = created_by
    .fetch_one(&state.pool)
    .await?;

    // Initiate workflow for the amendment
    workflow::service::initiate_workflow(
        &state.pool,
        workflow::ENTITY_APPLICATION,
        amendment.application_id,
        claims.sub,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(amendment)))
}

// ---------------------------------------------------------------------------
// discard_amendment — DELETE /api/v1/applications/:app_id/discard
// ---------------------------------------------------------------------------

/// Discard a draft application amendment. Only the creator or admin can discard,
/// and only in DRAFT status. Hard deletes the amendment (never-submitted drafts
/// have no governance value).
#[utoipa::path(
    delete,
    path = "/api/v1/applications/{app_id}/discard",
    params(("app_id" = Uuid, Path, description = "Amendment application ID to discard")),
    responses(
        (status = 204, description = "Amendment discarded"),
        (status = 403, description = "Only the creator can discard"),
        (status = 422, description = "Application is not a draft amendment")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn discard_amendment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(app_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    // Fetch the application
    let row = sqlx::query_as::<_, Application>(
        "SELECT * FROM applications WHERE application_id = $1 AND deleted_at IS NULL",
    )
    .bind(app_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("application not found: {app_id}")))?;

    // Must be an amendment (has previous_version_id)
    if row.previous_version_id.is_none() {
        return Err(AppError::Validation(
            "only amendments can be discarded — use the workflow to manage original applications".into(),
        ));
    }

    // Must be in DRAFT status
    let status_code = sqlx::query_scalar::<_, String>(
        "SELECT status_code FROM entity_statuses WHERE status_id = $1",
    )
    .bind(row.status_id)
    .fetch_one(&state.pool)
    .await?;

    if status_code != "DRAFT" {
        return Err(AppError::Validation(
            "only draft amendments can be discarded — submitted amendments must be rejected through the workflow".into(),
        ));
    }

    // Only the creator or admin can discard
    let is_admin = claims.roles.iter().any(|r| r == "ADMIN" || r == "admin");
    if row.created_by != claims.sub && !is_admin {
        return Err(AppError::Forbidden(
            "only the amendment creator or an admin can discard it".into(),
        ));
    }

    // Hard delete: a never-submitted draft has no governance value.

    // Delete workflow tasks and history, then the instance
    sqlx::query(
        r#"
        DELETE FROM workflow_tasks
        WHERE instance_id IN (SELECT instance_id FROM workflow_instances WHERE entity_id = $1)
        "#,
    )
    .bind(app_id)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM workflow_history
        WHERE instance_id IN (SELECT instance_id FROM workflow_instances WHERE entity_id = $1)
        "#,
    )
    .bind(app_id)
    .execute(&state.pool)
    .await?;

    sqlx::query("DELETE FROM workflow_instances WHERE entity_id = $1")
        .bind(app_id).execute(&state.pool).await?;

    // Delete the amendment application itself
    sqlx::query("DELETE FROM applications WHERE application_id = $1")
        .bind(app_id).execute(&state.pool).await?;

    Ok(StatusCode::NO_CONTENT)
}
