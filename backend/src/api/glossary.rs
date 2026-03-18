use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::ai::{AiEnrichRequest, AiEnrichResponse};
use crate::domain::glossary::*;
use crate::error::{AppError, AppResult};
use crate::workflow;

// ---------------------------------------------------------------------------
// list_terms — GET /api/v1/glossary/terms
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/terms",
    params(SearchGlossaryTermsRequest),
    responses(
        (status = 200, description = "Paginated list of glossary terms",
         body = PaginatedGlossaryTerms)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_terms(
    State(state): State<AppState>,
    Query(params): Query<SearchGlossaryTermsRequest>,
) -> AppResult<Json<PaginatedResponse<GlossaryTermListItem>>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Count query — mirrors the same WHERE conditions as the data query
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM glossary_terms gt
        JOIN entity_statuses es ON es.status_id = gt.status_id
        WHERE gt.is_current_version = TRUE
          AND gt.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR gt.search_vector @@ plainto_tsquery('english', $1))
          AND ($2::UUID IS NULL OR gt.domain_id = $2)
          AND ($3::UUID IS NULL OR gt.category_id = $3)
          AND ($4::TEXT IS NULL OR es.status_code = $4)
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.domain_id)
    .bind(params.category_id)
    .bind(params.status.as_deref())
    .fetch_one(&state.pool)
    .await?;

    // Data query with joins for display fields
    let items = sqlx::query_as::<_, GlossaryTermListItem>(
        r#"
        SELECT
            gt.term_id,
            gt.term_name,
            gt.definition,
            gt.abbreviation,
            gd.domain_name                AS domain_name,
            gc.category_name              AS category_name,
            es.status_code                AS status_code,
            es.status_name                AS status_name,
            uo.display_name               AS owner_name,
            us.display_name               AS steward_name,
            gt.version_number,
            gt.created_at,
            gt.updated_at
        FROM glossary_terms gt
        JOIN entity_statuses es ON es.status_id = gt.status_id
        LEFT JOIN glossary_domains gd ON gd.domain_id = gt.domain_id
        LEFT JOIN glossary_categories gc ON gc.category_id = gt.category_id
        LEFT JOIN users uo ON uo.user_id = gt.owner_user_id
        LEFT JOIN users us ON us.user_id = gt.steward_user_id
        WHERE gt.is_current_version = TRUE
          AND gt.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR gt.search_vector @@ plainto_tsquery('english', $1))
          AND ($2::UUID IS NULL OR gt.domain_id = $2)
          AND ($3::UUID IS NULL OR gt.category_id = $3)
          AND ($4::TEXT IS NULL OR es.status_code = $4)
        ORDER BY gt.term_name ASC
        LIMIT $5
        OFFSET $6
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.domain_id)
    .bind(params.category_id)
    .bind(params.status.as_deref())
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
// get_term — GET /api/v1/glossary/terms/:term_id
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/terms/{term_id}",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    responses(
        (status = 200, description = "Glossary term details", body = GlossaryTerm),
        (status = 404, description = "Term not found")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn get_term(
    State(state): State<AppState>,
    Path(term_id): Path<Uuid>,
) -> AppResult<Json<GlossaryTerm>> {
    let term = sqlx::query_as::<_, GlossaryTerm>(
        r#"
        SELECT
            term_id, term_name, definition, business_context, examples,
            abbreviation, domain_id, category_id, status_id,
            owner_user_id, steward_user_id, version_number,
            is_current_version, source_reference, regulatory_reference,
            created_by, updated_by, created_at, updated_at
        FROM glossary_terms
        WHERE term_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(term_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("glossary term not found: {term_id}")))?;

    Ok(Json(term))
}

// ---------------------------------------------------------------------------
// create_term — POST /api/v1/glossary/terms
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/glossary/terms",
    request_body = CreateGlossaryTermRequest,
    responses(
        (status = 201, description = "Term created", body = GlossaryTerm),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn create_term(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateGlossaryTermRequest>,
) -> AppResult<(StatusCode, Json<GlossaryTerm>)> {

    // Validate required fields
    let term_name = body.term_name.trim().to_string();
    if term_name.is_empty() {
        return Err(AppError::Validation("term_name is required".into()));
    }
    let definition = body.definition.trim().to_string();
    if definition.is_empty() {
        return Err(AppError::Validation("definition is required".into()));
    }

    // Look up DRAFT status_id from entity_statuses
    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    // Insert the new glossary term
    let term = sqlx::query_as::<_, GlossaryTerm>(
        r#"
        INSERT INTO glossary_terms (
            term_name, definition, business_context, examples,
            abbreviation, domain_id, category_id, status_id,
            source_reference, regulatory_reference,
            version_number, is_current_version, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, TRUE, $11)
        RETURNING
            term_id, term_name, definition, business_context, examples,
            abbreviation, domain_id, category_id, status_id,
            owner_user_id, steward_user_id, version_number,
            is_current_version, source_reference, regulatory_reference,
            created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(&term_name)
    .bind(&definition)
    .bind(body.business_context.as_deref())
    .bind(body.examples.as_deref())
    .bind(body.abbreviation.as_deref())
    .bind(body.domain_id)
    .bind(body.category_id)
    .bind(draft_status_id)
    .bind(body.source_reference.as_deref())
    .bind(body.regulatory_reference.as_deref())
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    // Initiate the workflow instance for this new term
    workflow::service::initiate_workflow(
        &state.pool,
        workflow::ENTITY_GLOSSARY_TERM,
        term.term_id,
        claims.sub,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(term)))
}

// ---------------------------------------------------------------------------
// update_term — PUT /api/v1/glossary/terms/:term_id
// ---------------------------------------------------------------------------

#[utoipa::path(
    put,
    path = "/api/v1/glossary/terms/{term_id}",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    request_body = UpdateGlossaryTermRequest,
    responses(
        (status = 200, description = "Term updated", body = GlossaryTerm),
        (status = 404, description = "Term not found")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn update_term(
    State(state): State<AppState>,
    Path(term_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateGlossaryTermRequest>,
) -> AppResult<Json<GlossaryTerm>> {

    // Verify the term exists and is not deleted
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM glossary_terms WHERE term_id = $1 AND deleted_at IS NULL)",
    )
    .bind(term_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "glossary term not found: {term_id}"
        )));
    }

    // Update using COALESCE to only change provided fields
    let term = sqlx::query_as::<_, GlossaryTerm>(
        r#"
        UPDATE glossary_terms
        SET term_name            = COALESCE($1, term_name),
            definition           = COALESCE($2, definition),
            business_context     = COALESCE($3, business_context),
            examples             = COALESCE($4, examples),
            abbreviation         = COALESCE($5, abbreviation),
            domain_id            = COALESCE($6, domain_id),
            category_id          = COALESCE($7, category_id),
            source_reference     = COALESCE($8, source_reference),
            regulatory_reference = COALESCE($9, regulatory_reference),
            updated_by           = $10,
            updated_at           = CURRENT_TIMESTAMP
        WHERE term_id = $11 AND deleted_at IS NULL
        RETURNING
            term_id, term_name, definition, business_context, examples,
            abbreviation, domain_id, category_id, status_id,
            owner_user_id, steward_user_id, version_number,
            is_current_version, source_reference, regulatory_reference,
            created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(body.term_name.as_deref())
    .bind(body.definition.as_deref())
    .bind(body.business_context.as_deref())
    .bind(body.examples.as_deref())
    .bind(body.abbreviation.as_deref())
    .bind(body.domain_id)
    .bind(body.category_id)
    .bind(body.source_reference.as_deref())
    .bind(body.regulatory_reference.as_deref())
    .bind(claims.sub)
    .bind(term_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(term))
}

// ---------------------------------------------------------------------------
// list_domains — GET /api/v1/glossary/domains
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/domains",
    responses(
        (status = 200, description = "List glossary domains", body = Vec<GlossaryDomain>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_domains(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryDomain>>> {
    let domains = sqlx::query_as::<_, GlossaryDomain>(
        r#"
        SELECT domain_id, domain_name, description, parent_domain_id
        FROM glossary_domains
        ORDER BY domain_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(domains))
}

// ---------------------------------------------------------------------------
// list_categories — GET /api/v1/glossary/categories
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/categories",
    responses(
        (status = 200, description = "List glossary categories", body = Vec<GlossaryCategory>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_categories(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryCategory>>> {
    let categories = sqlx::query_as::<_, GlossaryCategory>(
        r#"
        SELECT category_id, category_name, description
        FROM glossary_categories
        ORDER BY category_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(categories))
}

// ---------------------------------------------------------------------------
// ai_enrich_term — POST /api/v1/glossary/terms/:term_id/ai-enrich
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/glossary/terms/{term_id}/ai-enrich",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    responses(
        (status = 200, description = "AI enrichment suggestions generated", body = AiEnrichResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn ai_enrich_term(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(term_id): Path<Uuid>,
) -> AppResult<Json<AiEnrichResponse>> {
    // Delegate to the generic AI enrich handler
    let request = AiEnrichRequest {
        entity_type: "glossary_term".to_string(),
        entity_id: term_id,
    };
    let result = super::ai::enrich(
        State(state),
        Extension(claims),
        Json(request),
    )
    .await?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// get_stats — GET /api/v1/stats
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/stats",
    responses(
        (status = 200, description = "Dashboard statistics", body = DashboardStats)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn get_stats(
    State(state): State<AppState>,
) -> AppResult<Json<DashboardStats>> {
    // Run all counts in a single query for efficiency
    let row = sqlx::query_as::<_, StatsRow>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM glossary_terms
             WHERE is_current_version = TRUE AND deleted_at IS NULL)         AS total_terms,
            (SELECT COUNT(*) FROM data_elements
             WHERE deleted_at IS NULL)                                       AS total_elements,
            (SELECT COUNT(*) FROM data_elements
             WHERE is_cde = TRUE AND deleted_at IS NULL)                     AS total_cde,
            (SELECT COUNT(*) FROM quality_rules
             WHERE deleted_at IS NULL)                                       AS total_quality_rules,
            (SELECT COUNT(*) FROM applications
             WHERE deleted_at IS NULL)                                       AS total_applications,
            (SELECT COUNT(*) FROM business_processes
             WHERE deleted_at IS NULL)                                       AS total_processes,
            (SELECT COUNT(*) FROM workflow_tasks
             WHERE status = 'PENDING')                                       AS pending_tasks_count
        "#,
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(DashboardStats {
        glossary_terms: row.total_terms,
        data_elements: row.total_elements,
        critical_data_elements: row.total_cde,
        quality_rules: row.total_quality_rules,
        applications: row.total_applications,
        business_processes: row.total_processes,
        pending_tasks: row.pending_tasks_count,
    }))
}

/// Internal row type for the stats aggregate query
#[derive(sqlx::FromRow)]
struct StatsRow {
    total_terms: i64,
    total_elements: i64,
    total_cde: i64,
    total_quality_rules: i64,
    total_applications: i64,
    total_processes: i64,
    pending_tasks_count: i64,
}
