use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Full application (single-record detail view)
// ---------------------------------------------------------------------------

/// A business application in the application registry. Maps to the `applications` table.
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
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedApplications {
    pub data: Vec<ApplicationListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Full view (detail with related counts)
// ---------------------------------------------------------------------------

/// Detail view of an application including owner names, linked elements, and process counts.
#[derive(Debug, Clone, Serialize, ToSchema)]
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

/// Request body for creating a new application.
#[derive(Debug, Clone, Deserialize, ToSchema)]
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

/// Request body for partially updating an application. All fields are optional.
#[derive(Debug, Clone, Deserialize, ToSchema)]
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

/// Query parameters for searching and filtering applications with pagination.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
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

/// Classification category for applications (e.g. Core, Support, Analytics). Maps to the `application_classifications` table.
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

/// Request body for linking a data element to an application.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LinkDataElementRequest {
    pub element_id: Uuid,
    pub usage_type: Option<String>,
    pub is_authoritative_source: Option<bool>,
    pub description: Option<String>,
}

/// A link between an application and a data element, with usage details. Maps to the `application_data_elements` table.
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

/// An interface between two applications describing data exchange. Maps to the `application_interfaces` table.
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
