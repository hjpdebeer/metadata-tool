use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Full data element (single-record detail view)
// ---------------------------------------------------------------------------

/// A data element representing a business-level data concept. Maps to the `data_elements` table.
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

// ---------------------------------------------------------------------------
// List view (joined fields for display in tables/lists)
// ---------------------------------------------------------------------------

/// List view of a data element with joined fields for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct DataElementListItem {
    pub element_id: Uuid,
    pub element_name: String,
    pub element_code: String,
    pub description: String,
    pub data_type: String,
    pub is_cde: bool,
    pub domain_name: Option<String>,
    pub classification_name: Option<String>,
    pub status_code: String,
    pub status_name: String,
    pub owner_name: Option<String>,
    pub glossary_term_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Paginated response
// ---------------------------------------------------------------------------

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedDataElements {
    pub data: Vec<DataElementListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Full view (detail with related counts)
// ---------------------------------------------------------------------------

/// Response combining business and technical metadata for a data element
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DataElementFullView {
    #[serde(flatten)]
    pub element: DataElement,
    pub glossary_term_name: Option<String>,
    pub technical_columns: Vec<TechnicalColumn>,
    pub quality_rules_count: i64,
    pub linked_processes_count: i64,
    pub linked_applications_count: i64,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for creating a new data element.
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

/// Request body for partially updating a data element. All fields are optional.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDataElementRequest {
    pub element_name: Option<String>,
    pub element_code: Option<String>,
    pub description: Option<String>,
    pub business_definition: Option<String>,
    pub business_rules: Option<String>,
    pub data_type: Option<String>,
    pub format_pattern: Option<String>,
    pub allowed_values: Option<serde_json::Value>,
    pub default_value: Option<String>,
    pub is_nullable: Option<bool>,
    pub glossary_term_id: Option<Uuid>,
    pub domain_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,
    pub sensitivity_level: Option<String>,
}

/// Query parameters for searching and filtering data elements with pagination.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SearchDataElementsRequest {
    pub query: Option<String>,
    pub domain_id: Option<Uuid>,
    pub is_cde: Option<bool>,
    pub status: Option<String>,
    pub glossary_term_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Request body for designating or removing Critical Data Element (CDE) status (Principle 12).
#[derive(Debug, Deserialize, ToSchema)]
pub struct CdeDesignationRequest {
    pub is_cde: bool,
    pub cde_rationale: Option<String>,
}

// ---------------------------------------------------------------------------
// Source systems
// ---------------------------------------------------------------------------

/// A registered source system that produces or consumes data. Maps to the `source_systems` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SourceSystem {
    pub system_id: Uuid,
    pub system_name: String,
    pub system_code: String,
    pub system_type: String,
    pub description: Option<String>,
    pub connection_details: Option<serde_json::Value>,
}

/// Request body for registering a new source system.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSourceSystemRequest {
    pub system_name: String,
    pub system_code: String,
    pub system_type: String,
    pub description: Option<String>,
    pub connection_details: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Data classifications
// ---------------------------------------------------------------------------

/// Data classification level (e.g. Public, Internal, Confidential). Maps to the `data_classifications` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DataClassification {
    pub classification_id: Uuid,
    pub classification_code: String,
    pub classification_name: String,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Technical metadata
// ---------------------------------------------------------------------------

/// A database schema within a source system. Maps to the `technical_schemas` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TechnicalSchema {
    pub schema_id: Uuid,
    pub system_id: Uuid,
    pub schema_name: String,
    pub description: Option<String>,
}

/// Request body for creating a new technical schema under a source system.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTechnicalSchemaRequest {
    pub schema_name: String,
    pub description: Option<String>,
}

/// A database table or view within a technical schema. Maps to the `technical_tables` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TechnicalTable {
    pub table_id: Uuid,
    pub schema_id: Uuid,
    pub table_name: String,
    pub table_type: String,
    pub description: Option<String>,
    pub row_count: Option<i64>,
    pub size_bytes: Option<i64>,
}

/// Request body for creating a new technical table under a schema.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTechnicalTableRequest {
    pub table_name: String,
    pub table_type: Option<String>,
    pub description: Option<String>,
    pub row_count: Option<i64>,
    pub size_bytes: Option<i64>,
}

/// A column within a technical table, including naming standard compliance (Principle 8). Maps to the `technical_columns` table.
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

/// Request body for creating a new column under a technical table.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTechnicalColumnRequest {
    pub column_name: String,
    pub ordinal_position: i32,
    pub data_type: String,
    pub max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub is_nullable: Option<bool>,
    pub is_primary_key: Option<bool>,
    pub is_foreign_key: Option<bool>,
    pub element_id: Option<Uuid>,
}

/// Response for column creation including naming validation results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTechnicalColumnResponse {
    pub column: TechnicalColumn,
    pub naming_validation: NamingValidationInfo,
}

/// Naming validation information returned with column creation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NamingValidationInfo {
    pub is_compliant: bool,
    pub violations: Vec<NamingViolationInfo>,
}

/// Individual naming violation detail
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NamingViolationInfo {
    pub standard_name: String,
    pub message: String,
}
