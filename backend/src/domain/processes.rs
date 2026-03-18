use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Full business process (single-record detail view)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// List view (joined fields for display in tables/lists)
// ---------------------------------------------------------------------------

/// List view of a business process with joined fields for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct BusinessProcessListItem {
    pub process_id: Uuid,
    pub process_name: String,
    pub process_code: String,
    pub description: String,
    pub category_name: Option<String>,
    pub status_code: String,
    pub status_name: String,
    pub owner_name: Option<String>,
    pub is_critical: bool,
    pub frequency: Option<String>,
    pub parent_process_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Paginated response
// ---------------------------------------------------------------------------

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedBusinessProcesses {
    pub data: Vec<BusinessProcessListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Full view (detail with related counts and sub-entities)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct BusinessProcessFullView {
    #[serde(flatten)]
    pub process: BusinessProcess,
    pub owner_name: Option<String>,
    pub category_name: Option<String>,
    pub parent_process_name: Option<String>,
    pub steps: Vec<ProcessStep>,
    pub data_elements_count: i64,
    pub linked_applications: Vec<String>,
    pub sub_processes: Vec<BusinessProcess>,
}

// ---------------------------------------------------------------------------
// Process steps
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateBusinessProcessRequest {
    pub process_name: Option<String>,
    pub description: Option<String>,
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

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SearchProcessesRequest {
    pub query: Option<String>,
    pub category_id: Option<Uuid>,
    pub status: Option<String>,
    pub is_critical: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ---------------------------------------------------------------------------
// Process category lookup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ProcessCategory {
    pub category_id: Uuid,
    pub category_name: String,
    pub description: Option<String>,
    pub parent_category_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Process-Data Element links
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkProcessDataElementRequest {
    pub element_id: Uuid,
    pub usage_type: Option<String>,
    pub is_required: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ProcessDataElementLink {
    pub id: Uuid,
    pub process_id: Uuid,
    pub element_id: Uuid,
    pub element_name: String,
    pub element_code: String,
    pub is_cde: bool,
    pub usage_type: String,
    pub is_required: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Process-Application links
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkProcessApplicationRequest {
    pub application_id: Uuid,
    pub role_in_process: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ProcessApplicationLink {
    pub id: Uuid,
    pub process_id: Uuid,
    pub application_id: Uuid,
    pub application_name: String,
    pub application_code: String,
    pub role_in_process: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Process step creation
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProcessStepRequest {
    pub step_number: i32,
    pub step_name: String,
    pub description: Option<String>,
    pub responsible_role: Option<String>,
    pub application_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Critical process summary (with element counts)
// ---------------------------------------------------------------------------

/// Critical process with data element count for the critical processes list
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct CriticalProcessSummary {
    pub process_id: Uuid,
    pub process_name: String,
    pub process_code: String,
    pub description: String,
    pub category_name: Option<String>,
    pub owner_name: Option<String>,
    pub frequency: Option<String>,
    pub data_elements_count: i64,
}
