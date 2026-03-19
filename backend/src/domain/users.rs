use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// A platform user with authentication and profile information. Maps to the `users` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub department: Option<String>,
    pub job_title: Option<String>,
    pub entra_object_id: Option<String>,
    pub is_active: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// List view of a user with role names joined for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct UserListItem {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub department: Option<String>,
    pub job_title: Option<String>,
    pub is_active: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// An RBAC role that can be assigned to users (Principle 10). Maps to the `roles` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Role {
    pub role_id: Uuid,
    pub role_code: String,
    pub role_name: String,
    pub description: Option<String>,
    pub is_system_role: bool,
}

/// A user combined with their assigned roles for detail views.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserWithRoles {
    #[serde(flatten)]
    pub user: User,
    pub roles: Vec<Role>,
}

/// Request body for creating a new user with initial role assignments.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub department: Option<String>,
    pub job_title: Option<String>,
    pub role_ids: Vec<Uuid>,
}

/// Request body for partially updating a user's profile. All fields are optional.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub department: Option<String>,
    pub job_title: Option<String>,
    pub is_active: Option<bool>,
}

/// Query parameters for searching and filtering users with pagination.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct SearchUsersParams {
    pub query: Option<String>,
    pub role_code: Option<String>,
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Request body for assigning a role to a user.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AssignRoleRequest {
    pub role_id: Uuid,
}

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedUsers {
    pub data: Vec<UserListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}
