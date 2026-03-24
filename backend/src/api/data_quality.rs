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
                AVG(latest.score_percentage)::FLOAT8  AS avg_score,
                MAX(latest.assessed_at)       AS last_assessed
            FROM quality_rules qr
            JOIN LATERAL (
                SELECT score_percentage::FLOAT8 AS score_percentage, assessed_at
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
        WHERE qr.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR qr.rule_name ILIKE '%' || $1 || '%'
               OR qr.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR qr.dimension_id = $2)
          AND ($3::UUID IS NULL OR qr.element_id = $3)
          AND ($4::TEXT IS NULL OR qr.severity = $4)
          AND ($5::BOOL IS NULL OR qr.is_active = $5)
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.dimension_id)
    .bind(params.element_id)
    .bind(params.severity.as_deref())
    .bind(params.is_active)
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
            uo.display_name               AS owner_name,
            qr.threshold_percentage::FLOAT8 AS threshold_percentage,
            qr.scope,
            qr.check_frequency,
            qr.created_at,
            qr.updated_at
        FROM quality_rules qr
        JOIN quality_dimensions qd ON qd.dimension_id = qr.dimension_id
        JOIN quality_rule_types qrt ON qrt.rule_type_id = qr.rule_type_id
        LEFT JOIN data_elements de ON de.element_id = qr.element_id
        LEFT JOIN users uo ON uo.user_id = qr.owner_user_id
        WHERE qr.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR qr.rule_name ILIKE '%' || $1 || '%'
               OR qr.description ILIKE '%' || $1 || '%')
          AND ($2::UUID IS NULL OR qr.dimension_id = $2)
          AND ($3::UUID IS NULL OR qr.element_id = $3)
          AND ($4::TEXT IS NULL OR qr.severity = $4)
          AND ($5::BOOL IS NULL OR qr.is_active = $5)
        ORDER BY qr.rule_name ASC
        LIMIT $6
        OFFSET $7
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.dimension_id)
    .bind(params.element_id)
    .bind(params.severity.as_deref())
    .bind(params.is_active)
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
            qr.rule_id, qr.rule_name, qr.rule_code, qr.description,
            qr.dimension_id, qr.rule_type_id, qr.element_id, qr.column_id,
            qr.rule_definition, qr.threshold_percentage::FLOAT8 AS threshold_percentage, qr.severity,
            qr.is_active, qr.scope, qr.check_frequency, qr.owner_user_id, qr.deleted_at,
            qr.created_by, qr.updated_by, qr.created_at, qr.updated_at,
            qd.dimension_name,
            qrt.type_name AS rule_type_name,
            de.element_name
        FROM quality_rules qr
        LEFT JOIN quality_dimensions qd ON qd.dimension_id = qr.dimension_id
        LEFT JOIN quality_rule_types qrt ON qrt.rule_type_id = qr.rule_type_id
        LEFT JOIN data_elements de ON de.element_id = qr.element_id
        WHERE qr.rule_id = $1 AND qr.deleted_at IS NULL
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

    // Insert the new quality rule (no status_id — rules inherit parent element's workflow)
    let is_active = body.is_active.unwrap_or(true);
    let rule = sqlx::query_as::<_, QualityRule>(
        r#"
        INSERT INTO quality_rules (
            rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage, severity,
            is_active, scope, check_frequency, owner_user_id, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, COALESCE($12, 'RECORD'), $13, $14, $15)
        RETURNING
            rule_id, rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage::FLOAT8 AS threshold_percentage, severity,
            is_active, scope, check_frequency, owner_user_id, deleted_at,
            created_by, updated_by, created_at, updated_at,
            NULL::VARCHAR AS dimension_name,
            NULL::VARCHAR AS rule_type_name,
            NULL::VARCHAR AS element_name
        "#,
    )
    .bind(&rule_name)           // $1
    .bind(&rule_code)           // $2
    .bind(&description)         // $3
    .bind(body.dimension_id)    // $4
    .bind(body.rule_type_id)    // $5
    .bind(body.element_id)      // $6
    .bind(body.column_id)       // $7
    .bind(&body.rule_definition) // $8
    .bind(body.threshold_percentage) // $9
    .bind(&severity)            // $10
    .bind(is_active)            // $11
    .bind(body.scope.as_deref()) // $12
    .bind(body.check_frequency.as_deref()) // $13
    .bind(body.owner_user_id)   // $14
    .bind(claims.sub)           // $15
    .fetch_one(&state.pool)
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
            scope                = COALESCE($13, scope),
            check_frequency      = COALESCE($14, check_frequency),
            updated_by           = $15,
            updated_at           = CURRENT_TIMESTAMP
        WHERE rule_id = $16 AND deleted_at IS NULL
        RETURNING
            rule_id, rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage::FLOAT8 AS threshold_percentage, severity,
            is_active, scope, check_frequency, owner_user_id, deleted_at,
            created_by, updated_by, created_at, updated_at,
            NULL::VARCHAR AS dimension_name,
            NULL::VARCHAR AS rule_type_name,
            NULL::VARCHAR AS element_name
        "#,
    )
    .bind(body.rule_name.as_deref())     // $1
    .bind(body.rule_code.as_deref())     // $2
    .bind(body.description.as_deref())   // $3
    .bind(body.dimension_id)             // $4
    .bind(body.rule_type_id)             // $5
    .bind(body.element_id)               // $6
    .bind(body.column_id)                // $7
    .bind(&body.rule_definition)         // $8
    .bind(body.threshold_percentage)     // $9
    .bind(body.severity.as_deref())      // $10
    .bind(body.is_active)                // $11
    .bind(body.owner_user_id)            // $12
    .bind(body.scope.as_deref())         // $13
    .bind(body.check_frequency.as_deref()) // $14
    .bind(claims.sub)                    // $15
    .bind(rule_id)                       // $16
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
            score_percentage::FLOAT8 AS score_percentage, status, error_message, details,
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
            rule_definition, threshold_percentage::FLOAT8 AS threshold_percentage, severity,
            is_active, scope, check_frequency, owner_user_id, deleted_at,
            created_by, updated_by, created_at, updated_at,
            NULL::VARCHAR AS dimension_name,
            NULL::VARCHAR AS rule_type_name,
            NULL::VARCHAR AS element_name
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
            score_percentage::FLOAT8 AS score_percentage, status, error_message, details,
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

// ---------------------------------------------------------------------------
// accept_rule_suggestion — POST /api/v1/data-quality/rules/from-suggestion
// ---------------------------------------------------------------------------

/// Accept an AI-suggested quality rule by creating a real quality_rules row.
/// Looks up dimension_id from dimension_code and rule_type_id from comparison_type.
/// The new rule inherits DRAFT status and gets an associated workflow instance.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/data-quality/rules/from-suggestion",
    request_body = crate::domain::ai::AcceptRuleSuggestionRequest,
    responses(
        (status = 201, description = "Quality rule created from AI suggestion", body = QualityRule),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn accept_rule_suggestion(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<crate::domain::ai::AcceptRuleSuggestionRequest>,
) -> AppResult<(StatusCode, Json<QualityRule>)> {
    // Validate required fields
    let rule_name = body.rule_name.trim().to_string();
    if rule_name.is_empty() {
        return Err(AppError::Validation("rule_name is required".into()));
    }
    let description = body.description.trim().to_string();
    if description.is_empty() {
        return Err(AppError::Validation("description is required".into()));
    }

    // Validate severity
    let severity = body.severity.clone();
    if !["LOW", "MEDIUM", "HIGH", "CRITICAL"].contains(&severity.as_str()) {
        return Err(AppError::Validation(
            "severity must be LOW, MEDIUM, HIGH, or CRITICAL".into(),
        ));
    }

    // Look up dimension_id from dimension_code
    let dimension_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT dimension_id FROM quality_dimensions WHERE dimension_code = $1",
    )
    .bind(&body.dimension_code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        AppError::Validation(format!(
            "unknown quality dimension code: {}",
            body.dimension_code
        ))
    })?;

    // Look up a default rule_type_id from comparison_type
    // Map the comparison_type to a type_code in quality_rule_types
    // Map comparison_type to existing quality_rule_types type_codes
    let type_code = match body.comparison_type.as_deref() {
        Some("NOT_NULL") => "NOT_NULL",
        Some("UNIQUE") => "UNIQUE",
        Some("GREATER_THAN") | Some("LESS_THAN") | Some("BETWEEN") => "RANGE",
        Some("EQUAL") | Some("NOT_EQUAL") | Some("IN_LIST") => "REFERENTIAL",
        Some("REGEX") => "PATTERN",
        Some("CUSTOM_SQL") => "CUSTOM_SQL",
        _ => "CUSTOM_SQL",
    };

    let rule_type_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT rule_type_id FROM quality_rule_types WHERE type_code = $1",
    )
    .bind(type_code)
    .fetch_optional(&state.pool)
    .await?
    .unwrap_or_else(|| {
        // If the mapped type_code doesn't exist, this will be handled below
        Uuid::nil()
    });

    // If we couldn't find the rule type, fall back to the first available
    let rule_type_id = if rule_type_id.is_nil() {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT rule_type_id FROM quality_rule_types ORDER BY type_code LIMIT 1",
        )
        .fetch_one(&state.pool)
        .await?
    } else {
        rule_type_id
    };

    // rule_code is auto-generated by DB trigger (QR-{DIMENSION}-{SEQ})

    // Build rule_definition JSON from comparison_type and comparison_value
    let rule_definition = serde_json::json!({
        "comparison_type": body.comparison_type,
        "comparison_value": body.comparison_value,
    });

    // Insert the new quality rule (no status_id — rules inherit parent element's workflow)
    let rule = sqlx::query_as::<_, QualityRule>(
        r#"
        INSERT INTO quality_rules (
            rule_name, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage, severity,
            is_active, created_by,
            rule_expression, comparison_type, comparison_value, scope
        )
        VALUES ($1, $2, $3, $4, $5, NULL, $6, $7, $8, TRUE, $9, $10, $11, $12, 'RECORD')
        RETURNING rule_id, rule_name, rule_code, description,
            dimension_id, rule_type_id, element_id, column_id,
            rule_definition, threshold_percentage::FLOAT8 AS threshold_percentage, severity,
            is_active, scope, check_frequency, owner_user_id, deleted_at,
            created_by, updated_by, created_at, updated_at,
            NULL::VARCHAR AS dimension_name,
            NULL::VARCHAR AS rule_type_name,
            NULL::VARCHAR AS element_name
        "#,
    )
    .bind(&rule_name)       // $1
    .bind(&description)     // $2
    .bind(dimension_id)     // $3
    .bind(rule_type_id)     // $4
    .bind(body.element_id)  // $5
    .bind(&rule_definition) // $6
    .bind(body.threshold_percentage) // $7
    .bind(&severity)        // $8
    .bind(claims.sub)       // $9
    .bind(&description)     // $10 rule_expression
    .bind(body.comparison_type.as_deref()) // $11
    .bind(body.comparison_value.as_deref()) // $12
    .fetch_one(&state.pool)
    .await?;

    // Quality rules inherit the element's workflow — no separate workflow instance.
    // When the element is approved, its rules are approved with it.
    tracing::info!(
        rule_id = %rule.rule_id,
        element_id = %body.element_id,
        user_id = %claims.sub,
        "Quality rule created from AI suggestion"
    );

    Ok((StatusCode::CREATED, Json(rule)))
}

// ---------------------------------------------------------------------------
// delete_rule — DELETE /api/v1/data-quality/rules/:rule_id
// ---------------------------------------------------------------------------

/// Soft-delete a quality rule.
#[utoipa::path(
    delete,
    path = "/api/v1/data-quality/rules/{rule_id}",
    params(("rule_id" = Uuid, Path, description = "Rule ID")),
    responses(
        (status = 204, description = "Rule deleted"),
        (status = 404, description = "Rule not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn delete_rule(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(rule_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let rows = sqlx::query(
        "UPDATE quality_rules SET deleted_at = CURRENT_TIMESTAMP, updated_by = $2 WHERE rule_id = $1 AND deleted_at IS NULL",
    )
    .bind(rule_id)
    .bind(claims.sub)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "quality rule not found: {rule_id}"
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// get_recent_assessments — GET /api/v1/data-quality/assessments/recent
// ---------------------------------------------------------------------------

/// Get recent quality assessments across all rules.
#[utoipa::path(
    get,
    path = "/api/v1/data-quality/assessments/recent",
    params(("limit" = Option<i64>, Query, description = "Max results (default 10)")),
    responses(
        (status = 200, description = "Recent assessments", body = Vec<QualityAssessmentWithRule>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn get_recent_assessments(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> AppResult<Json<Vec<QualityAssessmentWithRule>>> {
    let limit: i64 = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(10)
        .min(50);

    let assessments = sqlx::query_as::<_, QualityAssessmentWithRule>(
        r#"
        SELECT qa.assessment_id, qa.rule_id, qr.rule_name,
               qa.assessed_at, qa.records_assessed, qa.records_passed,
               qa.records_failed, qa.score_percentage::FLOAT8 AS "score_percentage",
               qa.status, qa.created_at
        FROM quality_assessments qa
        JOIN quality_rules qr ON qa.rule_id = qr.rule_id
        WHERE qr.deleted_at IS NULL
        ORDER BY qa.assessed_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(assessments))
}
