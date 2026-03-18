use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::ai::*;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Helper: fetch entity data as JSON for the AI prompt
// ---------------------------------------------------------------------------

async fn fetch_entity_data(
    pool: &sqlx::PgPool,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<(serde_json::Value, Vec<String>), AppError> {
    match entity_type {
        "glossary_term" => {
            let row = sqlx::query_as::<_, crate::domain::glossary::GlossaryTerm>(
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
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("glossary term not found: {entity_id}"))
            })?;

            let json = serde_json::to_value(&row)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("serialization error: {e}")))?;

            // Track which fields already have values
            let mut existing = Vec::new();
            if !row.term_name.is_empty() {
                existing.push("term_name".to_string());
            }
            if !row.definition.is_empty() {
                existing.push("definition".to_string());
            }
            if row.business_context.is_some() {
                existing.push("business_context".to_string());
            }
            if row.examples.is_some() {
                existing.push("examples".to_string());
            }
            if row.abbreviation.is_some() {
                existing.push("abbreviation".to_string());
            }
            if row.source_reference.is_some() {
                existing.push("source_reference".to_string());
            }
            if row.regulatory_reference.is_some() {
                existing.push("regulatory_reference".to_string());
            }

            Ok((json, existing))
        }
        "data_element" => {
            let row = sqlx::query_as::<_, crate::domain::data_dictionary::DataElement>(
                r#"
                SELECT
                    element_id, element_name, element_code, description,
                    business_definition, business_rules, data_type,
                    format_pattern, allowed_values, default_value,
                    is_nullable, is_cde, cde_rationale, cde_designated_at,
                    glossary_term_id, domain_id, classification_id,
                    sensitivity_level, status_id, owner_user_id,
                    steward_user_id, created_by, updated_by,
                    created_at, updated_at
                FROM data_elements
                WHERE element_id = $1 AND deleted_at IS NULL
                "#,
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("data element not found: {entity_id}"))
            })?;

            let json = serde_json::to_value(&row)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("serialization error: {e}")))?;

            let mut existing = Vec::new();
            if !row.element_name.is_empty() {
                existing.push("element_name".to_string());
            }
            if !row.description.is_empty() {
                existing.push("description".to_string());
            }
            if row.business_definition.is_some() {
                existing.push("business_definition".to_string());
            }
            if row.business_rules.is_some() {
                existing.push("business_rules".to_string());
            }
            if row.format_pattern.is_some() {
                existing.push("format_pattern".to_string());
            }
            if row.sensitivity_level.is_some() {
                existing.push("sensitivity_level".to_string());
            }

            Ok((json, existing))
        }
        _ => Err(AppError::Validation(format!(
            "Unsupported entity type for AI enrichment: {entity_type}. Supported types: glossary_term, data_element"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Helper: apply accepted suggestion value to the entity
// ---------------------------------------------------------------------------

async fn apply_suggestion_to_entity(
    pool: &sqlx::PgPool,
    entity_type: &str,
    entity_id: Uuid,
    field_name: &str,
    value: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    match entity_type {
        "glossary_term" => {
            // Only allow updating known text fields
            let column = match field_name {
                "definition" | "business_context" | "examples" | "abbreviation"
                | "source_reference" | "regulatory_reference" => field_name,
                _ => {
                    return Err(AppError::Validation(format!(
                        "Cannot auto-apply suggestion to field '{field_name}' on glossary_term"
                    )));
                }
            };
            let query = format!(
                r#"UPDATE glossary_terms
                   SET {column} = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP
                   WHERE term_id = $3 AND deleted_at IS NULL"#,
                column = column,
            );
            sqlx::query(&query)
                .bind(value)
                .bind(user_id)
                .bind(entity_id)
                .execute(pool)
                .await?;
        }
        "data_element" => {
            let column = match field_name {
                "description" | "business_definition" | "business_rules" | "format_pattern"
                | "sensitivity_level" | "default_value" => field_name,
                _ => {
                    return Err(AppError::Validation(format!(
                        "Cannot auto-apply suggestion to field '{field_name}' on data_element"
                    )));
                }
            };
            let query = format!(
                r#"UPDATE data_elements
                   SET {column} = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP
                   WHERE element_id = $3 AND deleted_at IS NULL"#,
                column = column,
            );
            sqlx::query(&query)
                .bind(value)
                .bind(user_id)
                .bind(entity_id)
                .execute(pool)
                .await?;
        }
        _ => {
            return Err(AppError::Validation(format!(
                "Cannot apply suggestion to unsupported entity type: {entity_type}"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// enrich — POST /api/v1/ai/enrich
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/enrich",
    request_body = AiEnrichRequest,
    responses(
        (status = 200, description = "AI enrichment suggestions", body = AiEnrichResponse),
        (status = 404, description = "Entity not found"),
        (status = 502, description = "AI service error")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn enrich(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<AiEnrichRequest>,
) -> AppResult<Json<AiEnrichResponse>> {
    // Fetch entity data
    let (entity_data, existing_fields) =
        fetch_entity_data(&state.pool, &body.entity_type, body.entity_id).await?;

    // Call AI enrichment service
    let result = crate::ai::enrich_entity(
        &state.config.ai,
        &body.entity_type,
        entity_data,
        existing_fields,
    )
    .await?;

    // Store suggestions in the database
    let mut suggestion_responses = Vec::new();

    for raw in &result.suggestions {
        let row = sqlx::query_as::<_, AiSuggestion>(
            r#"
            INSERT INTO ai_suggestions (
                entity_type, entity_id, field_name, suggested_value,
                confidence, rationale, source, model, status
            )
            VALUES ($1, $2, $3, $4, $5::FLOAT8::NUMERIC(3,2), $6, $7, $8, 'PENDING')
            RETURNING
                suggestion_id, entity_type, entity_id, field_name,
                suggested_value, confidence::FLOAT8 AS confidence,
                rationale, source, model, status,
                accepted_by, accepted_at, created_at
            "#,
        )
        .bind(&body.entity_type)
        .bind(body.entity_id)
        .bind(&raw.field_name)
        .bind(&raw.suggested_value)
        .bind(raw.confidence)
        .bind(&raw.rationale)
        .bind(&result.provider)
        .bind(&result.model)
        .fetch_one(&state.pool)
        .await?;

        suggestion_responses.push(AiSuggestionResponse {
            suggestion_id: row.suggestion_id,
            field_name: row.field_name,
            suggested_value: row.suggested_value,
            confidence: row.confidence.unwrap_or(0.0),
            rationale: row.rationale.unwrap_or_default(),
            source: row.source,
            model: row.model,
            status: row.status,
            created_at: row.created_at,
        });
    }

    // Log the enrichment action
    tracing::info!(
        entity_type = %body.entity_type,
        entity_id = %body.entity_id,
        user_id = %claims.sub,
        provider = %result.provider,
        suggestion_count = suggestion_responses.len(),
        "AI enrichment completed"
    );

    Ok(Json(AiEnrichResponse {
        entity_type: body.entity_type,
        entity_id: body.entity_id,
        suggestions: suggestion_responses,
        provider: result.provider,
        model: result.model,
    }))
}

// ---------------------------------------------------------------------------
// list_suggestions — GET /api/v1/ai/suggestions/:entity_type/:entity_id
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/ai/suggestions/{entity_type}/{entity_id}",
    params(
        ("entity_type" = String, Path, description = "Entity type (glossary_term, data_element)"),
        ("entity_id" = Uuid, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "List of suggestions for entity", body = Vec<AiSuggestionResponse>)
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn list_suggestions(
    State(state): State<AppState>,
    Path((entity_type, entity_id)): Path<(String, Uuid)>,
) -> AppResult<Json<Vec<AiSuggestionResponse>>> {
    let rows = sqlx::query_as::<_, AiSuggestion>(
        r#"
        SELECT
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        FROM ai_suggestions
        WHERE entity_type = $1 AND entity_id = $2
        ORDER BY created_at DESC
        "#,
    )
    .bind(&entity_type)
    .bind(entity_id)
    .fetch_all(&state.pool)
    .await?;

    let responses: Vec<AiSuggestionResponse> = rows
        .into_iter()
        .map(|row| AiSuggestionResponse {
            suggestion_id: row.suggestion_id,
            field_name: row.field_name,
            suggested_value: row.suggested_value,
            confidence: row.confidence.unwrap_or(0.0),
            rationale: row.rationale.unwrap_or_default(),
            source: row.source,
            model: row.model,
            status: row.status,
            created_at: row.created_at,
        })
        .collect();

    Ok(Json(responses))
}

// ---------------------------------------------------------------------------
// accept_suggestion — POST /api/v1/ai/suggestions/:suggestion_id/accept
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/suggestions/{suggestion_id}/accept",
    params(("suggestion_id" = Uuid, Path, description = "Suggestion ID")),
    request_body = AcceptSuggestionRequest,
    responses(
        (status = 200, description = "Suggestion accepted and applied", body = AiSuggestionResponse),
        (status = 404, description = "Suggestion not found")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn accept_suggestion(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
    Json(body): Json<AcceptSuggestionRequest>,
) -> AppResult<Json<AiSuggestionResponse>> {
    // Fetch the suggestion
    let suggestion = sqlx::query_as::<_, AiSuggestion>(
        r#"
        SELECT
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        FROM ai_suggestions
        WHERE suggestion_id = $1
        "#,
    )
    .bind(suggestion_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("suggestion not found: {suggestion_id}")))?;

    if suggestion.status != "PENDING" {
        return Err(AppError::Validation(format!(
            "Suggestion is already {}, cannot accept",
            suggestion.status
        )));
    }

    // Determine the value to apply
    let final_value = body
        .modified_value
        .as_deref()
        .unwrap_or(&suggestion.suggested_value);
    let new_status = if body.modified_value.is_some() {
        "MODIFIED"
    } else {
        "ACCEPTED"
    };

    // Apply the value to the entity
    apply_suggestion_to_entity(
        &state.pool,
        &suggestion.entity_type,
        suggestion.entity_id,
        &suggestion.field_name,
        final_value,
        claims.sub,
    )
    .await?;

    // Update the suggestion status
    let updated = sqlx::query_as::<_, AiSuggestion>(
        r#"
        UPDATE ai_suggestions
        SET status = $1,
            suggested_value = $2,
            accepted_by = $3,
            accepted_at = CURRENT_TIMESTAMP
        WHERE suggestion_id = $4
        RETURNING
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        "#,
    )
    .bind(new_status)
    .bind(final_value)
    .bind(claims.sub)
    .bind(suggestion_id)
    .fetch_one(&state.pool)
    .await?;

    tracing::info!(
        suggestion_id = %suggestion_id,
        status = new_status,
        user_id = %claims.sub,
        "AI suggestion accepted"
    );

    Ok(Json(AiSuggestionResponse {
        suggestion_id: updated.suggestion_id,
        field_name: updated.field_name,
        suggested_value: updated.suggested_value,
        confidence: updated.confidence.unwrap_or(0.0),
        rationale: updated.rationale.unwrap_or_default(),
        source: updated.source,
        model: updated.model,
        status: updated.status,
        created_at: updated.created_at,
    }))
}

// ---------------------------------------------------------------------------
// reject_suggestion — POST /api/v1/ai/suggestions/:suggestion_id/reject
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/suggestions/{suggestion_id}/reject",
    params(("suggestion_id" = Uuid, Path, description = "Suggestion ID")),
    responses(
        (status = 200, description = "Suggestion rejected", body = AiSuggestionResponse),
        (status = 404, description = "Suggestion not found")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn reject_suggestion(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
) -> AppResult<Json<AiSuggestionResponse>> {
    // Verify and update in one query
    let updated = sqlx::query_as::<_, AiSuggestion>(
        r#"
        UPDATE ai_suggestions
        SET status = 'REJECTED',
            accepted_by = $1,
            accepted_at = CURRENT_TIMESTAMP
        WHERE suggestion_id = $2 AND status = 'PENDING'
        RETURNING
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        "#,
    )
    .bind(claims.sub)
    .bind(suggestion_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "Pending suggestion not found: {suggestion_id}"
        ))
    })?;

    tracing::info!(
        suggestion_id = %suggestion_id,
        user_id = %claims.sub,
        "AI suggestion rejected"
    );

    Ok(Json(AiSuggestionResponse {
        suggestion_id: updated.suggestion_id,
        field_name: updated.field_name,
        suggested_value: updated.suggested_value,
        confidence: updated.confidence.unwrap_or(0.0),
        rationale: updated.rationale.unwrap_or_default(),
        source: updated.source,
        model: updated.model,
        status: updated.status,
        created_at: updated.created_at,
    }))
}

// ---------------------------------------------------------------------------
// submit_feedback — POST /api/v1/ai/suggestions/:suggestion_id/feedback
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/suggestions/{suggestion_id}/feedback",
    params(("suggestion_id" = Uuid, Path, description = "Suggestion ID")),
    request_body = FeedbackRequest,
    responses(
        (status = 200, description = "Feedback recorded", body = FeedbackResponse),
        (status = 404, description = "Suggestion not found")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn submit_feedback(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
    Json(body): Json<FeedbackRequest>,
) -> AppResult<Json<FeedbackResponse>> {
    // Validate rating if provided
    if let Some(rating) = body.rating {
        if !(1..=5).contains(&rating) {
            return Err(AppError::Validation(
                "Rating must be between 1 and 5".into(),
            ));
        }
    }

    // Verify suggestion exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM ai_suggestions WHERE suggestion_id = $1)",
    )
    .bind(suggestion_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "suggestion not found: {suggestion_id}"
        )));
    }

    // Insert feedback
    let feedback_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_feedback (suggestion_id, user_id, rating, feedback_text)
        VALUES ($1, $2, $3, $4)
        RETURNING feedback_id
        "#,
    )
    .bind(suggestion_id)
    .bind(claims.sub)
    .bind(body.rating)
    .bind(body.feedback_text.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(FeedbackResponse {
        feedback_id,
        suggestion_id,
        message: "Feedback recorded successfully".to_string(),
    }))
}
