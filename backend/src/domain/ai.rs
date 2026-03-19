use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// AI Suggestion (maps to ai_suggestions table)
// ---------------------------------------------------------------------------

/// Row type mapping directly to the `ai_suggestions` table.
/// The `confidence` column is NUMERIC(3,2) in PostgreSQL; sqlx maps it
/// through the `rust_decimal` or `bigdecimal` feature, but since neither
/// is enabled we read it via a wrapper query that casts to FLOAT8.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AiSuggestion {
    pub suggestion_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub field_name: String,
    pub suggested_value: String,
    pub confidence: Option<f64>,
    pub rationale: Option<String>,
    pub source: String,
    pub model: Option<String>,
    pub status: String,
    pub accepted_by: Option<Uuid>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

/// Response returned after AI enrichment — includes all generated suggestions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiEnrichResponse {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub suggestions: Vec<AiSuggestionResponse>,
    pub provider: String,
    pub model: String,
}

/// Single suggestion with display-friendly fields
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiSuggestionResponse {
    pub suggestion_id: Uuid,
    pub field_name: String,
    pub suggested_value: String,
    pub confidence: f64,
    pub rationale: String,
    pub source: String,
    pub model: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for POST /api/v1/ai/enrich
#[derive(Debug, Deserialize, ToSchema)]
pub struct AiEnrichRequest {
    /// Entity type: "glossary_term", "data_element", etc.
    pub entity_type: String,
    /// ID of the entity to enrich
    pub entity_id: Uuid,
}

/// Request body for accepting a suggestion (optional modified value)
#[derive(Debug, Deserialize, ToSchema)]
pub struct AcceptSuggestionRequest {
    /// If provided, use this value instead of the original suggestion
    pub modified_value: Option<String>,
}

/// Request body for rejecting a suggestion (optional reason)
#[derive(Debug, Deserialize, ToSchema)]
pub struct RejectSuggestionRequest {
    pub reason: Option<String>,
}

/// Request body for submitting feedback on a suggestion
#[derive(Debug, Deserialize, ToSchema)]
pub struct FeedbackRequest {
    /// Rating from 1 to 5
    pub rating: Option<i32>,
    /// Free-text feedback
    pub feedback_text: Option<String>,
}

/// Response after feedback is recorded
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeedbackResponse {
    pub feedback_id: Uuid,
    pub suggestion_id: Uuid,
    pub message: String,
}
