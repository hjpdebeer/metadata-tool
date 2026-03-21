use axum::Extension;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::data_dictionary::*;
use crate::domain::glossary::PaginatedResponse;
use crate::error::{AppError, AppResult};
use crate::naming;
use crate::workflow;

/// All column names for SELECT in the data_elements table (used by RETURNING clauses)
const DATA_ELEMENT_COLUMNS: &str = r#"
    element_id, element_name, element_code, description,
    business_definition, business_rules, data_type,
    max_length, numeric_precision, numeric_scale,
    format_pattern,
    allowed_values, default_value, is_nullable, is_cde,
    cde_rationale, cde_designated_at, glossary_term_id,
    domain_id, classification_id,
    status_id, owner_user_id, steward_user_id,
    approver_user_id, organisational_unit,
    review_frequency_id, next_review_date, approved_at,
    is_pii, version_number, is_current_version, previous_version_id,
    created_by, updated_by, created_at, updated_at
"#;

// ---------------------------------------------------------------------------
// list_elements — GET /api/v1/data-dictionary/elements
// ---------------------------------------------------------------------------

/// List data elements with optional filtering, pagination, and visibility filtering.
/// Requires authentication. Supports full-text search via `query` parameter.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/elements",
    params(SearchDataElementsRequest),
    responses(
        (status = 200, description = "Paginated list of data elements",
         body = PaginatedDataElements)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_elements(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<SearchDataElementsRequest>,
) -> AppResult<Json<PaginatedResponse<DataElementListItem>>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let is_admin = claims.roles.iter().any(|r| r == "ADMIN" || r == "admin");

    // Visibility + version filter (same pattern as glossary/applications):
    // - Current versions: ACCEPTED/DEPRECATED visible to all, others to involved users/admins.
    // - Non-current versions (amendments): visible only to involved users/admins,
    //   excluding SUPERSEDED/REJECTED.
    let visibility_clause = r#"
          AND (
              (de.is_current_version = TRUE AND (
                  es.status_code IN ('ACCEPTED', 'DEPRECATED')
                  OR de.created_by = $7
                  OR de.owner_user_id = $7
                  OR de.steward_user_id = $7
                  OR de.approver_user_id = $7
                  OR $8::BOOLEAN = TRUE
              ))
              OR (de.is_current_version = FALSE AND es.status_code NOT IN ('SUPERSEDED', 'REJECTED') AND (
                  de.created_by = $7
                  OR de.owner_user_id = $7
                  OR de.steward_user_id = $7
                  OR de.approver_user_id = $7
                  OR $8::BOOLEAN = TRUE
              ))
          )
    "#;

    // Count query — mirrors the same WHERE conditions as the data query
    let count_query = format!(
        r#"
        SELECT COUNT(*)
        FROM data_elements de
        JOIN entity_statuses es ON es.status_id = de.status_id
        WHERE de.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR de.search_vector @@ plainto_tsquery('english', $1))
          AND ($2::UUID IS NULL OR de.domain_id = $2)
          AND ($3::BOOL IS NULL OR de.is_cde = $3)
          AND ($4::TEXT IS NULL OR es.status_code = $4)
          AND ($5::UUID IS NULL OR de.glossary_term_id = $5)
          AND ($6::UUID IS NULL OR de.classification_id = $6)
          {visibility}
        "#,
        visibility = visibility_clause,
    );

    let total_count = sqlx::query_scalar::<_, i64>(&count_query)
        .bind(params.query.as_deref())
        .bind(params.domain_id)
        .bind(params.is_cde)
        .bind(params.status.as_deref())
        .bind(params.glossary_term_id)
        .bind(params.classification_id)
        .bind(claims.sub)
        .bind(is_admin)
        .fetch_one(&state.pool)
        .await?;

    // Data query with joins for display fields
    let data_query = format!(
        r#"
        SELECT
            de.element_id,
            de.element_name,
            de.element_code,
            de.description,
            de.data_type,
            de.is_cde,
            gd.domain_name                AS domain_name,
            dc.classification_name        AS classification_name,
            es.status_code                AS status_code,
            es.status_name                AS status_name,
            uo.display_name               AS owner_name,
            gt.term_name                  AS glossary_term_name,
            de.created_at,
            de.updated_at
        FROM data_elements de
        JOIN entity_statuses es ON es.status_id = de.status_id
        LEFT JOIN glossary_domains gd ON gd.domain_id = de.domain_id
        LEFT JOIN data_classifications dc ON dc.classification_id = de.classification_id
        LEFT JOIN users uo ON uo.user_id = de.owner_user_id
        LEFT JOIN glossary_terms gt ON gt.term_id = de.glossary_term_id
        WHERE de.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR de.search_vector @@ plainto_tsquery('english', $1))
          AND ($2::UUID IS NULL OR de.domain_id = $2)
          AND ($3::BOOL IS NULL OR de.is_cde = $3)
          AND ($4::TEXT IS NULL OR es.status_code = $4)
          AND ($5::UUID IS NULL OR de.glossary_term_id = $5)
          AND ($6::UUID IS NULL OR de.classification_id = $6)
          {visibility}
        ORDER BY de.element_name ASC, de.version_number DESC
        LIMIT $9
        OFFSET $10
        "#,
        visibility = visibility_clause,
    );

    let items = sqlx::query_as::<_, DataElementListItem>(&data_query)
        .bind(params.query.as_deref())
        .bind(params.domain_id)
        .bind(params.is_cde)
        .bind(params.status.as_deref())
        .bind(params.glossary_term_id)
        .bind(params.classification_id)
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
// get_element — GET /api/v1/data-dictionary/elements/:element_id
// ---------------------------------------------------------------------------

/// Retrieve a single data element with full detail including linked technical columns.
/// Requires authentication. Includes visibility check.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/elements/{element_id}",
    params(("element_id" = Uuid, Path, description = "Element ID")),
    responses(
        (status = 200, description = "Full data element view", body = DataElementFullView),
        (status = 404, description = "Element not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn get_element(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(element_id): Path<Uuid>,
) -> AppResult<Json<DataElementFullView>> {
    // Single JOIN query resolving all FK lookups (ADR-0006 Pattern 1)
    let row = sqlx::query_as::<_, DataElementDetailRow>(
        r#"
        SELECT
            de.element_id, de.element_name, de.element_code, de.description,
            de.business_definition, de.business_rules, de.data_type,
            de.max_length, de.numeric_precision, de.numeric_scale,
            de.format_pattern,
            de.allowed_values, de.default_value, de.is_nullable, de.is_cde,
            de.cde_rationale, de.cde_designated_at, de.glossary_term_id,
            de.domain_id, de.classification_id,
            de.status_id, de.owner_user_id, de.steward_user_id,
            de.approver_user_id, de.organisational_unit,
            de.review_frequency_id, de.next_review_date, de.approved_at,
            de.is_pii, de.version_number, de.is_current_version, de.previous_version_id,
            de.created_by, de.updated_by, de.created_at, de.updated_at,
            gt.term_name                  AS glossary_term_name,
            gd.domain_name,
            dc.classification_name,
            uo.display_name               AS owner_name,
            us.display_name               AS steward_name,
            uap.display_name              AS approver_name,
            grf.frequency_name            AS review_frequency_name,
            es.status_code,
            es.status_name,
            ucb.display_name              AS created_by_name,
            uub.display_name              AS updated_by_name,
            wi.instance_id                AS workflow_instance_id
        FROM data_elements de
        LEFT JOIN glossary_terms gt ON gt.term_id = de.glossary_term_id
        LEFT JOIN glossary_domains gd ON gd.domain_id = de.domain_id
        LEFT JOIN data_classifications dc ON dc.classification_id = de.classification_id
        LEFT JOIN users uo ON uo.user_id = de.owner_user_id
        LEFT JOIN users us ON us.user_id = de.steward_user_id
        LEFT JOIN users uap ON uap.user_id = de.approver_user_id
        LEFT JOIN glossary_review_frequencies grf ON grf.frequency_id = de.review_frequency_id
        LEFT JOIN entity_statuses es ON es.status_id = de.status_id
        LEFT JOIN users ucb ON ucb.user_id = de.created_by
        LEFT JOIN users uub ON uub.user_id = de.updated_by
        LEFT JOIN workflow_instances wi ON wi.entity_id = de.element_id
            AND wi.completed_at IS NULL
        WHERE de.element_id = $1 AND de.deleted_at IS NULL
        "#,
    )
    .bind(element_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("data element not found: {element_id}")))?;

    // Visibility check: non-public elements visible only to involved users or admins
    let status_code = row.status_code.as_deref().unwrap_or("DRAFT");
    if !matches!(status_code, "ACCEPTED" | "DEPRECATED") {
        let is_admin = claims.roles.iter().any(|r| r == "ADMIN" || r == "admin");
        let is_involved = row.created_by == claims.sub
            || row.owner_user_id == Some(claims.sub)
            || row.steward_user_id == Some(claims.sub)
            || row.approver_user_id == Some(claims.sub);
        if !is_admin && !is_involved {
            return Err(AppError::NotFound(format!(
                "data element not found: {element_id}"
            )));
        }
    }

    // Separate queries for junction/aggregate data only
    let technical_columns = sqlx::query_as::<_, TechnicalColumn>(
        r#"
        SELECT
            column_id, table_id, column_name, ordinal_position,
            data_type, max_length, numeric_precision,
            is_nullable, is_primary_key, is_foreign_key,
            element_id, naming_standard_compliant, naming_standard_violation
        FROM technical_columns
        WHERE element_id = $1 AND deleted_at IS NULL
        ORDER BY ordinal_position ASC
        "#,
    )
    .bind(element_id)
    .fetch_all(&state.pool)
    .await?;

    let quality_rules_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM quality_rules WHERE element_id = $1 AND deleted_at IS NULL",
    )
    .bind(element_id)
    .fetch_one(&state.pool)
    .await?;

    let linked_processes_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM process_data_elements WHERE element_id = $1",
    )
    .bind(element_id)
    .fetch_one(&state.pool)
    .await?;

    let linked_applications_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM application_data_elements WHERE element_id = $1",
    )
    .bind(element_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(DataElementFullView::from_row_and_junctions(
        row,
        technical_columns,
        quality_rules_count,
        linked_processes_count,
        linked_applications_count,
    )))
}

// ---------------------------------------------------------------------------
// create_element — POST /api/v1/data-dictionary/elements
// ---------------------------------------------------------------------------

/// Create a new data element in DRAFT status with an associated workflow instance.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/elements",
    request_body = CreateDataElementRequest,
    responses(
        (status = 201, description = "Data element created", body = DataElement),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn create_element(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateDataElementRequest>,
) -> AppResult<(StatusCode, Json<DataElement>)> {
    // Validate required fields
    let element_name = body.element_name.trim().to_string();
    if element_name.is_empty() {
        return Err(AppError::Validation("element_name is required".into()));
    }
    // element_code is auto-generated by DB trigger (DE-{DOMAIN}-{SEQ})
    let description = body.description.trim().to_string();
    if description.is_empty() {
        return Err(AppError::Validation("description is required".into()));
    }
    // data_type is optional on create — AI will suggest it
    let data_type = body.data_type.as_deref().map(|s| s.trim().to_string());

    // Look up DRAFT status_id from entity_statuses
    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    // Insert the new data element
    let insert_query = format!(
        r#"
        INSERT INTO data_elements (
            element_name, description,
            business_definition, business_rules, data_type,
            max_length, numeric_precision, numeric_scale,
            format_pattern, allowed_values, default_value,
            is_nullable, glossary_term_id, domain_id,
            classification_id, status_id, review_frequency_id,
            version_number, is_current_version, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 1, TRUE, $18)
        RETURNING {cols}
        "#,
        cols = DATA_ELEMENT_COLUMNS,
    );
    // Default review frequency to ANNUAL on create (CHANGE 2)
    let annual_frequency_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT frequency_id FROM glossary_review_frequencies WHERE frequency_code = 'ANNUAL'",
    )
    .fetch_optional(&state.pool)
    .await?;

    let element = sqlx::query_as::<_, DataElement>(&insert_query)
        .bind(&element_name)                      // $1
        .bind(&description)                       // $2
        .bind(body.business_definition.as_deref()) // $3
        .bind(body.business_rules.as_deref())      // $4
        .bind(data_type.as_deref())                // $5
        .bind(body.max_length)                     // $6
        .bind(body.numeric_precision)              // $7
        .bind(body.numeric_scale)                  // $8
        .bind(body.format_pattern.as_deref())      // $9
        .bind(&body.allowed_values)                // $10
        .bind(body.default_value.as_deref())       // $11
        .bind(body.is_nullable.unwrap_or(true))    // $12
        .bind(body.glossary_term_id)               // $13
        .bind(body.domain_id)                      // $14
        .bind(body.classification_id)              // $15
        .bind(draft_status_id)                     // $16
        .bind(annual_frequency_id)                 // $17
        .bind(claims.sub)                          // $18
        .fetch_one(&state.pool)
        .await?;

    // Initiate the workflow instance for this new data element
    workflow::service::initiate_workflow(
        &state.pool,
        workflow::ENTITY_DATA_ELEMENT,
        element.element_id,
        claims.sub,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(element)))
}

// ---------------------------------------------------------------------------
// update_element — PUT /api/v1/data-dictionary/elements/:element_id
// ---------------------------------------------------------------------------

/// Update an existing data element. Only provided fields are changed.
/// Requires authentication.
#[utoipa::path(
    put,
    path = "/api/v1/data-dictionary/elements/{element_id}",
    params(("element_id" = Uuid, Path, description = "Element ID")),
    request_body = UpdateDataElementRequest,
    responses(
        (status = 200, description = "Element updated", body = DataElement),
        (status = 404, description = "Element not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn update_element(
    State(state): State<AppState>,
    Path(element_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateDataElementRequest>,
) -> AppResult<Json<DataElement>> {
    // Verify the element exists and is not deleted
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM data_elements WHERE element_id = $1 AND deleted_at IS NULL)",
    )
    .bind(element_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "data element not found: {element_id}"
        )));
    }

    // Update using COALESCE to only change provided fields
    let update_query = format!(
        r#"
        UPDATE data_elements
        SET element_name        = COALESCE($1, element_name),
            element_code        = COALESCE($2, element_code),
            description         = COALESCE($3, description),
            business_definition = COALESCE($4, business_definition),
            business_rules      = COALESCE($5, business_rules),
            data_type           = COALESCE($6, data_type),
            max_length          = COALESCE($7, max_length),
            numeric_precision   = COALESCE($8, numeric_precision),
            numeric_scale       = COALESCE($9, numeric_scale),
            format_pattern      = COALESCE($10, format_pattern),
            allowed_values      = COALESCE($11, allowed_values),
            default_value       = COALESCE($12, default_value),
            is_nullable         = COALESCE($13, is_nullable),
            glossary_term_id    = COALESCE($14, glossary_term_id),
            domain_id           = COALESCE($15, domain_id),
            classification_id   = COALESCE($16, classification_id),
            owner_user_id       = COALESCE($17, owner_user_id),
            steward_user_id     = COALESCE($18, steward_user_id),
            approver_user_id    = COALESCE($19, approver_user_id),
            organisational_unit = COALESCE($20, organisational_unit),
            review_frequency_id = COALESCE($21, review_frequency_id),
            is_pii              = COALESCE($22, is_pii),
            updated_by          = $23,
            updated_at          = CURRENT_TIMESTAMP
        WHERE element_id = $24 AND deleted_at IS NULL
        RETURNING {cols}
        "#,
        cols = DATA_ELEMENT_COLUMNS,
    );

    let element = sqlx::query_as::<_, DataElement>(&update_query)
        .bind(body.element_name.as_deref())        // $1
        .bind(body.element_code.as_deref())        // $2
        .bind(body.description.as_deref())         // $3
        .bind(body.business_definition.as_deref()) // $4
        .bind(body.business_rules.as_deref())      // $5
        .bind(body.data_type.as_deref())           // $6
        .bind(body.max_length)                     // $7
        .bind(body.numeric_precision)              // $8
        .bind(body.numeric_scale)                  // $9
        .bind(body.format_pattern.as_deref())      // $10
        .bind(&body.allowed_values)                // $11
        .bind(body.default_value.as_deref())       // $12
        .bind(body.is_nullable)                    // $13
        .bind(body.glossary_term_id)               // $14
        .bind(body.domain_id)                      // $15
        .bind(body.classification_id)              // $16
        .bind(body.owner_user_id)                  // $17
        .bind(body.steward_user_id)                // $18
        .bind(body.approver_user_id)               // $19
        .bind(body.organisational_unit.as_deref()) // $20
        .bind(body.review_frequency_id)            // $21
        .bind(body.is_pii)                         // $22
        .bind(claims.sub)                          // $23
        .bind(element_id)                          // $24
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(element))
}

// ---------------------------------------------------------------------------
// amend_element — POST /api/v1/data-dictionary/elements/:element_id/amend
// ---------------------------------------------------------------------------

/// Create a new draft amendment of an accepted data element.
/// Copies all fields to a new row with incremented version_number.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/elements/{element_id}/amend",
    params(("element_id" = Uuid, Path, description = "Element ID of the accepted element to amend")),
    responses(
        (status = 201, description = "Amendment created in DRAFT status", body = DataElement),
        (status = 200, description = "Existing amendment returned", body = DataElement),
        (status = 422, description = "Element is not in ACCEPTED status")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn amend_element(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(element_id): Path<Uuid>,
) -> AppResult<(StatusCode, Json<DataElement>)> {
    // Verify the element exists
    let original = sqlx::query_as::<_, DataElement>(&format!(
        "SELECT {cols} FROM data_elements WHERE element_id = $1 AND deleted_at IS NULL",
        cols = DATA_ELEMENT_COLUMNS
    ))
    .bind(element_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("data element not found: {element_id}")))?;

    // Check status is ACCEPTED
    let status_code = sqlx::query_scalar::<_, String>(
        "SELECT status_code FROM entity_statuses WHERE status_id = $1",
    )
    .bind(original.status_id)
    .fetch_one(&state.pool)
    .await?;

    if status_code != "ACCEPTED" {
        return Err(AppError::Validation(
            "only accepted data elements can be amended".into(),
        ));
    }

    // If an amendment already exists, return it instead of creating a new one
    let existing_amendment = sqlx::query_as::<_, DataElement>(
        &format!(
            "SELECT {cols} FROM data_elements WHERE previous_version_id = $1 AND deleted_at IS NULL AND is_current_version = FALSE LIMIT 1",
            cols = DATA_ELEMENT_COLUMNS
        ),
    )
    .bind(element_id)
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

    // Insert new version with all fields copied, new element_id, DRAFT status
    let amendment = sqlx::query_as::<_, DataElement>(
        &format!(
            r#"
            INSERT INTO data_elements (
                element_name, element_code, description,
                business_definition, business_rules, data_type,
                max_length, numeric_precision, numeric_scale,
                format_pattern, allowed_values, default_value,
                is_nullable, is_cde, cde_rationale, cde_designated_at,
                glossary_term_id, domain_id, classification_id,
                status_id, owner_user_id, steward_user_id,
                approver_user_id, organisational_unit,
                review_frequency_id, is_pii,
                version_number, is_current_version, previous_version_id,
                created_by
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19,
                $20, $21, $22, $23, $24, $25, $26,
                $27, FALSE, $28, $29
            )
            RETURNING {cols}
            "#,
            cols = DATA_ELEMENT_COLUMNS
        ),
    )
    .bind(&original.element_name)            // $1
    .bind(&original.element_code)            // $2 — same code, new version
    .bind(&original.description)             // $3
    .bind(original.business_definition.as_deref()) // $4
    .bind(original.business_rules.as_deref()) // $5
    .bind(&original.data_type)               // $6
    .bind(original.max_length)               // $7
    .bind(original.numeric_precision)        // $8
    .bind(original.numeric_scale)            // $9
    .bind(original.format_pattern.as_deref()) // $10
    .bind(&original.allowed_values)          // $11
    .bind(original.default_value.as_deref()) // $12
    .bind(original.is_nullable)              // $13
    .bind(original.is_cde)                   // $14
    .bind(original.cde_rationale.as_deref()) // $15
    .bind(original.cde_designated_at)        // $16
    .bind(original.glossary_term_id)         // $17
    .bind(original.domain_id)                // $18
    .bind(original.classification_id)        // $19
    .bind(draft_status_id)                   // $20
    .bind(original.owner_user_id)            // $21
    .bind(original.steward_user_id)          // $22
    .bind(original.approver_user_id)         // $23
    .bind(original.organisational_unit.as_deref()) // $24
    .bind(original.review_frequency_id)      // $25
    .bind(original.is_pii)                   // $26
    .bind(new_version)                       // $27
    .bind(element_id)                        // $28 = previous_version_id
    .bind(claims.sub)                        // $29 = created_by
    .fetch_one(&state.pool)
    .await?;

    // Initiate workflow for the amendment
    workflow::service::initiate_workflow(
        &state.pool,
        workflow::ENTITY_DATA_ELEMENT,
        amendment.element_id,
        claims.sub,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(amendment)))
}

// ---------------------------------------------------------------------------
// discard_amendment — DELETE /api/v1/data-dictionary/elements/:element_id/discard
// ---------------------------------------------------------------------------

/// Discard a draft data element. Only the creator or admin can discard,
/// and only in DRAFT status. Hard deletes the element (never-submitted drafts
/// have no governance value). Works for both new drafts and amendments.
#[utoipa::path(
    delete,
    path = "/api/v1/data-dictionary/elements/{element_id}/discard",
    params(("element_id" = Uuid, Path, description = "Data element ID to discard")),
    responses(
        (status = 204, description = "Draft discarded"),
        (status = 403, description = "Only the creator can discard"),
        (status = 422, description = "Element is not in DRAFT status")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn discard_amendment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(element_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    // Fetch the element
    let row = sqlx::query_as::<_, DataElement>(&format!(
        "SELECT {cols} FROM data_elements WHERE element_id = $1 AND deleted_at IS NULL",
        cols = DATA_ELEMENT_COLUMNS
    ))
    .bind(element_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("data element not found: {element_id}")))?;

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
    .bind(element_id)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM workflow_history
        WHERE instance_id IN (SELECT instance_id FROM workflow_instances WHERE entity_id = $1)
        "#,
    )
    .bind(element_id)
    .execute(&state.pool)
    .await?;

    sqlx::query("DELETE FROM workflow_instances WHERE entity_id = $1")
        .bind(element_id)
        .execute(&state.pool)
        .await?;

    // Delete the amendment element itself
    sqlx::query("DELETE FROM data_elements WHERE element_id = $1")
        .bind(element_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// list_cde — GET /api/v1/data-dictionary/elements/cde
// ---------------------------------------------------------------------------

/// List all Critical Data Elements (CDEs) across the data dictionary (Principle 12).
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/elements/cde",
    responses(
        (status = 200, description = "List critical data elements",
         body = Vec<DataElementListItem>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_cde(State(state): State<AppState>) -> AppResult<Json<Vec<DataElementListItem>>> {
    let items = sqlx::query_as::<_, DataElementListItem>(
        r#"
        SELECT
            de.element_id,
            de.element_name,
            de.element_code,
            de.description,
            de.data_type,
            de.is_cde,
            gd.domain_name                AS domain_name,
            dc.classification_name        AS classification_name,
            es.status_code                AS status_code,
            es.status_name                AS status_name,
            uo.display_name               AS owner_name,
            gt.term_name                  AS glossary_term_name,
            de.created_at,
            de.updated_at
        FROM data_elements de
        JOIN entity_statuses es ON es.status_id = de.status_id
        LEFT JOIN glossary_domains gd ON gd.domain_id = de.domain_id
        LEFT JOIN data_classifications dc ON dc.classification_id = de.classification_id
        LEFT JOIN users uo ON uo.user_id = de.owner_user_id
        LEFT JOIN glossary_terms gt ON gt.term_id = de.glossary_term_id
        WHERE de.is_cde = TRUE
          AND de.deleted_at IS NULL
          AND de.is_current_version = TRUE
        ORDER BY de.element_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(items))
}

// ---------------------------------------------------------------------------
// designate_cde — POST /api/v1/data-dictionary/elements/:element_id/cde
// ---------------------------------------------------------------------------

/// Designate or remove Critical Data Element (CDE) status for a data element (Principle 12).
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/elements/{element_id}/cde",
    params(("element_id" = Uuid, Path, description = "Element ID")),
    request_body = CdeDesignationRequest,
    responses(
        (status = 200, description = "CDE designation updated", body = DataElement),
        (status = 404, description = "Element not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn designate_cde(
    State(state): State<AppState>,
    Path(element_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CdeDesignationRequest>,
) -> AppResult<Json<DataElement>> {
    // Verify the element exists and is not deleted
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM data_elements WHERE element_id = $1 AND deleted_at IS NULL)",
    )
    .bind(element_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "data element not found: {element_id}"
        )));
    }

    let designate_query = format!(
        r#"
        UPDATE data_elements
        SET is_cde             = $1,
            cde_rationale      = $2,
            cde_designated_at  = CASE WHEN $1 = TRUE THEN CURRENT_TIMESTAMP ELSE NULL END,
            cde_designated_by  = CASE WHEN $1 = TRUE THEN $3 ELSE NULL END,
            updated_by         = $3,
            updated_at         = CURRENT_TIMESTAMP
        WHERE element_id = $4 AND deleted_at IS NULL
        RETURNING {cols}
        "#,
        cols = DATA_ELEMENT_COLUMNS,
    );

    let element = sqlx::query_as::<_, DataElement>(&designate_query)
        .bind(body.is_cde)
        .bind(body.cde_rationale.as_deref())
        .bind(claims.sub)
        .bind(element_id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(element))
}

// ---------------------------------------------------------------------------
// list_source_systems — GET /api/v1/data-dictionary/source-systems
// ---------------------------------------------------------------------------

/// List all registered source systems.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/source-systems",
    responses(
        (status = 200, description = "List source systems", body = Vec<SourceSystem>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_source_systems(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<SourceSystem>>> {
    let systems = sqlx::query_as::<_, SourceSystem>(
        r#"
        SELECT system_id, system_name, system_code, system_type,
               description, connection_details,
               application_id, vendor, environment
        FROM source_systems
        WHERE deleted_at IS NULL
        ORDER BY system_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(systems))
}

// ---------------------------------------------------------------------------
// create_source_system — POST /api/v1/data-dictionary/source-systems
// ---------------------------------------------------------------------------

/// Register a new source system.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/source-systems",
    request_body = CreateSourceSystemRequest,
    responses(
        (status = 201, description = "Source system created", body = SourceSystem),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn create_source_system(
    State(state): State<AppState>,
    Json(body): Json<CreateSourceSystemRequest>,
) -> AppResult<(StatusCode, Json<SourceSystem>)> {
    let system_name = body.system_name.trim().to_string();
    if system_name.is_empty() {
        return Err(AppError::Validation("system_name is required".into()));
    }
    let system_code = body.system_code.trim().to_string();
    if system_code.is_empty() {
        return Err(AppError::Validation("system_code is required".into()));
    }
    let system_type = body.system_type.trim().to_string();
    if system_type.is_empty() {
        return Err(AppError::Validation("system_type is required".into()));
    }

    let system = sqlx::query_as::<_, SourceSystem>(
        r#"
        INSERT INTO source_systems (
            system_name, system_code, system_type, description, connection_details,
            application_id, vendor, environment
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING system_id, system_name, system_code, system_type, description,
                  connection_details, application_id, vendor, environment
        "#,
    )
    .bind(&system_name)
    .bind(&system_code)
    .bind(&system_type)
    .bind(body.description.as_deref())
    .bind(&body.connection_details)
    .bind(body.application_id)
    .bind(body.vendor.as_deref())
    .bind(body.environment.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(system)))
}

// ---------------------------------------------------------------------------
// list_classifications — GET /api/v1/data-dictionary/classifications
// ---------------------------------------------------------------------------

/// List all data classification levels.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/classifications",
    responses(
        (status = 200, description = "List data classifications",
         body = Vec<DataClassification>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_classifications(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DataClassification>>> {
    let classifications = sqlx::query_as::<_, DataClassification>(
        r#"
        SELECT classification_id, classification_code, classification_name, description
        FROM data_classifications
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(classifications))
}

// ---------------------------------------------------------------------------
// list_schemas — GET /api/v1/data-dictionary/source-systems/:system_id/schemas
// ---------------------------------------------------------------------------

/// List technical schemas for a source system.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/source-systems/{system_id}/schemas",
    params(("system_id" = Uuid, Path, description = "Source system ID")),
    responses(
        (status = 200, description = "List schemas for a source system",
         body = Vec<TechnicalSchema>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_schemas(
    State(state): State<AppState>,
    Path(system_id): Path<Uuid>,
) -> AppResult<Json<Vec<TechnicalSchema>>> {
    let schemas = sqlx::query_as::<_, TechnicalSchema>(
        r#"
        SELECT schema_id, system_id, schema_name, description
        FROM technical_schemas
        WHERE system_id = $1 AND deleted_at IS NULL
        ORDER BY schema_name ASC
        "#,
    )
    .bind(system_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(schemas))
}

// ---------------------------------------------------------------------------
// create_schema — POST /api/v1/data-dictionary/source-systems/:system_id/schemas
// ---------------------------------------------------------------------------

/// Create a new technical schema under a source system.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/source-systems/{system_id}/schemas",
    params(("system_id" = Uuid, Path, description = "Source system ID")),
    request_body = CreateTechnicalSchemaRequest,
    responses(
        (status = 201, description = "Schema created", body = TechnicalSchema),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn create_schema(
    State(state): State<AppState>,
    Path(system_id): Path<Uuid>,
    Json(body): Json<CreateTechnicalSchemaRequest>,
) -> AppResult<(StatusCode, Json<TechnicalSchema>)> {
    let schema_name = body.schema_name.trim().to_string();
    if schema_name.is_empty() {
        return Err(AppError::Validation("schema_name is required".into()));
    }

    // Verify the source system exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM source_systems WHERE system_id = $1 AND deleted_at IS NULL)",
    )
    .bind(system_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "source system not found: {system_id}"
        )));
    }

    let schema = sqlx::query_as::<_, TechnicalSchema>(
        r#"
        INSERT INTO technical_schemas (system_id, schema_name, description)
        VALUES ($1, $2, $3)
        RETURNING schema_id, system_id, schema_name, description
        "#,
    )
    .bind(system_id)
    .bind(&schema_name)
    .bind(body.description.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(schema)))
}

// ---------------------------------------------------------------------------
// list_tables — GET /api/v1/data-dictionary/schemas/:schema_id/tables
// ---------------------------------------------------------------------------

/// List technical tables for a schema.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/schemas/{schema_id}/tables",
    params(("schema_id" = Uuid, Path, description = "Schema ID")),
    responses(
        (status = 200, description = "List tables for a schema",
         body = Vec<TechnicalTable>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_tables(
    State(state): State<AppState>,
    Path(schema_id): Path<Uuid>,
) -> AppResult<Json<Vec<TechnicalTable>>> {
    let tables = sqlx::query_as::<_, TechnicalTable>(
        r#"
        SELECT table_id, schema_id, table_name, table_type,
               description, row_count, size_bytes
        FROM technical_tables
        WHERE schema_id = $1 AND deleted_at IS NULL
        ORDER BY table_name ASC
        "#,
    )
    .bind(schema_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(tables))
}

// ---------------------------------------------------------------------------
// create_table — POST /api/v1/data-dictionary/schemas/:schema_id/tables
// ---------------------------------------------------------------------------

/// Create a new technical table under a schema.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/schemas/{schema_id}/tables",
    params(("schema_id" = Uuid, Path, description = "Schema ID")),
    request_body = CreateTechnicalTableRequest,
    responses(
        (status = 201, description = "Table created", body = TechnicalTable),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn create_table(
    State(state): State<AppState>,
    Path(schema_id): Path<Uuid>,
    Json(body): Json<CreateTechnicalTableRequest>,
) -> AppResult<(StatusCode, Json<TechnicalTable>)> {
    let table_name = body.table_name.trim().to_string();
    if table_name.is_empty() {
        return Err(AppError::Validation("table_name is required".into()));
    }

    // Verify the schema exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM technical_schemas WHERE schema_id = $1 AND deleted_at IS NULL)",
    )
    .bind(schema_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "technical schema not found: {schema_id}"
        )));
    }

    let table_type = body.table_type.as_deref().unwrap_or("TABLE").to_string();

    let table = sqlx::query_as::<_, TechnicalTable>(
        r#"
        INSERT INTO technical_tables (schema_id, table_name, table_type, description, row_count, size_bytes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING table_id, schema_id, table_name, table_type, description, row_count, size_bytes
        "#,
    )
    .bind(schema_id)
    .bind(&table_name)
    .bind(&table_type)
    .bind(body.description.as_deref())
    .bind(body.row_count)
    .bind(body.size_bytes)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(table)))
}

// ---------------------------------------------------------------------------
// list_columns — GET /api/v1/data-dictionary/tables/:table_id/columns
// ---------------------------------------------------------------------------

/// List columns for a technical table.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/tables/{table_id}/columns",
    params(("table_id" = Uuid, Path, description = "Table ID")),
    responses(
        (status = 200, description = "List columns for a table",
         body = Vec<TechnicalColumn>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_columns(
    State(state): State<AppState>,
    Path(table_id): Path<Uuid>,
) -> AppResult<Json<Vec<TechnicalColumn>>> {
    let columns = sqlx::query_as::<_, TechnicalColumn>(
        r#"
        SELECT
            column_id, table_id, column_name, ordinal_position,
            data_type, max_length, numeric_precision,
            is_nullable, is_primary_key, is_foreign_key,
            element_id, naming_standard_compliant, naming_standard_violation
        FROM technical_columns
        WHERE table_id = $1 AND deleted_at IS NULL
        ORDER BY ordinal_position ASC
        "#,
    )
    .bind(table_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(columns))
}

// ---------------------------------------------------------------------------
// create_column — POST /api/v1/data-dictionary/tables/:table_id/columns
// ---------------------------------------------------------------------------

/// Create a new column and validate its name against naming standards (Principle 8).
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/tables/{table_id}/columns",
    params(("table_id" = Uuid, Path, description = "Table ID")),
    request_body = CreateTechnicalColumnRequest,
    responses(
        (status = 201, description = "Column created with naming validation",
         body = CreateTechnicalColumnResponse),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn create_column(
    State(state): State<AppState>,
    Path(table_id): Path<Uuid>,
    Json(body): Json<CreateTechnicalColumnRequest>,
) -> AppResult<(StatusCode, Json<CreateTechnicalColumnResponse>)> {
    let column_name = body.column_name.trim().to_string();
    if column_name.is_empty() {
        return Err(AppError::Validation("column_name is required".into()));
    }
    let data_type = body.data_type.trim().to_string();
    if data_type.is_empty() {
        return Err(AppError::Validation("data_type is required".into()));
    }

    // Verify the table exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM technical_tables WHERE table_id = $1 AND deleted_at IS NULL)",
    )
    .bind(table_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "technical table not found: {table_id}"
        )));
    }

    // Load naming standards from the database for column validation
    let standards_rows = sqlx::query_as::<_, NamingStandardRow>(
        r#"
        SELECT standard_name, applies_to, pattern_regex, description,
               example_valid, example_invalid, is_mandatory
        FROM naming_standards
        WHERE deleted_at IS NULL
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let standards: Vec<naming::NamingStandard> = standards_rows
        .into_iter()
        .map(|r| naming::NamingStandard {
            name: r.standard_name,
            applies_to: r.applies_to,
            pattern: r.pattern_regex,
            description: r.description,
            example_valid: r.example_valid.unwrap_or_default(),
            example_invalid: r.example_invalid.unwrap_or_default(),
            is_mandatory: r.is_mandatory,
        })
        .collect();

    // Validate the column name against naming standards
    let validation = naming::validate_name(&column_name, "COLUMN", &standards);

    // Store the validation result in the column record
    let naming_compliant = validation.is_compliant;
    let naming_violation = if validation.violations.is_empty() {
        None
    } else {
        Some(
            validation
                .violations
                .iter()
                .map(|v| v.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        )
    };

    let column = sqlx::query_as::<_, TechnicalColumn>(
        r#"
        INSERT INTO technical_columns (
            table_id, column_name, ordinal_position, data_type,
            max_length, numeric_precision, is_nullable,
            is_primary_key, is_foreign_key, element_id,
            naming_standard_compliant, naming_standard_violation
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING
            column_id, table_id, column_name, ordinal_position,
            data_type, max_length, numeric_precision,
            is_nullable, is_primary_key, is_foreign_key,
            element_id, naming_standard_compliant, naming_standard_violation
        "#,
    )
    .bind(table_id)
    .bind(&column_name)
    .bind(body.ordinal_position)
    .bind(&data_type)
    .bind(body.max_length)
    .bind(body.numeric_precision)
    .bind(body.is_nullable.unwrap_or(true))
    .bind(body.is_primary_key.unwrap_or(false))
    .bind(body.is_foreign_key.unwrap_or(false))
    .bind(body.element_id)
    .bind(naming_compliant)
    .bind(naming_violation.as_deref())
    .fetch_one(&state.pool)
    .await?;

    let naming_info = NamingValidationInfo {
        is_compliant: naming_compliant,
        violations: validation
            .violations
            .into_iter()
            .map(|v| NamingViolationInfo {
                standard_name: v.standard_name,
                message: v.message,
            })
            .collect(),
    };

    Ok((
        StatusCode::CREATED,
        Json(CreateTechnicalColumnResponse {
            column,
            naming_validation: naming_info,
        }),
    ))
}

// ---------------------------------------------------------------------------
// Internal row types
// ---------------------------------------------------------------------------

/// Internal row type for loading naming standards from the database
#[derive(sqlx::FromRow)]
struct NamingStandardRow {
    standard_name: String,
    applies_to: String,
    pattern_regex: String,
    description: String,
    example_valid: Option<String>,
    example_invalid: Option<String>,
    is_mandatory: bool,
}
