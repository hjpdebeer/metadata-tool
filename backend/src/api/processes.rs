use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::glossary::PaginatedResponse;
use crate::domain::processes::*;
use crate::error::{AppError, AppResult};
use crate::workflow;

// ---------------------------------------------------------------------------
// list_processes — GET /api/v1/processes
// ---------------------------------------------------------------------------

/// List business processes with optional filtering and pagination.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/processes",
    params(SearchProcessesRequest),
    responses(
        (status = 200, description = "Paginated list of business processes",
         body = PaginatedBusinessProcesses)
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_processes(
    State(state): State<AppState>,
    Query(params): Query<SearchProcessesRequest>,
) -> AppResult<Json<PaginatedResponse<BusinessProcessListItem>>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Count query — mirrors the same WHERE conditions as the data query
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM business_processes bp
        JOIN entity_statuses es ON es.status_id = bp.status_id
        WHERE bp.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR bp.process_name ILIKE '%' || $1 || '%'
               OR bp.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR bp.category_id = $2)
          AND ($3::TEXT IS NULL OR es.status_code = $3)
          AND ($4::BOOL IS NULL OR bp.is_critical = $4)
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.category_id)
    .bind(params.status.as_deref())
    .bind(params.is_critical)
    .fetch_one(&state.pool)
    .await?;

    // Data query with joins for display fields
    let items = sqlx::query_as::<_, BusinessProcessListItem>(
        r#"
        SELECT
            bp.process_id,
            bp.process_name,
            bp.process_code,
            bp.description,
            pc.category_name              AS category_name,
            es.status_code                AS status_code,
            es.status_name                AS status_name,
            uo.display_name               AS owner_name,
            bp.is_critical,
            bp.frequency,
            pp.process_name               AS parent_process_name,
            bp.created_at,
            bp.updated_at
        FROM business_processes bp
        JOIN entity_statuses es ON es.status_id = bp.status_id
        LEFT JOIN process_categories pc ON pc.category_id = bp.category_id
        LEFT JOIN users uo ON uo.user_id = bp.owner_user_id
        LEFT JOIN business_processes pp ON pp.process_id = bp.parent_process_id
        WHERE bp.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR bp.process_name ILIKE '%' || $1 || '%'
               OR bp.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR bp.category_id = $2)
          AND ($3::TEXT IS NULL OR es.status_code = $3)
          AND ($4::BOOL IS NULL OR bp.is_critical = $4)
        ORDER BY bp.process_name ASC
        LIMIT $5
        OFFSET $6
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.category_id)
    .bind(params.status.as_deref())
    .bind(params.is_critical)
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
// get_process — GET /api/v1/processes/:process_id
// ---------------------------------------------------------------------------

/// Retrieve a single business process with full detail including steps and sub-processes.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/processes/{process_id}",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    responses(
        (status = 200, description = "Process details", body = BusinessProcessFullView),
        (status = 404, description = "Process not found")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn get_process(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> AppResult<Json<BusinessProcessFullView>> {
    // Fetch the main process record
    let process = sqlx::query_as::<_, BusinessProcess>(
        r#"
        SELECT
            process_id, process_name, process_code, description,
            detailed_description, category_id, status_id, owner_user_id,
            parent_process_id, is_critical, criticality_rationale,
            frequency, regulatory_requirement, sla_description,
            documentation_url, created_by, updated_by, created_at, updated_at
        FROM business_processes
        WHERE process_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(process_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("business process not found: {process_id}")))?;

    // Fetch owner name
    let owner_name: Option<String> = if let Some(owner_id) = process.owner_user_id {
        sqlx::query_scalar::<_, String>(
            "SELECT display_name FROM users WHERE user_id = $1",
        )
        .bind(owner_id)
        .fetch_optional(&state.pool)
        .await?
    } else {
        None
    };

    // Fetch category name
    let category_name: Option<String> = if let Some(cat_id) = process.category_id {
        sqlx::query_scalar::<_, String>(
            "SELECT category_name FROM process_categories WHERE category_id = $1",
        )
        .bind(cat_id)
        .fetch_optional(&state.pool)
        .await?
    } else {
        None
    };

    // Fetch parent process name
    let parent_process_name: Option<String> = if let Some(parent_id) = process.parent_process_id {
        sqlx::query_scalar::<_, String>(
            "SELECT process_name FROM business_processes WHERE process_id = $1",
        )
        .bind(parent_id)
        .fetch_optional(&state.pool)
        .await?
    } else {
        None
    };

    // Fetch process steps
    let steps = sqlx::query_as::<_, ProcessStep>(
        r#"
        SELECT
            step_id, process_id, step_number, step_name,
            description, responsible_role, application_id,
            input_data_elements, output_data_elements
        FROM process_steps
        WHERE process_id = $1
        ORDER BY step_number ASC
        "#,
    )
    .bind(process_id)
    .fetch_all(&state.pool)
    .await?;

    // Count linked data elements
    let data_elements_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM process_data_elements WHERE process_id = $1",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    // Fetch linked application names
    let linked_applications = sqlx::query_scalar::<_, String>(
        r#"
        SELECT a.application_name
        FROM process_applications pa
        JOIN applications a ON a.application_id = pa.application_id
        WHERE pa.process_id = $1 AND a.deleted_at IS NULL
        ORDER BY a.application_name ASC
        "#,
    )
    .bind(process_id)
    .fetch_all(&state.pool)
    .await?;

    // Fetch sub-processes
    let sub_processes = sqlx::query_as::<_, BusinessProcess>(
        r#"
        SELECT
            process_id, process_name, process_code, description,
            detailed_description, category_id, status_id, owner_user_id,
            parent_process_id, is_critical, criticality_rationale,
            frequency, regulatory_requirement, sla_description,
            documentation_url, created_by, updated_by, created_at, updated_at
        FROM business_processes
        WHERE parent_process_id = $1 AND deleted_at IS NULL
        ORDER BY process_name ASC
        "#,
    )
    .bind(process_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(BusinessProcessFullView {
        process,
        owner_name,
        category_name,
        parent_process_name,
        steps,
        data_elements_count,
        linked_applications,
        sub_processes,
    }))
}

// ---------------------------------------------------------------------------
// create_process — POST /api/v1/processes
// ---------------------------------------------------------------------------

/// Create a new business process in DRAFT status with an associated workflow instance.
/// Requires authentication. Critical processes trigger auto-CDE designation via DB trigger (Principle 12).
#[utoipa::path(
    post,
    path = "/api/v1/processes",
    request_body = CreateBusinessProcessRequest,
    responses(
        (status = 201, description = "Process created", body = BusinessProcess),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn create_process(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateBusinessProcessRequest>,
) -> AppResult<(StatusCode, Json<BusinessProcess>)> {
    // Validate required fields
    let process_name = body.process_name.trim().to_string();
    if process_name.is_empty() {
        return Err(AppError::Validation("process_name is required".into()));
    }
    let process_code = body.process_code.trim().to_string();
    if process_code.is_empty() {
        return Err(AppError::Validation("process_code is required".into()));
    }
    let description = body.description.trim().to_string();
    if description.is_empty() {
        return Err(AppError::Validation("description is required".into()));
    }

    // Look up DRAFT status_id from entity_statuses
    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    // Insert the new business process
    // NOTE: if is_critical=true, the DB trigger (trg_critical_process_cde)
    // will auto-designate linked elements as CDEs when elements are linked later
    let process = sqlx::query_as::<_, BusinessProcess>(
        r#"
        INSERT INTO business_processes (
            process_name, process_code, description, detailed_description,
            category_id, status_id, parent_process_id,
            is_critical, criticality_rationale, frequency,
            regulatory_requirement, sla_description, documentation_url,
            created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING
            process_id, process_name, process_code, description,
            detailed_description, category_id, status_id, owner_user_id,
            parent_process_id, is_critical, criticality_rationale,
            frequency, regulatory_requirement, sla_description,
            documentation_url, created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(&process_name)
    .bind(&process_code)
    .bind(&description)
    .bind(body.detailed_description.as_deref())
    .bind(body.category_id)
    .bind(draft_status_id)
    .bind(body.parent_process_id)
    .bind(body.is_critical.unwrap_or(false))
    .bind(body.criticality_rationale.as_deref())
    .bind(body.frequency.as_deref())
    .bind(body.regulatory_requirement.as_deref())
    .bind(body.sla_description.as_deref())
    .bind(body.documentation_url.as_deref())
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    // Initiate the workflow instance for this new business process
    workflow::service::initiate_workflow(
        &state.pool,
        workflow::ENTITY_BUSINESS_PROCESS,
        process.process_id,
        claims.sub,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(process)))
}

// ---------------------------------------------------------------------------
// update_process — PUT /api/v1/processes/:process_id
// ---------------------------------------------------------------------------

/// Update an existing business process. Only provided fields are changed.
/// Requires authentication. CDE propagation is handled by DB triggers (Principle 12).
#[utoipa::path(
    put,
    path = "/api/v1/processes/{process_id}",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    request_body = UpdateBusinessProcessRequest,
    responses(
        (status = 200, description = "Process updated", body = BusinessProcess),
        (status = 404, description = "Process not found")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn update_process(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateBusinessProcessRequest>,
) -> AppResult<Json<BusinessProcess>> {
    // Verify the process exists and is not deleted
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM business_processes WHERE process_id = $1 AND deleted_at IS NULL)",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "business process not found: {process_id}"
        )));
    }

    // Update using COALESCE to only change provided fields
    // NOTE: if is_critical changes to true, the DB trigger
    // (trg_critical_process_cde) automatically handles CDE propagation
    let process = sqlx::query_as::<_, BusinessProcess>(
        r#"
        UPDATE business_processes
        SET process_name          = COALESCE($1, process_name),
            description           = COALESCE($2, description),
            detailed_description  = COALESCE($3, detailed_description),
            category_id           = COALESCE($4, category_id),
            parent_process_id     = COALESCE($5, parent_process_id),
            is_critical           = COALESCE($6, is_critical),
            criticality_rationale = COALESCE($7, criticality_rationale),
            frequency             = COALESCE($8, frequency),
            regulatory_requirement = COALESCE($9, regulatory_requirement),
            sla_description       = COALESCE($10, sla_description),
            documentation_url     = COALESCE($11, documentation_url),
            updated_by            = $12,
            updated_at            = CURRENT_TIMESTAMP
        WHERE process_id = $13 AND deleted_at IS NULL
        RETURNING
            process_id, process_name, process_code, description,
            detailed_description, category_id, status_id, owner_user_id,
            parent_process_id, is_critical, criticality_rationale,
            frequency, regulatory_requirement, sla_description,
            documentation_url, created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(body.process_name.as_deref())
    .bind(body.description.as_deref())
    .bind(body.detailed_description.as_deref())
    .bind(body.category_id)
    .bind(body.parent_process_id)
    .bind(body.is_critical)
    .bind(body.criticality_rationale.as_deref())
    .bind(body.frequency.as_deref())
    .bind(body.regulatory_requirement.as_deref())
    .bind(body.sla_description.as_deref())
    .bind(body.documentation_url.as_deref())
    .bind(claims.sub)
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(process))
}

// ---------------------------------------------------------------------------
// list_critical_processes — GET /api/v1/processes/critical
// ---------------------------------------------------------------------------

/// List all critical business processes with their data element counts (Principle 12).
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/processes/critical",
    responses(
        (status = 200, description = "List critical business processes with data element counts",
         body = Vec<CriticalProcessSummary>)
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_critical_processes(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<CriticalProcessSummary>>> {
    let processes = sqlx::query_as::<_, CriticalProcessSummary>(
        r#"
        SELECT
            bp.process_id,
            bp.process_name,
            bp.process_code,
            bp.description,
            pc.category_name              AS category_name,
            uo.display_name               AS owner_name,
            bp.frequency,
            COALESCE(pde_counts.cnt, 0)   AS data_elements_count
        FROM business_processes bp
        LEFT JOIN process_categories pc ON pc.category_id = bp.category_id
        LEFT JOIN users uo ON uo.user_id = bp.owner_user_id
        LEFT JOIN (
            SELECT process_id, COUNT(*) AS cnt
            FROM process_data_elements
            GROUP BY process_id
        ) pde_counts ON pde_counts.process_id = bp.process_id
        WHERE bp.is_critical = TRUE AND bp.deleted_at IS NULL
        ORDER BY bp.process_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(processes))
}

// ---------------------------------------------------------------------------
// list_categories — GET /api/v1/processes/categories
// ---------------------------------------------------------------------------

/// List all process categories.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/processes/categories",
    responses(
        (status = 200, description = "List process categories",
         body = Vec<ProcessCategory>)
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_categories(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ProcessCategory>>> {
    let categories = sqlx::query_as::<_, ProcessCategory>(
        r#"
        SELECT category_id, category_name, description, parent_category_id
        FROM process_categories
        ORDER BY category_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(categories))
}

// ---------------------------------------------------------------------------
// add_step — POST /api/v1/processes/:process_id/steps
// ---------------------------------------------------------------------------

/// Add a step to a business process.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/processes/{process_id}/steps",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    request_body = CreateProcessStepRequest,
    responses(
        (status = 201, description = "Process step added", body = ProcessStep),
        (status = 404, description = "Process not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn add_step(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<CreateProcessStepRequest>,
) -> AppResult<(StatusCode, Json<ProcessStep>)> {
    // Verify the process exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM business_processes WHERE process_id = $1 AND deleted_at IS NULL)",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "business process not found: {process_id}"
        )));
    }

    // Validate required fields
    let step_name = body.step_name.trim().to_string();
    if step_name.is_empty() {
        return Err(AppError::Validation("step_name is required".into()));
    }

    // Insert the step
    let step = sqlx::query_as::<_, ProcessStep>(
        r#"
        INSERT INTO process_steps (
            process_id, step_number, step_name, description,
            responsible_role, application_id
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            step_id, process_id, step_number, step_name,
            description, responsible_role, application_id,
            input_data_elements, output_data_elements
        "#,
    )
    .bind(process_id)
    .bind(body.step_number)
    .bind(&step_name)
    .bind(body.description.as_deref())
    .bind(body.responsible_role.as_deref())
    .bind(body.application_id)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(step)))
}

// ---------------------------------------------------------------------------
// list_steps — GET /api/v1/processes/:process_id/steps
// ---------------------------------------------------------------------------

/// List steps for a business process, ordered by step number.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/processes/{process_id}/steps",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    responses(
        (status = 200, description = "List steps for a process",
         body = Vec<ProcessStep>),
        (status = 404, description = "Process not found")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_steps(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> AppResult<Json<Vec<ProcessStep>>> {
    // Verify the process exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM business_processes WHERE process_id = $1 AND deleted_at IS NULL)",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "business process not found: {process_id}"
        )));
    }

    let steps = sqlx::query_as::<_, ProcessStep>(
        r#"
        SELECT
            step_id, process_id, step_number, step_name,
            description, responsible_role, application_id,
            input_data_elements, output_data_elements
        FROM process_steps
        WHERE process_id = $1
        ORDER BY step_number ASC
        "#,
    )
    .bind(process_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(steps))
}

// ---------------------------------------------------------------------------
// link_data_element — POST /api/v1/processes/:process_id/elements
// ---------------------------------------------------------------------------

/// Link a data element to a business process. Auto-CDE designation via DB trigger for critical processes (Principle 12).
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/processes/{process_id}/elements",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    request_body = LinkProcessDataElementRequest,
    responses(
        (status = 201, description = "Data element linked to process",
         body = ProcessDataElementLink),
        (status = 404, description = "Process or element not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn link_data_element(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<LinkProcessDataElementRequest>,
) -> AppResult<(StatusCode, Json<ProcessDataElementLink>)> {
    // Verify the process exists
    let process_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM business_processes WHERE process_id = $1 AND deleted_at IS NULL)",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    if !process_exists {
        return Err(AppError::NotFound(format!(
            "business process not found: {process_id}"
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
    if !["INPUT", "OUTPUT", "BOTH"].contains(&usage_type.as_str()) {
        return Err(AppError::Validation(
            "usage_type must be INPUT, OUTPUT, or BOTH".into(),
        ));
    }

    // Insert the link
    // NOTE: The DB trigger (trg_process_link_cde) fires on INSERT into
    // process_data_elements — if the process is critical, the linked element
    // will be automatically designated as a CDE
    let link = sqlx::query_as::<_, ProcessDataElementLink>(
        r#"
        INSERT INTO process_data_elements (
            process_id, element_id, usage_type, is_required, description
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id, process_id, element_id,
            (SELECT element_name FROM data_elements WHERE element_id = $2) AS element_name,
            (SELECT element_code FROM data_elements WHERE element_id = $2) AS element_code,
            (SELECT is_cde FROM data_elements WHERE element_id = $2) AS is_cde,
            usage_type, is_required, description, created_at
        "#,
    )
    .bind(process_id)
    .bind(body.element_id)
    .bind(&usage_type)
    .bind(body.is_required.unwrap_or(true))
    .bind(body.description.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(link)))
}

// ---------------------------------------------------------------------------
// list_process_elements — GET /api/v1/processes/:process_id/elements
// ---------------------------------------------------------------------------

/// List data elements linked to a business process, including CDE indicator.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/processes/{process_id}/elements",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    responses(
        (status = 200, description = "List data elements linked to process (includes is_cde indicator)",
         body = Vec<ProcessDataElementLink>),
        (status = 404, description = "Process not found")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_process_elements(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> AppResult<Json<Vec<ProcessDataElementLink>>> {
    // Verify the process exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM business_processes WHERE process_id = $1 AND deleted_at IS NULL)",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "business process not found: {process_id}"
        )));
    }

    let links = sqlx::query_as::<_, ProcessDataElementLink>(
        r#"
        SELECT
            pde.id,
            pde.process_id,
            pde.element_id,
            de.element_name,
            de.element_code,
            de.is_cde,
            pde.usage_type,
            pde.is_required,
            pde.description,
            pde.created_at
        FROM process_data_elements pde
        JOIN data_elements de ON de.element_id = pde.element_id
        WHERE pde.process_id = $1
        ORDER BY de.element_name ASC
        "#,
    )
    .bind(process_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(links))
}

// ---------------------------------------------------------------------------
// link_application — POST /api/v1/processes/:process_id/applications
// ---------------------------------------------------------------------------

/// Link an application to a business process with an optional role description.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/processes/{process_id}/applications",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    request_body = LinkProcessApplicationRequest,
    responses(
        (status = 201, description = "Application linked to process",
         body = ProcessApplicationLink),
        (status = 404, description = "Process or application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn link_application(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<LinkProcessApplicationRequest>,
) -> AppResult<(StatusCode, Json<ProcessApplicationLink>)> {
    // Verify the process exists
    let process_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM business_processes WHERE process_id = $1 AND deleted_at IS NULL)",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    if !process_exists {
        return Err(AppError::NotFound(format!(
            "business process not found: {process_id}"
        )));
    }

    // Verify the application exists
    let app_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM applications WHERE application_id = $1 AND deleted_at IS NULL)",
    )
    .bind(body.application_id)
    .fetch_one(&state.pool)
    .await?;

    if !app_exists {
        return Err(AppError::NotFound(format!(
            "application not found: {}",
            body.application_id
        )));
    }

    // Insert the link
    let link = sqlx::query_as::<_, ProcessApplicationLink>(
        r#"
        INSERT INTO process_applications (
            process_id, application_id, role_in_process, description
        )
        VALUES ($1, $2, $3, $4)
        RETURNING
            id, process_id, application_id,
            (SELECT application_name FROM applications WHERE application_id = $2) AS application_name,
            (SELECT application_code FROM applications WHERE application_id = $2) AS application_code,
            role_in_process, description, created_at
        "#,
    )
    .bind(process_id)
    .bind(body.application_id)
    .bind(body.role_in_process.as_deref())
    .bind(body.description.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(link)))
}

// ---------------------------------------------------------------------------
// list_process_applications — GET /api/v1/processes/:process_id/applications
// ---------------------------------------------------------------------------

/// List applications linked to a business process.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/processes/{process_id}/applications",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    responses(
        (status = 200, description = "List applications linked to process",
         body = Vec<ProcessApplicationLink>),
        (status = 404, description = "Process not found")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_process_applications(
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> AppResult<Json<Vec<ProcessApplicationLink>>> {
    // Verify the process exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM business_processes WHERE process_id = $1 AND deleted_at IS NULL)",
    )
    .bind(process_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "business process not found: {process_id}"
        )));
    }

    let links = sqlx::query_as::<_, ProcessApplicationLink>(
        r#"
        SELECT
            pa.id,
            pa.process_id,
            pa.application_id,
            a.application_name,
            a.application_code,
            pa.role_in_process,
            pa.description,
            pa.created_at
        FROM process_applications pa
        JOIN applications a ON a.application_id = pa.application_id
        WHERE pa.process_id = $1
        ORDER BY a.application_name ASC
        "#,
    )
    .bind(process_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(links))
}
