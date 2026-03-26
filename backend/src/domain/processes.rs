use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Full business process (single-record detail view)
// ---------------------------------------------------------------------------

/// A business process with optional criticality designation (Principle 12). Maps to the `business_processes` table.
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
    pub steward_user_id: Option<Uuid>,
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
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedBusinessProcesses {
    pub data: Vec<BusinessProcessListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Full view (detail with related counts and sub-entities)
// ---------------------------------------------------------------------------

/// Internal row type for the single JOIN query that fetches all process columns
/// plus resolved FK lookup names. Used by the `get_process` handler (ADR-0006 Pattern 1).
#[derive(Debug, Clone, FromRow)]
pub struct BusinessProcessDetailRow {
    // === Entity columns ===
    pub process_id: Uuid,
    pub process_name: String,
    pub process_code: String,
    pub description: String,
    pub detailed_description: Option<String>,
    pub category_id: Option<Uuid>,
    pub status_id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
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
    // === Resolved lookup names (from LEFT JOINs) ===
    pub owner_name: Option<String>,
    pub steward_name: Option<String>,
    pub category_name: Option<String>,
    pub parent_process_name: Option<String>,
    pub status_code: Option<String>,
    pub status_name: Option<String>,
    pub created_by_name: Option<String>,
    pub updated_by_name: Option<String>,
    pub workflow_instance_id: Option<Uuid>,
}

/// Complete business process detail view with resolved lookup names and junction data.
/// All fields are at the root level -- no nesting, no `#[serde(flatten)]` (ADR-0006 Pattern 1).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BusinessProcessFullView {
    // === Entity columns ===
    pub process_id: Uuid,
    pub process_name: String,
    pub process_code: String,
    pub description: String,
    pub detailed_description: Option<String>,
    pub category_id: Option<Uuid>,
    pub category_name: Option<String>,
    pub status_id: Uuid,
    pub status_code: Option<String>,
    pub owner_user_id: Option<Uuid>,
    pub owner_name: Option<String>,
    pub steward_user_id: Option<Uuid>,
    pub steward_name: Option<String>,
    pub parent_process_id: Option<Uuid>,
    pub parent_process_name: Option<String>,
    pub is_critical: bool,
    pub criticality_rationale: Option<String>,
    pub frequency: Option<String>,
    pub regulatory_requirement: Option<String>,
    pub sla_description: Option<String>,
    pub documentation_url: Option<String>,
    pub created_by: Uuid,
    pub created_by_name: Option<String>,
    pub updated_by: Option<Uuid>,
    pub updated_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub workflow_instance_id: Option<Uuid>,
    // === Junction data (from separate queries) ===
    pub steps: Vec<ProcessStep>,
    pub data_elements_count: i64,
    pub linked_applications: Vec<String>,
    pub sub_processes: Vec<BusinessProcess>,
}

impl BusinessProcessFullView {
    /// Construct from a `BusinessProcessDetailRow` (JOIN query result) and junction data.
    pub fn from_row_and_junctions(
        row: BusinessProcessDetailRow,
        steps: Vec<ProcessStep>,
        data_elements_count: i64,
        linked_applications: Vec<String>,
        sub_processes: Vec<BusinessProcess>,
    ) -> Self {
        Self {
            process_id: row.process_id,
            process_name: row.process_name,
            process_code: row.process_code,
            description: row.description,
            detailed_description: row.detailed_description,
            category_id: row.category_id,
            category_name: row.category_name,
            status_id: row.status_id,
            status_code: row.status_code,
            owner_user_id: row.owner_user_id,
            owner_name: row.owner_name,
            steward_user_id: row.steward_user_id,
            steward_name: row.steward_name,
            parent_process_id: row.parent_process_id,
            parent_process_name: row.parent_process_name,
            is_critical: row.is_critical,
            criticality_rationale: row.criticality_rationale,
            frequency: row.frequency,
            regulatory_requirement: row.regulatory_requirement,
            sla_description: row.sla_description,
            documentation_url: row.documentation_url,
            created_by: row.created_by,
            created_by_name: row.created_by_name,
            updated_by: row.updated_by,
            updated_by_name: row.updated_by_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
            workflow_instance_id: row.workflow_instance_id,
            steps,
            data_elements_count,
            linked_applications,
            sub_processes,
        }
    }
}

// ---------------------------------------------------------------------------
// Process steps
// ---------------------------------------------------------------------------

/// An ordered step within a business process. Maps to the `process_steps` table.
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

/// Request body for creating a new business process.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateBusinessProcessRequest {
    pub process_name: String,
    pub process_code: Option<String>,
    pub description: String,
    pub detailed_description: Option<String>,
    pub category_id: Option<Uuid>,
    pub parent_process_id: Option<Uuid>,
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub is_critical: Option<bool>,
    pub criticality_rationale: Option<String>,
    pub frequency: Option<String>,
    pub regulatory_requirement: Option<String>,
    pub sla_description: Option<String>,
    pub documentation_url: Option<String>,
}

/// Request body for partially updating a business process. All fields are optional.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateBusinessProcessRequest {
    pub process_name: Option<String>,
    pub description: Option<String>,
    pub detailed_description: Option<String>,
    pub category_id: Option<Uuid>,
    pub parent_process_id: Option<Uuid>,
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub is_critical: Option<bool>,
    pub criticality_rationale: Option<String>,
    pub frequency: Option<String>,
    pub regulatory_requirement: Option<String>,
    pub sla_description: Option<String>,
    pub documentation_url: Option<String>,
}

/// Query parameters for searching and filtering business processes with pagination.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
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

/// A hierarchical category for organising business processes. Maps to the `process_categories` table.
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

/// Request body for linking a data element to a business process.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LinkProcessDataElementRequest {
    pub element_id: Uuid,
    pub usage_type: Option<String>,
    pub is_required: Option<bool>,
    pub description: Option<String>,
}

/// A link between a process and a data element, with CDE indicator. Maps to the `process_data_elements` table.
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

/// Request body for linking an application to a business process.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LinkProcessApplicationRequest {
    pub application_id: Uuid,
    pub role_in_process: Option<String>,
    pub description: Option<String>,
}

/// A link between a process and an application, with role description. Maps to the `process_applications` table.
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

/// Request body for adding a step to a business process.
#[derive(Debug, Clone, Deserialize, ToSchema)]
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
