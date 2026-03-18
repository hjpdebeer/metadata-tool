use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Application {
    pub application_id: Uuid,
    pub application_name: String,
    pub application_code: String,
    pub description: String,
    pub classification_id: Option<Uuid>,
    pub status_id: Uuid,
    pub business_owner_id: Option<Uuid>,
    pub technical_owner_id: Option<Uuid>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    pub is_critical: bool,
    pub criticality_rationale: Option<String>,
    pub go_live_date: Option<DateTime<Utc>>,
    pub retirement_date: Option<DateTime<Utc>>,
    pub documentation_url: Option<String>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApplicationRequest {
    pub application_name: String,
    pub application_code: String,
    pub description: String,
    pub classification_id: Option<Uuid>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    pub is_critical: Option<bool>,
    pub criticality_rationale: Option<String>,
    pub go_live_date: Option<DateTime<Utc>>,
    pub documentation_url: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateApplicationRequest {
    pub application_name: Option<String>,
    pub description: Option<String>,
    pub classification_id: Option<Uuid>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    pub is_critical: Option<bool>,
    pub criticality_rationale: Option<String>,
    pub retirement_date: Option<DateTime<Utc>>,
    pub documentation_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApplicationInterface {
    pub interface_id: Uuid,
    pub source_app_id: Uuid,
    pub target_app_id: Uuid,
    pub interface_name: String,
    pub interface_type: String,
    pub protocol: Option<String>,
    pub frequency: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApplicationFullView {
    #[serde(flatten)]
    pub application: Application,
    pub business_owner_name: Option<String>,
    pub technical_owner_name: Option<String>,
    pub data_elements_count: i64,
    pub interfaces_count: i64,
    pub linked_processes: Vec<String>,
}
