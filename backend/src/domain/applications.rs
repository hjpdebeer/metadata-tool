use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Full application (single-record detail view)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// List view (joined fields for display in tables/lists)
// ---------------------------------------------------------------------------

/// List view of an application with joined fields for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ApplicationListItem {
    pub application_id: Uuid,
    pub application_name: String,
    pub application_code: String,
    pub description: String,
    pub classification_name: Option<String>,
    pub status_code: String,
    pub status_name: String,
    pub business_owner_name: Option<String>,
    pub technical_owner_name: Option<String>,
    pub vendor: Option<String>,
    pub is_critical: bool,
    pub deployment_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Paginated response
// ---------------------------------------------------------------------------

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedApplications {
    pub data: Vec<ApplicationListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Full view (detail with related counts)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

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

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SearchApplicationsRequest {
    pub query: Option<String>,
    pub classification_id: Option<Uuid>,
    pub status: Option<String>,
    pub is_critical: Option<bool>,
    pub deployment_type: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ---------------------------------------------------------------------------
// Classification lookup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApplicationClassification {
    pub classification_id: Uuid,
    pub classification_code: String,
    pub classification_name: String,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Application-Data Element links
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct LinkDataElementRequest {
    pub element_id: Uuid,
    pub usage_type: Option<String>,
    pub is_authoritative_source: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ApplicationDataElementLink {
    pub id: Uuid,
    pub application_id: Uuid,
    pub element_id: Uuid,
    pub element_name: String,
    pub element_code: String,
    pub usage_type: String,
    pub is_authoritative_source: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Application interfaces
// ---------------------------------------------------------------------------

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
