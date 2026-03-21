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
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AiEnrichRequest {
    /// Entity type: "glossary_term", "data_element", etc.
    pub entity_type: String,
    /// ID of the entity to enrich
    pub entity_id: Uuid,
}

/// Request body for accepting a suggestion (optional modified value)
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AcceptSuggestionRequest {
    /// If provided, use this value instead of the original suggestion
    pub modified_value: Option<String>,
}

/// Request body for rejecting a suggestion (optional reason)
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RejectSuggestionRequest {
    pub reason: Option<String>,
}

/// Request body for submitting feedback on a suggestion
#[derive(Debug, Clone, Deserialize, ToSchema)]
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

// ---------------------------------------------------------------------------
// AI Quality Rule Suggestions (suggest-quality-rules endpoint)
// ---------------------------------------------------------------------------

/// Request body for POST /api/v1/ai/suggest-quality-rules
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AiSuggestRulesRequest {
    pub element_id: Uuid,
}

/// A single AI-suggested quality rule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AiRuleSuggestion {
    /// Quality dimension: COMPLETENESS, UNIQUENESS, VALIDITY, ACCURACY, TIMELINESS, CONSISTENCY
    pub dimension: String,
    /// Short descriptive rule name
    pub rule_name: String,
    /// What the rule checks
    pub description: String,
    /// Comparison type: NOT_NULL, UNIQUE, GREATER_THAN, LESS_THAN, BETWEEN, EQUAL, NOT_EQUAL, REGEX, IN_LIST, CUSTOM_SQL
    pub comparison_type: Option<String>,
    /// The value to compare against
    pub comparison_value: Option<String>,
    /// Pass rate threshold (0-100)
    pub threshold_percentage: f64,
    /// Severity: CRITICAL, HIGH, MEDIUM, LOW
    pub severity: String,
    /// Why this rule, citing standards where applicable
    pub rationale: String,
    /// AI confidence 0.0-1.0
    pub confidence: f64,
}

/// Response from POST /api/v1/ai/suggest-quality-rules
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AiSuggestRulesResponse {
    pub element_id: Uuid,
    pub element_name: String,
    pub suggestions: Vec<AiRuleSuggestion>,
    pub provider: String,
    pub model: String,
}

/// Request body for POST /api/v1/data-quality/rules/from-suggestion
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AcceptRuleSuggestionRequest {
    pub element_id: Uuid,
    pub dimension_code: String,
    pub rule_name: String,
    pub description: String,
    pub comparison_type: Option<String>,
    pub comparison_value: Option<String>,
    pub threshold_percentage: f64,
    pub severity: String,
}
