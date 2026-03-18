use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::db::AppState;
use crate::error::AppResult;

#[derive(Debug, Deserialize, ToSchema)]
pub struct AiEnrichRequest {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AiSuggestion {
    pub suggestion_id: Uuid,
    pub field_name: String,
    pub suggested_value: String,
    pub confidence: f64,
    pub rationale: String,
    pub source: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AiEnrichResponse {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub suggestions: Vec<AiSuggestion>,
    pub provider: String,
    pub model: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/ai/enrich",
    request_body = AiEnrichRequest,
    responses(
        (status = 200, description = "AI enrichment suggestions", body = AiEnrichResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn enrich(
    State(_state): State<AppState>,
    Json(_body): Json<AiEnrichRequest>,
) -> AppResult<Json<AiEnrichResponse>> {
    // TODO: Call Claude (primary) or OpenAI (fallback) to generate metadata suggestions
    // based on financial services standard definitions
    Err(crate::error::AppError::AiService(
        "AI service not configured yet".into(),
    ))
}
