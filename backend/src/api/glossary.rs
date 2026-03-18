use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::db::AppState;
use crate::domain::glossary::*;
use crate::error::AppResult;

#[utoipa::path(
    get,
    path = "/api/v1/glossary/terms",
    params(SearchGlossaryTermsRequest),
    responses(
        (status = 200, description = "List glossary terms", body = Vec<GlossaryTerm>)
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn list_terms(
    State(_state): State<AppState>,
    Query(_params): Query<SearchGlossaryTermsRequest>,
) -> AppResult<Json<Vec<GlossaryTerm>>> {
    // TODO: Query database with filters and pagination
    Ok(Json(vec![]))
}

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
    State(_state): State<AppState>,
    Path(_term_id): Path<Uuid>,
) -> AppResult<Json<GlossaryTerm>> {
    // TODO: Fetch term by ID
    Err(crate::error::AppError::NotFound("Term not found".into()))
}

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
    State(_state): State<AppState>,
    Json(_body): Json<CreateGlossaryTermRequest>,
) -> AppResult<Json<GlossaryTerm>> {
    // TODO: Validate, create term in DRAFT state, trigger AI enrichment, initiate workflow
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

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
    State(_state): State<AppState>,
    Path(_term_id): Path<Uuid>,
    Json(_body): Json<UpdateGlossaryTermRequest>,
) -> AppResult<Json<GlossaryTerm>> {
    // TODO: Update term, record audit, handle workflow state
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

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
    State(_state): State<AppState>,
) -> AppResult<Json<Vec<GlossaryDomain>>> {
    // TODO: Return all domains
    Ok(Json(vec![]))
}

#[utoipa::path(
    post,
    path = "/api/v1/glossary/terms/{term_id}/ai-enrich",
    params(("term_id" = Uuid, Path, description = "Term ID")),
    responses(
        (status = 200, description = "AI enrichment suggestions generated")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn ai_enrich_term(
    State(_state): State<AppState>,
    Path(_term_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    // TODO: Call AI service to generate suggestions for this term
    Ok(Json(serde_json::json!({
        "status": "not_implemented",
        "message": "AI enrichment will be available once AI integration is configured"
    })))
}
