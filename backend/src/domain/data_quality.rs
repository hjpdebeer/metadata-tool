use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// The 6 data quality dimensions (DAMA framework)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct QualityDimension {
    pub dimension_id: Uuid,
    pub dimension_code: String,
    pub dimension_name: String,
    pub description: String,
}

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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
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
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct QualityScore {
    pub score_id: Uuid,
    pub element_id: Option<Uuid>,
    pub table_id: Option<Uuid>,
    pub dimension_id: Option<Uuid>,
    pub overall_score: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}
