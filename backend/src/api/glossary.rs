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
          AND ($5::UUID IS NULL OR gt.term_type_id = $5)
          AND ($6::BOOLEAN IS NULL OR gt.is_cde = $6)
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.domain_id)
    .bind(params.category_id)
    .bind(params.status.as_deref())
    .bind(params.term_type_id)
    .bind(params.is_cde)
    .fetch_one(&state.pool)
    .await?;

    // Data query with joins for display fields
    let items = sqlx::query_as::<_, GlossaryTermListItem>(
        r#"
        SELECT
            gt.term_id,
            gt.term_name,
            gt.term_code,
            gt.definition,
            gt.abbreviation,
            gd.domain_name                AS domain_name,
            gc.category_name              AS category_name,
            gtt.type_name                 AS term_type_name,
            es.status_code                AS status_code,
            es.status_name                AS status_name,
            uo.display_name               AS owner_name,
            us.display_name               AS steward_name,
            gt.is_cde,
            gt.version_number,
            gt.created_at,
            gt.updated_at
        FROM glossary_terms gt
        JOIN entity_statuses es ON es.status_id = gt.status_id
        LEFT JOIN glossary_domains gd ON gd.domain_id = gt.domain_id
        LEFT JOIN glossary_categories gc ON gc.category_id = gt.category_id
        LEFT JOIN glossary_term_types gtt ON gtt.term_type_id = gt.term_type_id
        LEFT JOIN users uo ON uo.user_id = gt.owner_user_id
        LEFT JOIN users us ON us.user_id = gt.steward_user_id
        WHERE gt.is_current_version = TRUE
          AND gt.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR gt.search_vector @@ plainto_tsquery('english', $1))
          AND ($2::UUID IS NULL OR gt.domain_id = $2)
          AND ($3::UUID IS NULL OR gt.category_id = $3)
          AND ($4::TEXT IS NULL OR es.status_code = $4)
          AND ($5::UUID IS NULL OR gt.term_type_id = $5)
          AND ($6::BOOLEAN IS NULL OR gt.is_cde = $6)
        ORDER BY gt.term_name ASC
        LIMIT $7
        OFFSET $8
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.domain_id)
    .bind(params.category_id)
    .bind(params.status.as_deref())
    .bind(params.term_type_id)
    .bind(params.is_cde)
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

/// All 45 column names for SELECT in the glossary_terms table
const GLOSSARY_TERM_COLUMNS: &str = r#"
    term_id, term_name, term_code, definition, abbreviation,
    business_context, examples, definition_notes, counter_examples,
    formula, unit_of_measure_id,
    term_type_id, domain_id, category_id, classification_id,
    owner_user_id, steward_user_id, domain_owner_user_id,
    approver_user_id, organisational_unit,
    status_id, version_number, is_current_version,
    approved_at, review_frequency_id, next_review_date,
    parent_term_id, source_reference, regulatory_reference,
    used_in_reports, used_in_policies, regulatory_reporting_usage,
    is_cde, golden_source, confidence_level_id,
    visibility_id, language_id, external_reference,
    created_by, updated_by, created_at, updated_at
"#;

#[utoipa::path(
    get,
    path = "/api/v1/glossary/terms/{term_id}",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    responses(
        (status = 200, description = "Glossary term details", body = GlossaryTermDetail),
        (status = 404, description = "Term not found")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn get_term(
    State(state): State<AppState>,
    Path(term_id): Path<Uuid>,
) -> AppResult<Json<GlossaryTermDetail>> {
    // ADR-0006 Pattern 1: Single JOIN query resolves all FK lookups in one round-trip
    let row = sqlx::query_as::<_, GlossaryTermDetailRow>(
        r#"
        SELECT
            gt.term_id, gt.term_name, gt.term_code, gt.definition,
            gt.abbreviation, gt.business_context, gt.examples,
            gt.definition_notes, gt.counter_examples, gt.formula,
            gt.unit_of_measure_id, gt.term_type_id, gt.domain_id,
            gt.category_id, gt.classification_id,
            gt.owner_user_id, gt.steward_user_id,
            gt.domain_owner_user_id, gt.approver_user_id,
            gt.organisational_unit,
            gt.status_id, gt.version_number, gt.is_current_version,
            gt.approved_at, gt.review_frequency_id, gt.next_review_date,
            gt.parent_term_id, gt.source_reference, gt.regulatory_reference,
            gt.used_in_reports, gt.used_in_policies,
            gt.regulatory_reporting_usage,
            gt.is_cde, gt.golden_source, gt.confidence_level_id,
            gt.visibility_id, gt.language_id, gt.external_reference,
            gt.previous_version_id,
            gt.created_by, gt.updated_by, gt.created_at, gt.updated_at,
            -- Resolved lookup names
            gd.domain_name,
            gc.category_name,
            gtt.type_name                 AS term_type_name,
            gum.unit_name                 AS unit_of_measure_name,
            dc.classification_name,
            grf.frequency_name            AS review_frequency_name,
            gcl.level_name                AS confidence_level_name,
            gvl.visibility_name,
            gl.language_name,
            pt.term_name                  AS parent_term_name,
            uo.display_name               AS owner_name,
            us.display_name               AS steward_name,
            udo.display_name              AS domain_owner_name,
            ua.display_name               AS approver_name,
            es.status_code,
            es.status_name
        FROM glossary_terms gt
        LEFT JOIN glossary_domains gd       ON gd.domain_id = gt.domain_id
        LEFT JOIN glossary_categories gc    ON gc.category_id = gt.category_id
        LEFT JOIN glossary_term_types gtt   ON gtt.term_type_id = gt.term_type_id
        LEFT JOIN glossary_units_of_measure gum ON gum.unit_id = gt.unit_of_measure_id
        LEFT JOIN data_classifications dc   ON dc.classification_id = gt.classification_id
        LEFT JOIN glossary_review_frequencies grf ON grf.frequency_id = gt.review_frequency_id
        LEFT JOIN glossary_confidence_levels gcl ON gcl.confidence_id = gt.confidence_level_id
        LEFT JOIN glossary_visibility_levels gvl ON gvl.visibility_id = gt.visibility_id
        LEFT JOIN glossary_languages gl     ON gl.language_id = gt.language_id
        LEFT JOIN glossary_terms pt         ON pt.term_id = gt.parent_term_id
        LEFT JOIN users uo                  ON uo.user_id = gt.owner_user_id
        LEFT JOIN users us                  ON us.user_id = gt.steward_user_id
        LEFT JOIN users udo                 ON udo.user_id = gt.domain_owner_user_id
        LEFT JOIN users ua                  ON ua.user_id = gt.approver_user_id
        LEFT JOIN entity_statuses es        ON es.status_id = gt.status_id
        WHERE gt.term_id = $1 AND gt.deleted_at IS NULL
        "#,
    )
    .bind(term_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("glossary term not found: {term_id}")))?;

    // Fetch junction data (always arrays — cannot be part of the single-row JOIN)
    let regulatory_tags = sqlx::query_as::<_, GlossaryRegulatoryTagItem>(
        r#"
        SELECT grt.tag_id, grt.tag_code, grt.tag_name, grt.description
        FROM glossary_term_regulatory_tags jtrt
        JOIN glossary_regulatory_tags grt ON grt.tag_id = jtrt.tag_id
        WHERE jtrt.term_id = $1
        ORDER BY grt.display_order
        "#,
    )
    .bind(term_id)
    .fetch_all(&state.pool)
    .await?;

    let subject_areas = sqlx::query_as::<_, GlossarySubjectAreaItem>(
        r#"
        SELECT gsa.subject_area_id, gsa.area_code, gsa.area_name, jtsa.is_primary
        FROM glossary_term_subject_areas jtsa
        JOIN glossary_subject_areas gsa ON gsa.subject_area_id = jtsa.subject_area_id
        WHERE jtsa.term_id = $1
        ORDER BY gsa.display_order
        "#,
    )
    .bind(term_id)
    .fetch_all(&state.pool)
    .await?;

    let tags = sqlx::query_as::<_, GlossaryTagItem>(
        r#"
        SELECT gt2.tag_id, gt2.tag_name
        FROM glossary_term_tags jtt
        JOIN glossary_tags gt2 ON gt2.tag_id = jtt.tag_id
        WHERE jtt.term_id = $1
        ORDER BY gt2.tag_name
        "#,
    )
    .bind(term_id)
    .fetch_all(&state.pool)
    .await?;

    let linked_processes = sqlx::query_as::<_, GlossaryLinkedProcess>(
        r#"
        SELECT bp.process_id, bp.process_name, jtp.usage_context
        FROM glossary_term_processes jtp
        JOIN business_processes bp ON bp.process_id = jtp.process_id
        WHERE jtp.term_id = $1
        ORDER BY bp.process_name
        "#,
    )
    .bind(term_id)
    .fetch_all(&state.pool)
    .await?;

    // Fetch aliases/synonyms
    let aliases = sqlx::query_as::<_, GlossaryAliasItem>(
        r#"
        SELECT DISTINCT ON (alias_name) alias_id, alias_name, alias_type
        FROM glossary_term_aliases
        WHERE term_id = $1
        ORDER BY alias_name, alias_id
        "#,
    )
    .bind(term_id)
    .fetch_all(&state.pool)
    .await?;

    // Construct the flat response (ADR-0006 Pattern 1)
    Ok(Json(GlossaryTermDetail::from_row_and_junctions(
        row,
        regulatory_tags,
        subject_areas,
        tags,
        linked_processes,
        aliases,
    )))
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

    // Default review frequency to ANNUAL (next_review_date calculated by DB trigger)
    let annual_frequency_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT frequency_id FROM glossary_review_frequencies WHERE frequency_code = 'ANNUAL'",
    )
    .fetch_optional(&state.pool)
    .await?;

    // Insert the new glossary term — minimal fields + review frequency default
    let insert_query = format!(
        r#"
        INSERT INTO glossary_terms (
            term_name, definition, domain_id, category_id, status_id,
            review_frequency_id, version_number, is_current_version, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, 1, TRUE, $7)
        RETURNING {cols}
        "#,
        cols = GLOSSARY_TERM_COLUMNS,
    );
    let term = sqlx::query_as::<_, GlossaryTerm>(&insert_query)
        .bind(&term_name)
        .bind(&definition)
        .bind(body.domain_id)
        .bind(body.category_id)
        .bind(draft_status_id)
        .bind(annual_frequency_id)
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
    let update_query = format!(
        r#"
        UPDATE glossary_terms
        SET term_name                = COALESCE($1, term_name),
            definition               = COALESCE($2, definition),
            abbreviation             = COALESCE($3, abbreviation),
            business_context         = COALESCE($4, business_context),
            examples                 = COALESCE($5, examples),
            definition_notes         = COALESCE($6, definition_notes),
            counter_examples         = COALESCE($7, counter_examples),
            formula                  = COALESCE($8, formula),
            unit_of_measure_id       = COALESCE($9, unit_of_measure_id),
            term_type_id             = COALESCE($10, term_type_id),
            domain_id                = COALESCE($11, domain_id),
            category_id              = COALESCE($12, category_id),
            classification_id        = COALESCE($13, classification_id),
            owner_user_id            = COALESCE($14, owner_user_id),
            steward_user_id          = COALESCE($15, steward_user_id),
            domain_owner_user_id     = COALESCE($16, domain_owner_user_id),
            approver_user_id         = COALESCE($17, approver_user_id),
            organisational_unit      = COALESCE($18, organisational_unit),
            approved_at              = COALESCE($19, approved_at),
            review_frequency_id      = COALESCE($20, review_frequency_id),
            parent_term_id           = COALESCE($21, parent_term_id),
            source_reference         = COALESCE($22, source_reference),
            regulatory_reference     = COALESCE($23, regulatory_reference),
            used_in_reports          = COALESCE($24, used_in_reports),
            used_in_policies         = COALESCE($25, used_in_policies),
            regulatory_reporting_usage = COALESCE($26, regulatory_reporting_usage),
            is_cde                   = COALESCE($27, is_cde),
            golden_source            = COALESCE($28, golden_source),
            confidence_level_id      = COALESCE($29, confidence_level_id),
            visibility_id            = COALESCE($30, visibility_id),
            language_id              = COALESCE($31, language_id),
            external_reference       = COALESCE($32, external_reference),
            updated_by               = $33,
            updated_at               = CURRENT_TIMESTAMP
        WHERE term_id = $34 AND deleted_at IS NULL
        RETURNING {cols}
        "#,
        cols = GLOSSARY_TERM_COLUMNS,
    );

    let term = sqlx::query_as::<_, GlossaryTerm>(&update_query)
        .bind(body.term_name.as_deref())
        .bind(body.definition.as_deref())
        .bind(body.abbreviation.as_deref())
        .bind(body.business_context.as_deref())
        .bind(body.examples.as_deref())
        .bind(body.definition_notes.as_deref())
        .bind(body.counter_examples.as_deref())
        .bind(body.formula.as_deref())
        .bind(body.unit_of_measure_id)
        .bind(body.term_type_id)
        .bind(body.domain_id)
        .bind(body.category_id)
        .bind(body.classification_id)
        .bind(body.owner_user_id)
        .bind(body.steward_user_id)
        .bind(body.domain_owner_user_id)
        .bind(body.approver_user_id)
        .bind(body.organisational_unit.as_deref())
        .bind(body.approved_at)
        .bind(body.review_frequency_id)
        .bind(body.parent_term_id)
        .bind(body.source_reference.as_deref())
        .bind(body.regulatory_reference.as_deref())
        .bind(body.used_in_reports.as_deref())
        .bind(body.used_in_policies.as_deref())
        .bind(body.regulatory_reporting_usage.as_deref())
        .bind(body.is_cde)
        .bind(body.golden_source.as_deref())
        .bind(body.confidence_level_id)
        .bind(body.visibility_id)
        .bind(body.language_id)
        .bind(body.external_reference.as_deref())
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

// ===========================================================================
// NEW LOOKUP ENDPOINTS
// ===========================================================================

// ---------------------------------------------------------------------------
// list_term_types — GET /api/v1/glossary/term-types
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/term-types",
    responses(
        (status = 200, description = "List term types", body = Vec<GlossaryTermType>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_term_types(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryTermType>>> {
    let rows = sqlx::query_as::<_, GlossaryTermType>(
        r#"
        SELECT term_type_id, type_code, type_name, description
        FROM glossary_term_types
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_review_frequencies — GET /api/v1/glossary/review-frequencies
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/review-frequencies",
    responses(
        (status = 200, description = "List review frequencies", body = Vec<GlossaryReviewFrequency>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_review_frequencies(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryReviewFrequency>>> {
    let rows = sqlx::query_as::<_, GlossaryReviewFrequency>(
        r#"
        SELECT frequency_id, frequency_code, frequency_name, months_interval
        FROM glossary_review_frequencies
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_confidence_levels — GET /api/v1/glossary/confidence-levels
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/confidence-levels",
    responses(
        (status = 200, description = "List confidence levels", body = Vec<GlossaryConfidenceLevel>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_confidence_levels(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryConfidenceLevel>>> {
    let rows = sqlx::query_as::<_, GlossaryConfidenceLevel>(
        r#"
        SELECT confidence_id, level_code, level_name, description
        FROM glossary_confidence_levels
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_visibility_levels — GET /api/v1/glossary/visibility-levels
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/visibility-levels",
    responses(
        (status = 200, description = "List visibility levels", body = Vec<GlossaryVisibilityLevel>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_visibility_levels(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryVisibilityLevel>>> {
    let rows = sqlx::query_as::<_, GlossaryVisibilityLevel>(
        r#"
        SELECT visibility_id, visibility_code, visibility_name, description
        FROM glossary_visibility_levels
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_units_of_measure — GET /api/v1/glossary/units-of-measure
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/units-of-measure",
    responses(
        (status = 200, description = "List units of measure", body = Vec<GlossaryUnitOfMeasure>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_units_of_measure(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryUnitOfMeasure>>> {
    let rows = sqlx::query_as::<_, GlossaryUnitOfMeasure>(
        r#"
        SELECT unit_id, unit_code, unit_name, unit_symbol
        FROM glossary_units_of_measure
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_regulatory_tags — GET /api/v1/glossary/regulatory-tags
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/regulatory-tags",
    responses(
        (status = 200, description = "List regulatory tags", body = Vec<GlossaryRegulatoryTag>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_regulatory_tags(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryRegulatoryTag>>> {
    let rows = sqlx::query_as::<_, GlossaryRegulatoryTag>(
        r#"
        SELECT tag_id, tag_code, tag_name, description
        FROM glossary_regulatory_tags
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_subject_areas — GET /api/v1/glossary/subject-areas
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/subject-areas",
    responses(
        (status = 200, description = "List subject areas", body = Vec<GlossarySubjectArea>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_subject_areas(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossarySubjectArea>>> {
    let rows = sqlx::query_as::<_, GlossarySubjectArea>(
        r#"
        SELECT subject_area_id, area_code, area_name, description
        FROM glossary_subject_areas
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_languages — GET /api/v1/glossary/languages
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/glossary/languages",
    responses(
        (status = 200, description = "List languages", body = Vec<GlossaryLanguage>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_languages(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryLanguage>>> {
    let rows = sqlx::query_as::<_, GlossaryLanguage>(
        r#"
        SELECT language_id, language_code, language_name
        FROM glossary_languages
        ORDER BY language_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ---------------------------------------------------------------------------
// list_organisational_units — GET /api/v1/glossary/organisational-units
// ---------------------------------------------------------------------------

/// List organisational units for dropdown selection.
#[utoipa::path(
    get,
    path = "/api/v1/glossary/organisational-units",
    responses(
        (status = 200, description = "List organisational units", body = Vec<OrganisationalUnit>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_organisational_units(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<OrganisationalUnit>>> {
    let rows = sqlx::query_as::<_, OrganisationalUnit>(
        r#"
        SELECT unit_id, unit_code, unit_name, description
        FROM organisational_units
        ORDER BY display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

// ===========================================================================
// JUNCTION MANAGEMENT ENDPOINTS
// ===========================================================================

// ---------------------------------------------------------------------------
// attach_regulatory_tag — POST /api/v1/glossary/terms/:term_id/regulatory-tags
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/glossary/terms/{term_id}/regulatory-tags",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    request_body = AttachRegulatoryTagRequest,
    responses(
        (status = 201, description = "Regulatory tag attached"),
        (status = 409, description = "Tag already attached")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn attach_regulatory_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(term_id): Path<Uuid>,
    Json(body): Json<AttachRegulatoryTagRequest>,
) -> AppResult<StatusCode> {
    sqlx::query(
        r#"
        INSERT INTO glossary_term_regulatory_tags (term_id, tag_id, created_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (term_id, tag_id) DO NOTHING
        "#,
    )
    .bind(term_id)
    .bind(body.tag_id)
    .bind(claims.sub)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::CREATED)
}

// ---------------------------------------------------------------------------
// detach_regulatory_tag — DELETE /api/v1/glossary/terms/:term_id/regulatory-tags/:tag_id
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/glossary/terms/{term_id}/regulatory-tags/{tag_id}",
    params(
        ("term_id" = Uuid, Path, description = "Term ID"),
        ("tag_id" = Uuid, Path, description = "Regulatory Tag ID")
    ),
    responses(
        (status = 204, description = "Regulatory tag removed"),
        (status = 404, description = "Attachment not found")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn detach_regulatory_tag(
    State(state): State<AppState>,
    Path((term_id, tag_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let result = sqlx::query(
        "DELETE FROM glossary_term_regulatory_tags WHERE term_id = $1 AND tag_id = $2",
    )
    .bind(term_id)
    .bind(tag_id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "regulatory tag attachment not found".into(),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// attach_subject_area — POST /api/v1/glossary/terms/:term_id/subject-areas
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/glossary/terms/{term_id}/subject-areas",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    request_body = AttachSubjectAreaRequest,
    responses(
        (status = 201, description = "Subject area attached"),
        (status = 409, description = "Subject area already attached")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn attach_subject_area(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(term_id): Path<Uuid>,
    Json(body): Json<AttachSubjectAreaRequest>,
) -> AppResult<StatusCode> {
    sqlx::query(
        r#"
        INSERT INTO glossary_term_subject_areas (term_id, subject_area_id, is_primary, created_by)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (term_id, subject_area_id) DO NOTHING
        "#,
    )
    .bind(term_id)
    .bind(body.area_id)
    .bind(body.is_primary.unwrap_or(false))
    .bind(claims.sub)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::CREATED)
}

// ---------------------------------------------------------------------------
// detach_subject_area — DELETE /api/v1/glossary/terms/:term_id/subject-areas/:area_id
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/glossary/terms/{term_id}/subject-areas/{area_id}",
    params(
        ("term_id" = Uuid, Path, description = "Term ID"),
        ("area_id" = Uuid, Path, description = "Subject Area ID")
    ),
    responses(
        (status = 204, description = "Subject area removed"),
        (status = 404, description = "Attachment not found")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn detach_subject_area(
    State(state): State<AppState>,
    Path((term_id, area_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let result = sqlx::query(
        "DELETE FROM glossary_term_subject_areas WHERE term_id = $1 AND subject_area_id = $2",
    )
    .bind(term_id)
    .bind(area_id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "subject area attachment not found".into(),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// attach_tag — POST /api/v1/glossary/terms/:term_id/tags
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/glossary/terms/{term_id}/tags",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    request_body = AttachTagRequest,
    responses(
        (status = 201, description = "Tag attached (created if not exists)"),
        (status = 409, description = "Tag already attached")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn attach_tag(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(term_id): Path<Uuid>,
    Json(body): Json<AttachTagRequest>,
) -> AppResult<StatusCode> {
    let tag_name = body.tag_name.trim().to_lowercase();
    if tag_name.is_empty() {
        return Err(AppError::Validation("tag_name is required".into()));
    }

    // Upsert the tag (create if not exists)
    let tag_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO glossary_tags (tag_name, created_by)
        VALUES ($1, $2)
        ON CONFLICT (tag_name) DO UPDATE SET tag_name = glossary_tags.tag_name
        RETURNING tag_id
        "#,
    )
    .bind(&tag_name)
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    // Attach the tag to the term
    sqlx::query(
        r#"
        INSERT INTO glossary_term_tags (term_id, tag_id, created_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (term_id, tag_id) DO NOTHING
        "#,
    )
    .bind(term_id)
    .bind(tag_id)
    .bind(claims.sub)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::CREATED)
}

// ---------------------------------------------------------------------------
// detach_tag — DELETE /api/v1/glossary/terms/:term_id/tags/:tag_id
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/glossary/terms/{term_id}/tags/{tag_id}",
    params(
        ("term_id" = Uuid, Path, description = "Term ID"),
        ("tag_id" = Uuid, Path, description = "Tag ID")
    ),
    responses(
        (status = 204, description = "Tag removed"),
        (status = 404, description = "Attachment not found")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn detach_tag(
    State(state): State<AppState>,
    Path((term_id, tag_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let result = sqlx::query(
        "DELETE FROM glossary_term_tags WHERE term_id = $1 AND tag_id = $2",
    )
    .bind(term_id)
    .bind(tag_id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("tag attachment not found".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ===========================================================================
// AI ENRICHMENT CONVENIENCE ENDPOINT
// ===========================================================================

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

// ===========================================================================
// DASHBOARD STATS
// ===========================================================================

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
