use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Quality dimensions (DAMA framework)
// ---------------------------------------------------------------------------

/// The 6 data quality dimensions (DAMA framework)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct QualityDimension {
    pub dimension_id: Uuid,
    pub dimension_code: String,
    pub dimension_name: String,
    pub description: String,
}

/// Dimension summary with aggregate stats
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct QualityDimensionSummary {
    pub dimension_id: Uuid,
    pub dimension_code: String,
    pub dimension_name: String,
    pub description: String,
    pub rules_count: i64,
    pub avg_score: Option<f64>,
    pub last_assessed_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Quality rule types
// ---------------------------------------------------------------------------

/// A quality rule type template (e.g. completeness check, range validation). Maps to the `quality_rule_types` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct QualityRuleType {
    pub rule_type_id: Uuid,
    pub type_code: String,
    pub type_name: String,
    pub description: Option<String>,
    pub sql_template: Option<String>,
}

// ---------------------------------------------------------------------------
// Full quality rule (single-record detail view)
// ---------------------------------------------------------------------------

/// A data quality rule defining a specific check against a data element or column. Maps to the `quality_rules` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct QualityRule {
    pub rule_id: Uuid,
    pub rule_name: String,
    pub rule_code: String,
    pub description: String,
    pub dimension_id: Uuid,
    pub rule_type_id: Uuid,
    pub element_id: Option<Uuid>,
    pub column_id: Option<Uuid>,
    pub rule_definition: serde_json::Value,
    pub threshold_percentage: Option<f64>,
    pub severity: String,
    pub is_active: bool,
    pub status_id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// List view (joined fields for display in tables/lists)
// ---------------------------------------------------------------------------

/// List view of a quality rule with joined fields for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct QualityRuleListItem {
    pub rule_id: Uuid,
    pub rule_name: String,
    pub rule_code: String,
    pub description: String,
    pub dimension_name: String,
    pub dimension_code: String,
    pub rule_type_name: String,
    pub element_name: Option<String>,
    pub severity: String,
    pub is_active: bool,
    pub status_code: String,
    pub status_name: String,
    pub owner_name: Option<String>,
    pub threshold_percentage: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Paginated response
// ---------------------------------------------------------------------------

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedQualityRules {
    pub data: Vec<QualityRuleListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Assessments
// ---------------------------------------------------------------------------

/// A recorded execution result for a quality rule. Maps to the `quality_assessments` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct QualityAssessment {
    pub assessment_id: Uuid,
    pub rule_id: Uuid,
    pub assessed_at: DateTime<Utc>,
    pub records_assessed: i64,
    pub records_passed: i64,
    pub records_failed: i64,
    pub score_percentage: f64,
    pub status: String,
    pub error_message: Option<String>,
    pub details: Option<serde_json::Value>,
    pub executed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Scores
// ---------------------------------------------------------------------------

/// An aggregated quality score for an element or table over a time period. Maps to the `quality_scores` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct QualityScore {
    pub score_id: Uuid,
    pub element_id: Option<Uuid>,
    pub table_id: Option<Uuid>,
    pub dimension_id: Option<Uuid>,
    pub overall_score: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Quality score with dimension name for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct QualityScoreWithDimension {
    pub score_id: Uuid,
    pub element_id: Option<Uuid>,
    pub table_id: Option<Uuid>,
    pub dimension_id: Option<Uuid>,
    pub dimension_name: Option<String>,
    pub dimension_code: Option<String>,
    pub overall_score: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Element quality overview
// ---------------------------------------------------------------------------

/// Overview of quality scores per dimension for a data element
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ElementQualityOverview {
    pub element_id: Uuid,
    pub element_name: String,
    pub dimension_scores: Vec<QualityScoreWithDimension>,
    pub overall_score: Option<f64>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for creating a new quality rule.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateQualityRuleRequest {
    pub rule_name: String,
    pub rule_code: String,
    pub description: String,
    pub dimension_id: Uuid,
    pub rule_type_id: Uuid,
    pub element_id: Option<Uuid>,
    pub column_id: Option<Uuid>,
    pub rule_definition: serde_json::Value,
    pub threshold_percentage: Option<f64>,
    pub severity: Option<String>,
}

/// Request body for partially updating a quality rule. All fields are optional.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateQualityRuleRequest {
    pub rule_name: Option<String>,
    pub rule_code: Option<String>,
    pub description: Option<String>,
    pub dimension_id: Option<Uuid>,
    pub rule_type_id: Option<Uuid>,
    pub element_id: Option<Uuid>,
    pub column_id: Option<Uuid>,
    pub rule_definition: Option<serde_json::Value>,
    pub threshold_percentage: Option<f64>,
    pub severity: Option<String>,
    pub is_active: Option<bool>,
    pub owner_user_id: Option<Uuid>,
}

/// Request body for recording a quality assessment result.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateAssessmentRequest {
    pub rule_id: Uuid,
    pub records_assessed: i64,
    pub records_passed: i64,
    pub records_failed: i64,
    pub score_percentage: f64,
    pub details: Option<serde_json::Value>,
}

/// Query parameters for searching and filtering quality rules with pagination.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct SearchQualityRulesRequest {
    pub query: Option<String>,
    pub dimension_id: Option<Uuid>,
    pub element_id: Option<Uuid>,
    pub severity: Option<String>,
    pub is_active: Option<bool>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
