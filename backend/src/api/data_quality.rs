use axum::Extension;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::data_quality::*;
use crate::domain::glossary::PaginatedResponse;
use crate::error::{AppError, AppResult};
use crate::workflow;

// ---------------------------------------------------------------------------
// list_dimensions — GET /api/v1/data-quality/dimensions
// ---------------------------------------------------------------------------

/// List all data quality dimensions with aggregate rule counts and scores.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-quality/dimensions",
    responses(
        (status = 200, description = "List quality dimensions with aggregate stats",
         body = Vec<QualityDimensionSummary>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn list_dimensions(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<QualityDimensionSummary>>> {
    let dimensions = sqlx::query_as::<_, QualityDimensionSummary>(
        r#"
        SELECT
            qd.dimension_id,
            qd.dimension_code,
            qd.dimension_name,
            qd.description,
            COALESCE(rule_counts.cnt, 0)  AS rules_count,
            score_stats.avg_score         AS avg_score,
            score_stats.last_assessed     AS last_assessed_at
        FROM quality_dimensions qd
        LEFT JOIN (
            SELECT dimension_id, COUNT(*) AS cnt
            FROM quality_rules
            WHERE deleted_at IS NULL
            GROUP BY dimension_id
        ) rule_counts ON rule_counts.dimension_id = qd.dimension_id
        LEFT JOIN (
            SELECT
                qr.dimension_id,
                AVG(latest.score_percentage)  AS avg_score,
                MAX(latest.assessed_at)       AS last_assessed
            FROM quality_rules qr
            JOIN LATERAL (
                SELECT score_percentage, assessed_at
                FROM quality_assessments qa
                WHERE qa.rule_id = qr.rule_id
                  AND qa.status = 'COMPLETED'
                ORDER BY qa.assessed_at DESC
                LIMIT 1
            ) latest ON TRUE
            WHERE qr.deleted_at IS NULL
            GROUP BY qr.dimension_id
        ) score_stats ON score_stats.dimension_id = qd.dimension_id
        ORDER BY qd.display_order ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(dimensions))
}

// ---------------------------------------------------------------------------
// list_rule_types — GET /api/v1/data-quality/rule-types
// ---------------------------------------------------------------------------

/// List all quality rule types with their SQL templates.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-quality/rule-types",
    responses(
        (status = 200, description = "List quality rule types", body = Vec<QualityRuleType>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn list_rule_types(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<QualityRuleType>>> {
    let rule_types = sqlx::query_as::<_, QualityRuleType>(
        r#"
        SELECT rule_type_id, type_code, type_name, description, sql_template
        FROM quality_rule_types
        ORDER BY type_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rule_types))
}

// ---------------------------------------------------------------------------
// list_rules — GET /api/v1/data-quality/rules
// ---------------------------------------------------------------------------

/// List quality rules with optional filtering and pagination.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-quality/rules",
    params(SearchQualityRulesRequest),
    responses(
        (status = 200, description = "Paginated list of quality rules",
         body = PaginatedQualityRules)
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn list_rules(
    State(state): State<AppState>,
    Query(params): Query<SearchQualityRulesRequest>,
) -> AppResult<Json<PaginatedResponse<QualityRuleListItem>>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Count query — mirrors the same WHERE conditions as the data query
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM quality_rules qr
        JOIN entity_statuses es ON es.status_id = qr.status_id
        WHERE qr.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR qr.rule_name ILIKE '%' || $1 || '%'
               OR qr.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR qr.dimension_id = $2)
          AND ($3::UUID IS NULL OR qr.element_id = $3)
          AND ($4::TEXT IS NULL OR qr.severity = $4)
          AND ($5::BOOL IS NULL OR qr.is_active = $5)
          AND ($6::TEXT IS NULL OR es.status_code = $6)
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.dimension_id)
    .bind(params.element_id)
    .bind(params.severity.as_deref())
    .bind(params.is_active)
    .bind(params.status.as_deref())
    .fetch_one(&state.pool)
    .await?;

    // Data query with joins for display fields
    let items = sqlx::query_as::<_, QualityRuleListItem>(
        r#"
        SELECT
            qr.rule_id,
            qr.rule_name,
            qr.rule_code,
            qr.description,
            qd.dimension_name             AS dimension_name,
            qd.dimension_code             AS dimension_code,
            qrt.type_name                 AS rule_type_name,
            de.element_name               AS element_name,
            qr.severity,
            qr.is_active,
            es.status_code                AS status_code,
            es.status_name                AS status_name,
            uo.display_name               AS owner_name,
            qr.threshold_percentage,
            qr.created_at,
            qr.updated_at
        FROM quality_rules qr
        JOIN quality_dimensions qd ON qd.dimension_id = qr.dimension_id
        JOIN quality_rule_types qrt ON qrt.rule_type_id = qr.rule_type_id
        JOIN entity_statuses es ON es.status_id = qr.status_id
        LEFT JOIN data_elements de ON de.element_id = qr.element_id
        LEFT JOIN users uo ON uo.user_id = qr.owner_user_id
        WHERE qr.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR qr.rule_name ILIKE '%' || $1 || '%'
               OR qr.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR qr.dimension_id = $2)
          AND ($3::UUID IS NULL OR qr.element_id = $3)
          AND ($4::TEXT IS NULL OR qr.severity = $4)
          AND ($5::BOOL IS NULL OR qr.is_active = $5)
          AND ($6::TEXT IS NULL OR es.status_code = $6)
        ORDER BY qr.rule_name ASC
        LIMIT $7
        OFFSET $8
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.dimension_id)
    .bind(params.element_id)
    .bind(params.severity.as_deref())
    .bind(params.is_active)
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
// get_rule — GET /api/v1/data-quality/rules/:rule_id
// ---------------------------------------------------------------------------

/// Retrieve a single quality rule by ID.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-quality/rules/{rule_id}",
    params(("rule_id" = Uuid, Path, description = "Rule ID")),
    responses(
        (status = 200, description = "Quality rule details", body = QualityRule),
        (status = 404, description = "Rule not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn get_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<Uuid>,
) -> AppResult<Json<QualityRule>> {
    let rule = sqlx::query_as::<_, QualityRule>(
        r#"
        SELECT
            rule_id, rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage, severity,
            is_active, status_id, owner_user_id,
            created_by, updated_by, created_at, updated_at
        FROM quality_rules
        WHERE rule_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(rule_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("quality rule not found: {rule_id}")))?;

    Ok(Json(rule))
}

// ---------------------------------------------------------------------------
// create_rule — POST /api/v1/data-quality/rules
// ---------------------------------------------------------------------------

/// Create a new quality rule in DRAFT status with an associated workflow instance.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-quality/rules",
    request_body = CreateQualityRuleRequest,
    responses(
        (status = 201, description = "Quality rule created", body = QualityRule),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn create_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateQualityRuleRequest>,
) -> AppResult<(StatusCode, Json<QualityRule>)> {
    // Validate required fields
    let rule_name = body.rule_name.trim().to_string();
    if rule_name.is_empty() {
        return Err(AppError::Validation("rule_name is required".into()));
    }
    let rule_code = body.rule_code.trim().to_string();
    if rule_code.is_empty() {
        return Err(AppError::Validation("rule_code is required".into()));
    }
    let description = body.description.trim().to_string();
    if description.is_empty() {
        return Err(AppError::Validation("description is required".into()));
    }

    // Validate severity if provided
    let severity = body.severity.as_deref().unwrap_or("MEDIUM").to_string();
    if !["LOW", "MEDIUM", "HIGH", "CRITICAL"].contains(&severity.as_str()) {
        return Err(AppError::Validation(
            "severity must be LOW, MEDIUM, HIGH, or CRITICAL".into(),
        ));
    }

    // Look up DRAFT status_id from entity_statuses
    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    // Insert the new quality rule
    let rule = sqlx::query_as::<_, QualityRule>(
        r#"
        INSERT INTO quality_rules (
            rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage, severity,
            is_active, status_id, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, TRUE, $11, $12)
        RETURNING
            rule_id, rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage, severity,
            is_active, status_id, owner_user_id,
            created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(&rule_name)
    .bind(&rule_code)
    .bind(&description)
    .bind(body.dimension_id)
    .bind(body.rule_type_id)
    .bind(body.element_id)
    .bind(body.column_id)
    .bind(&body.rule_definition)
    .bind(body.threshold_percentage)
    .bind(&severity)
    .bind(draft_status_id)
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    // Initiate the workflow instance for this new quality rule
    workflow::service::initiate_workflow(
        &state.pool,
        workflow::ENTITY_QUALITY_RULE,
        rule.rule_id,
        claims.sub,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(rule)))
}

// ---------------------------------------------------------------------------
// update_rule — PUT /api/v1/data-quality/rules/:rule_id
// ---------------------------------------------------------------------------

/// Update an existing quality rule. Only provided fields are changed.
/// Requires authentication.
#[utoipa::path(
    put,
    path = "/api/v1/data-quality/rules/{rule_id}",
    params(("rule_id" = Uuid, Path, description = "Rule ID")),
    request_body = UpdateQualityRuleRequest,
    responses(
        (status = 200, description = "Rule updated", body = QualityRule),
        (status = 404, description = "Rule not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn update_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateQualityRuleRequest>,
) -> AppResult<Json<QualityRule>> {
    // Verify the rule exists and is not deleted
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM quality_rules WHERE rule_id = $1 AND deleted_at IS NULL)",
    )
    .bind(rule_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "quality rule not found: {rule_id}"
        )));
    }

    // Validate severity if provided
    if let Some(ref severity) = body.severity
        && !["LOW", "MEDIUM", "HIGH", "CRITICAL"].contains(&severity.as_str())
    {
        return Err(AppError::Validation(
            "severity must be LOW, MEDIUM, HIGH, or CRITICAL".into(),
        ));
    }

    // Update using COALESCE to only change provided fields
    let rule = sqlx::query_as::<_, QualityRule>(
        r#"
        UPDATE quality_rules
        SET rule_name            = COALESCE($1, rule_name),
            rule_code            = COALESCE($2, rule_code),
            description          = COALESCE($3, description),
            dimension_id         = COALESCE($4, dimension_id),
            rule_type_id         = COALESCE($5, rule_type_id),
            element_id           = COALESCE($6, element_id),
            column_id            = COALESCE($7, column_id),
            rule_definition      = COALESCE($8, rule_definition),
            threshold_percentage = COALESCE($9, threshold_percentage),
            severity             = COALESCE($10, severity),
            is_active            = COALESCE($11, is_active),
            owner_user_id        = COALESCE($12, owner_user_id),
            updated_by           = $13,
            updated_at           = CURRENT_TIMESTAMP
        WHERE rule_id = $14 AND deleted_at IS NULL
        RETURNING
            rule_id, rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage, severity,
            is_active, status_id, owner_user_id,
            created_by, updated_by, created_at, updated_at
        "#,
    )
    .bind(body.rule_name.as_deref())
    .bind(body.rule_code.as_deref())
    .bind(body.description.as_deref())
    .bind(body.dimension_id)
    .bind(body.rule_type_id)
    .bind(body.element_id)
    .bind(body.column_id)
    .bind(&body.rule_definition)
    .bind(body.threshold_percentage)
    .bind(body.severity.as_deref())
    .bind(body.is_active)
    .bind(body.owner_user_id)
    .bind(claims.sub)
    .bind(rule_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(rule))
}

// ---------------------------------------------------------------------------
// get_assessments — GET /api/v1/data-quality/assessments/:rule_id
// ---------------------------------------------------------------------------

/// Retrieve the assessment history for a quality rule.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-quality/assessments/{rule_id}",
    params(("rule_id" = Uuid, Path, description = "Rule ID")),
    responses(
        (status = 200, description = "Assessment history for a rule",
         body = Vec<QualityAssessment>),
        (status = 404, description = "Rule not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn get_assessments(
    State(state): State<AppState>,
    Path(rule_id): Path<Uuid>,
) -> AppResult<Json<Vec<QualityAssessment>>> {
    // Verify the rule exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM quality_rules WHERE rule_id = $1 AND deleted_at IS NULL)",
    )
    .bind(rule_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "quality rule not found: {rule_id}"
        )));
    }

    let assessments = sqlx::query_as::<_, QualityAssessment>(
        r#"
        SELECT
            assessment_id, rule_id, assessed_at,
            records_assessed, records_passed, records_failed,
            score_percentage, status, error_message, details,
            executed_by, created_at
        FROM quality_assessments
        WHERE rule_id = $1
        ORDER BY assessed_at DESC
        "#,
    )
    .bind(rule_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(assessments))
}

// ---------------------------------------------------------------------------
// create_assessment — POST /api/v1/data-quality/assessments
// ---------------------------------------------------------------------------

/// Record a quality assessment result and update the associated element's quality score.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-quality/assessments",
    request_body = CreateAssessmentRequest,
    responses(
        (status = 201, description = "Assessment recorded", body = QualityAssessment),
        (status = 404, description = "Rule not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn create_assessment(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateAssessmentRequest>,
) -> AppResult<(StatusCode, Json<QualityAssessment>)> {
    // Validate basic fields
    if body.records_assessed < 0 {
        return Err(AppError::Validation(
            "records_assessed must be non-negative".into(),
        ));
    }
    if body.records_passed < 0 {
        return Err(AppError::Validation(
            "records_passed must be non-negative".into(),
        ));
    }
    if body.records_failed < 0 {
        return Err(AppError::Validation(
            "records_failed must be non-negative".into(),
        ));
    }

    // Verify the rule exists and get its element_id and dimension_id
    let rule = sqlx::query_as::<_, QualityRule>(
        r#"
        SELECT
            rule_id, rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage, severity,
            is_active, status_id, owner_user_id,
            created_by, updated_by, created_at, updated_at
        FROM quality_rules
        WHERE rule_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(body.rule_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("quality rule not found: {}", body.rule_id)))?;

    // Insert the assessment
    let assessment = sqlx::query_as::<_, QualityAssessment>(
        r#"
        INSERT INTO quality_assessments (
            rule_id, records_assessed, records_passed,
            records_failed, score_percentage, status,
            details, executed_by
        )
        VALUES ($1, $2, $3, $4, $5, 'COMPLETED', $6, $7)
        RETURNING
            assessment_id, rule_id, assessed_at,
            records_assessed, records_passed, records_failed,
            score_percentage, status, error_message, details,
            executed_by, created_at
        "#,
    )
    .bind(body.rule_id)
    .bind(body.records_assessed)
    .bind(body.records_passed)
    .bind(body.records_failed)
    .bind(body.score_percentage)
    .bind(&body.details)
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    // Update or create quality_scores for the associated element (if linked)
    if let Some(element_id) = rule.element_id {
        let now = chrono::Utc::now();
        // Use the current day as the period
        let period_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight (00:00:00) is always valid")
            .and_utc();
        let period_end = now
            .date_naive()
            .and_hms_opt(23, 59, 59)
            .expect("end of day (23:59:59) is always valid")
            .and_utc();

        // Insert a quality_scores record for this element + dimension + period
        sqlx::query(
            r#"
            INSERT INTO quality_scores (element_id, dimension_id, overall_score, period_start, period_end)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(element_id)
        .bind(rule.dimension_id)
        .bind(body.score_percentage)
        .bind(period_start)
        .bind(period_end)
        .execute(&state.pool)
        .await?;
    }

    Ok((StatusCode::CREATED, Json(assessment)))
}

// ---------------------------------------------------------------------------
// get_element_scores — GET /api/v1/data-quality/scores/element/:element_id
// ---------------------------------------------------------------------------

/// Retrieve quality scores per dimension for a data element.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/data-quality/scores/element/{element_id}",
    params(("element_id" = Uuid, Path, description = "Element ID")),
    responses(
        (status = 200, description = "Quality scores per dimension for the element",
         body = Vec<QualityScoreWithDimension>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn get_element_scores(
    State(state): State<AppState>,
    Path(element_id): Path<Uuid>,
) -> AppResult<Json<Vec<QualityScoreWithDimension>>> {
    let scores = sqlx::query_as::<_, QualityScoreWithDimension>(
        r#"
        SELECT
            qs.score_id,
            qs.element_id,
            qs.table_id,
            qs.dimension_id,
            qd.dimension_name             AS dimension_name,
            qd.dimension_code             AS dimension_code,
            qs.overall_score,
            qs.period_start,
            qs.period_end,
            qs.created_at
        FROM quality_scores qs
        LEFT JOIN quality_dimensions qd ON qd.dimension_id = qs.dimension_id
        WHERE qs.element_id = $1
        ORDER BY qd.display_order ASC, qs.period_end DESC
        "#,
    )
    .bind(element_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(scores))
}
