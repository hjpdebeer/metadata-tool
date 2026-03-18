use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DataElement {
    pub element_id: Uuid,
    pub element_name: String,
    pub element_code: String,
    pub description: String,
    pub business_definition: Option<String>,
    pub business_rules: Option<String>,
    pub data_type: String,
    pub format_pattern: Option<String>,
    pub allowed_values: Option<serde_json::Value>,
    pub default_value: Option<String>,
    pub is_nullable: bool,
    pub is_cde: bool,
    pub cde_rationale: Option<String>,
    pub cde_designated_at: Option<DateTime<Utc>>,
    pub glossary_term_id: Option<Uuid>,
    pub domain_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,
    pub sensitivity_level: Option<String>,
    pub status_id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDataElementRequest {
    pub element_name: String,
    pub element_code: String,
    pub description: String,
    pub business_definition: Option<String>,
    pub business_rules: Option<String>,
    pub data_type: String,
    pub format_pattern: Option<String>,
    pub allowed_values: Option<serde_json::Value>,
    pub default_value: Option<String>,
    pub is_nullable: Option<bool>,
    pub glossary_term_id: Option<Uuid>,
    pub domain_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,
    pub sensitivity_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SourceSystem {
    pub system_id: Uuid,
    pub system_name: String,
    pub system_code: String,
    pub system_type: String,
    pub description: Option<String>,
    pub connection_details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TechnicalColumn {
    pub column_id: Uuid,
    pub table_id: Uuid,
    pub column_name: String,
    pub ordinal_position: i32,
    pub data_type: String,
    pub max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub element_id: Option<Uuid>,
    pub naming_standard_compliant: Option<bool>,
    pub naming_standard_violation: Option<String>,
}

/// Response combining business and technical metadata for a data element
#[derive(Debug, Serialize, ToSchema)]
pub struct DataElementFullView {
    #[serde(flatten)]
    pub element: DataElement,
    pub glossary_term_name: Option<String>,
    pub technical_columns: Vec<TechnicalColumn>,
    pub quality_rules_count: i64,
    pub linked_processes_count: i64,
    pub linked_applications_count: i64,
}
