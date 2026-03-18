use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BusinessProcess {
    pub process_id: Uuid,
    pub process_name: String,
    pub process_code: String,
    pub description: String,
    pub detailed_description: Option<String>,
    pub category_id: Option<Uuid>,
    pub status_id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub parent_process_id: Option<Uuid>,
    pub is_critical: bool,
    pub criticality_rationale: Option<String>,
    pub frequency: Option<String>,
    pub regulatory_requirement: Option<String>,
    pub sla_description: Option<String>,
    pub documentation_url: Option<String>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBusinessProcessRequest {
    pub process_name: String,
    pub process_code: String,
    pub description: String,
    pub detailed_description: Option<String>,
    pub category_id: Option<Uuid>,
    pub parent_process_id: Option<Uuid>,
    pub is_critical: Option<bool>,
    pub criticality_rationale: Option<String>,
    pub frequency: Option<String>,
    pub regulatory_requirement: Option<String>,
    pub sla_description: Option<String>,
    pub documentation_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ProcessStep {
    pub step_id: Uuid,
    pub process_id: Uuid,
    pub step_number: i32,
    pub step_name: String,
    pub description: Option<String>,
    pub responsible_role: Option<String>,
    pub application_id: Option<Uuid>,
    pub input_data_elements: Option<serde_json::Value>,
    pub output_data_elements: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BusinessProcessFullView {
    #[serde(flatten)]
    pub process: BusinessProcess,
    pub owner_name: Option<String>,
    pub steps: Vec<ProcessStep>,
    pub data_elements_count: i64,
    pub linked_applications: Vec<String>,
    pub sub_processes: Vec<BusinessProcess>,
}
