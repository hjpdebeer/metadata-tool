use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Full application (single-record row from DB)
// ---------------------------------------------------------------------------

/// A business application in the application registry. Maps to the `applications` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Application {
    pub application_id: Uuid,
    pub application_name: String,
    pub application_code: String,
    pub description: String,
    // Classification & type
    pub classification_id: Option<Uuid>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    // Ownership & governance
    pub status_id: Uuid,
    pub business_owner_id: Option<Uuid>,
    pub technical_owner_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
    pub organisational_unit: Option<String>,
    // Vendor & product
    pub vendor: Option<String>,
    pub vendor_product_name: Option<String>,
    pub version: Option<String>,
    pub license_type: Option<String>,
    // Business context
    pub abbreviation: Option<String>,
    pub external_reference_id: Option<String>,
    pub business_capability: Option<String>,
    pub user_base: Option<String>,
    // Criticality & risk
    pub is_cba: bool,
    pub cba_rationale: Option<String>,
    pub criticality_tier_id: Option<Uuid>,
    pub risk_rating_id: Option<Uuid>,
    // Compliance
    pub data_classification_id: Option<Uuid>,
    pub regulatory_scope: Option<String>,
    pub last_security_assessment: Option<NaiveDate>,
    // Operational
    pub support_model: Option<String>,
    pub dr_tier_id: Option<Uuid>,
    // Lifecycle
    pub lifecycle_stage_id: Option<Uuid>,
    pub go_live_date: Option<DateTime<Utc>>,
    pub retirement_date: Option<DateTime<Utc>>,
    pub contract_end_date: Option<NaiveDate>,
    pub review_frequency_id: Option<Uuid>,
    pub next_review_date: Option<NaiveDate>,
    pub approved_at: Option<DateTime<Utc>>,
    // Reference
    pub documentation_url: Option<String>,
    // Versioning
    pub version_number: i32,
    pub is_current_version: bool,
    pub previous_version_id: Option<Uuid>,
    // Audit
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
    pub abbreviation: Option<String>,
    pub classification_name: Option<String>,
    pub status_code: String,
    pub status_name: String,
    pub business_owner_name: Option<String>,
    pub technical_owner_name: Option<String>,
    pub vendor: Option<String>,
    pub is_cba: bool,
    pub deployment_type: Option<String>,
    pub lifecycle_stage_name: Option<String>,
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

/// Internal row type for the single JOIN query that fetches all application columns
/// plus resolved FK lookup names. Used by the `get_application` handler (ADR-0006 Pattern 1).
#[derive(Debug, Clone, FromRow)]
pub struct ApplicationDetailRow {
    // === Entity columns ===
    pub application_id: Uuid,
    pub application_name: String,
    pub application_code: String,
    pub description: String,
    pub classification_id: Option<Uuid>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    pub status_id: Uuid,
    pub business_owner_id: Option<Uuid>,
    pub technical_owner_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
    pub organisational_unit: Option<String>,
    pub vendor: Option<String>,
    pub vendor_product_name: Option<String>,
    pub version: Option<String>,
    pub license_type: Option<String>,
    pub abbreviation: Option<String>,
    pub external_reference_id: Option<String>,
    pub business_capability: Option<String>,
    pub user_base: Option<String>,
    pub is_cba: bool,
    pub cba_rationale: Option<String>,
    pub criticality_tier_id: Option<Uuid>,
    pub risk_rating_id: Option<Uuid>,
    pub data_classification_id: Option<Uuid>,
    pub regulatory_scope: Option<String>,
    pub last_security_assessment: Option<NaiveDate>,
    pub support_model: Option<String>,
    pub dr_tier_id: Option<Uuid>,
    pub lifecycle_stage_id: Option<Uuid>,
    pub go_live_date: Option<DateTime<Utc>>,
    pub retirement_date: Option<DateTime<Utc>>,
    pub contract_end_date: Option<NaiveDate>,
    pub review_frequency_id: Option<Uuid>,
    pub next_review_date: Option<NaiveDate>,
    pub approved_at: Option<DateTime<Utc>>,
    pub documentation_url: Option<String>,
    pub version_number: i32,
    pub is_current_version: bool,
    pub previous_version_id: Option<Uuid>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // === Resolved lookup names (from LEFT JOINs) ===
    pub classification_name: Option<String>,
    pub status_code: Option<String>,
    pub status_name: Option<String>,
    pub business_owner_name: Option<String>,
    pub technical_owner_name: Option<String>,
    pub steward_name: Option<String>,
    pub approver_name: Option<String>,
    pub criticality_tier_name: Option<String>,
    pub risk_rating_name: Option<String>,
    pub data_classification_name: Option<String>,
    pub dr_tier_name: Option<String>,
    pub dr_tier_rto_hours: Option<i32>,
    pub dr_tier_rpo_minutes: Option<i32>,
    pub lifecycle_stage_name: Option<String>,
    pub review_frequency_name: Option<String>,
    pub created_by_name: Option<String>,
    pub updated_by_name: Option<String>,
}

/// Complete application detail view with resolved lookup names and junction data.
/// All fields are at the root level -- no nesting, no `#[serde(flatten)]` (ADR-0006 Pattern 1).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationFullView {
    // === Entity columns ===
    pub application_id: Uuid,
    pub application_name: String,
    pub application_code: String,
    pub description: String,
    pub classification_id: Option<Uuid>,
    pub classification_name: Option<String>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    pub status_id: Uuid,
    pub status_code: Option<String>,
    pub business_owner_id: Option<Uuid>,
    pub business_owner_name: Option<String>,
    pub technical_owner_id: Option<Uuid>,
    pub technical_owner_name: Option<String>,
    pub steward_user_id: Option<Uuid>,
    pub steward_name: Option<String>,
    pub approver_user_id: Option<Uuid>,
    pub approver_name: Option<String>,
    pub organisational_unit: Option<String>,
    pub vendor: Option<String>,
    pub vendor_product_name: Option<String>,
    pub version: Option<String>,
    pub license_type: Option<String>,
    pub abbreviation: Option<String>,
    pub external_reference_id: Option<String>,
    pub business_capability: Option<String>,
    pub user_base: Option<String>,
    pub is_cba: bool,
    pub cba_rationale: Option<String>,
    pub criticality_tier_id: Option<Uuid>,
    pub criticality_tier_name: Option<String>,
    pub risk_rating_id: Option<Uuid>,
    pub risk_rating_name: Option<String>,
    pub data_classification_id: Option<Uuid>,
    pub data_classification_name: Option<String>,
    pub regulatory_scope: Option<String>,
    pub last_security_assessment: Option<NaiveDate>,
    pub support_model: Option<String>,
    pub dr_tier_id: Option<Uuid>,
    pub dr_tier_name: Option<String>,
    pub dr_tier_rto_hours: Option<i32>,
    pub dr_tier_rpo_minutes: Option<i32>,
    pub lifecycle_stage_id: Option<Uuid>,
    pub lifecycle_stage_name: Option<String>,
    pub go_live_date: Option<DateTime<Utc>>,
    pub retirement_date: Option<DateTime<Utc>>,
    pub contract_end_date: Option<NaiveDate>,
    pub review_frequency_id: Option<Uuid>,
    pub review_frequency_name: Option<String>,
    pub next_review_date: Option<NaiveDate>,
    pub approved_at: Option<DateTime<Utc>>,
    pub documentation_url: Option<String>,
    pub version_number: i32,
    pub is_current_version: bool,
    pub previous_version_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_by_name: Option<String>,
    pub updated_by: Option<Uuid>,
    pub updated_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // === Junction data (from separate queries) ===
    pub data_elements_count: i64,
    pub interfaces_count: i64,
    pub linked_processes: Vec<String>,
}

impl ApplicationFullView {
    /// Construct from an `ApplicationDetailRow` (JOIN query result) and junction data.
    pub fn from_row_and_junctions(
        row: ApplicationDetailRow,
        data_elements_count: i64,
        interfaces_count: i64,
        linked_processes: Vec<String>,
    ) -> Self {
        Self {
            application_id: row.application_id,
            application_name: row.application_name,
            application_code: row.application_code,
            description: row.description,
            classification_id: row.classification_id,
            classification_name: row.classification_name,
            deployment_type: row.deployment_type,
            technology_stack: row.technology_stack,
            status_id: row.status_id,
            status_code: row.status_code,
            business_owner_id: row.business_owner_id,
            business_owner_name: row.business_owner_name,
            technical_owner_id: row.technical_owner_id,
            technical_owner_name: row.technical_owner_name,
            steward_user_id: row.steward_user_id,
            steward_name: row.steward_name,
            approver_user_id: row.approver_user_id,
            approver_name: row.approver_name,
            organisational_unit: row.organisational_unit,
            vendor: row.vendor,
            vendor_product_name: row.vendor_product_name,
            version: row.version,
            license_type: row.license_type,
            abbreviation: row.abbreviation,
            external_reference_id: row.external_reference_id,
            business_capability: row.business_capability,
            user_base: row.user_base,
            is_cba: row.is_cba,
            cba_rationale: row.cba_rationale,
            criticality_tier_id: row.criticality_tier_id,
            criticality_tier_name: row.criticality_tier_name,
            risk_rating_id: row.risk_rating_id,
            risk_rating_name: row.risk_rating_name,
            data_classification_id: row.data_classification_id,
            data_classification_name: row.data_classification_name,
            regulatory_scope: row.regulatory_scope,
            last_security_assessment: row.last_security_assessment,
            support_model: row.support_model,
            dr_tier_id: row.dr_tier_id,
            dr_tier_name: row.dr_tier_name,
            dr_tier_rto_hours: row.dr_tier_rto_hours,
            dr_tier_rpo_minutes: row.dr_tier_rpo_minutes,
            lifecycle_stage_id: row.lifecycle_stage_id,
            lifecycle_stage_name: row.lifecycle_stage_name,
            go_live_date: row.go_live_date,
            retirement_date: row.retirement_date,
            contract_end_date: row.contract_end_date,
            review_frequency_id: row.review_frequency_id,
            review_frequency_name: row.review_frequency_name,
            next_review_date: row.next_review_date,
            approved_at: row.approved_at,
            documentation_url: row.documentation_url,
            version_number: row.version_number,
            is_current_version: row.is_current_version,
            previous_version_id: row.previous_version_id,
            created_by: row.created_by,
            created_by_name: row.created_by_name,
            updated_by: row.updated_by,
            updated_by_name: row.updated_by_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
            data_elements_count,
            interfaces_count,
            linked_processes,
        }
    }
}

// ---------------------------------------------------------------------------
// Lookup types
// ---------------------------------------------------------------------------

/// Classification category for applications (e.g. Core Banking, Payments, Risk).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApplicationClassification {
    pub classification_id: Uuid,
    pub classification_code: String,
    pub classification_name: String,
    pub description: Option<String>,
}

/// Disaster recovery tier with RTO/RPO definitions.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DisasterRecoveryTier {
    pub dr_tier_id: Uuid,
    pub tier_code: String,
    pub tier_name: String,
    pub rto_hours: i32,
    pub rpo_minutes: i32,
    pub description: Option<String>,
}

/// Application lifecycle stage (Planning, Development, Active, Sunset, Retired).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApplicationLifecycleStage {
    pub stage_id: Uuid,
    pub stage_code: String,
    pub stage_name: String,
    pub description: Option<String>,
}

/// Application criticality tier (Tier 1 through Tier 4).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApplicationCriticalityTier {
    pub tier_id: Uuid,
    pub tier_code: String,
    pub tier_name: String,
    pub description: Option<String>,
}

/// Application risk rating (Critical, High, Medium, Low).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApplicationRiskRating {
    pub rating_id: Uuid,
    pub rating_code: String,
    pub rating_name: String,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for creating a new application.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateApplicationRequest {
    pub application_name: String,
    pub description: String,
    pub classification_id: Option<Uuid>,
    pub vendor: Option<String>,
    pub vendor_product_name: Option<String>,
    pub version: Option<String>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    pub is_cba: Option<bool>,
    pub cba_rationale: Option<String>,
    pub go_live_date: Option<DateTime<Utc>>,
    pub documentation_url: Option<String>,
    pub abbreviation: Option<String>,
    pub external_reference_id: Option<String>,
    pub license_type: Option<String>,
    pub lifecycle_stage_id: Option<Uuid>,
    pub business_owner_id: Option<Uuid>,
    pub technical_owner_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
}

/// Request body for partially updating an application. All fields are optional.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateApplicationRequest {
    pub application_name: Option<String>,
    pub description: Option<String>,
    pub classification_id: Option<Uuid>,
    pub vendor: Option<String>,
    pub vendor_product_name: Option<String>,
    pub version: Option<String>,
    pub deployment_type: Option<String>,
    pub technology_stack: Option<serde_json::Value>,
    pub is_cba: Option<bool>,
    pub cba_rationale: Option<String>,
    pub go_live_date: Option<DateTime<Utc>>,
    pub retirement_date: Option<DateTime<Utc>>,
    pub documentation_url: Option<String>,
    pub abbreviation: Option<String>,
    pub external_reference_id: Option<String>,
    pub business_capability: Option<String>,
    pub user_base: Option<String>,
    pub license_type: Option<String>,
    pub lifecycle_stage_id: Option<Uuid>,
    pub criticality_tier_id: Option<Uuid>,
    pub risk_rating_id: Option<Uuid>,
    pub data_classification_id: Option<Uuid>,
    pub regulatory_scope: Option<String>,
    pub last_security_assessment: Option<NaiveDate>,
    pub support_model: Option<String>,
    pub dr_tier_id: Option<Uuid>,
    pub contract_end_date: Option<NaiveDate>,
    pub review_frequency_id: Option<Uuid>,
    // Ownership
    pub business_owner_id: Option<Uuid>,
    pub technical_owner_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
    pub organisational_unit: Option<String>,
}

/// Query parameters for searching and filtering applications with pagination.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct SearchApplicationsRequest {
    pub query: Option<String>,
    pub classification_id: Option<Uuid>,
    pub status: Option<String>,
    pub is_cba: Option<bool>,
    pub deployment_type: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
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

/// A link between an application and a data element, with usage details.
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

/// An interface between two applications describing data exchange.
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
