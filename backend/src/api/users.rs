use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::glossary::PaginatedResponse;
use crate::domain::users::*;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Admin role guard helper
// ---------------------------------------------------------------------------

fn require_admin(claims: &Claims) -> AppResult<()> {
    if !claims.roles.iter().any(|r| r == "ADMIN") {
        return Err(AppError::Forbidden(
            "admin role required for this operation".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// list_users — GET /api/v1/users
// ---------------------------------------------------------------------------

/// List users with optional filtering and pagination.
/// Requires ADMIN role.
#[utoipa::path(
    get,
    path = "/api/v1/users",
    params(SearchUsersParams),
    responses(
        (status = 200, description = "Paginated list of users", body = PaginatedUsers)
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn list_users(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<SearchUsersParams>,
) -> AppResult<Json<PaginatedResponse<UserListItem>>> {
    require_admin(&claims)?;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(DISTINCT u.user_id)
        FROM users u
        LEFT JOIN user_roles ur ON ur.user_id = u.user_id
        LEFT JOIN roles r ON r.role_id = ur.role_id
        WHERE ($1::TEXT IS NULL OR u.display_name ILIKE '%' || $1 || '%'
               OR u.email ILIKE '%' || $1 || '%')
          AND ($2::TEXT IS NULL OR r.role_code = $2)
          AND ($3::BOOLEAN IS NULL OR u.is_active = $3)
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.role_code.as_deref())
    .bind(params.is_active)
    .fetch_one(&state.pool)
    .await?;

    let items = sqlx::query_as::<_, UserListItem>(
        r#"
        SELECT DISTINCT
            u.user_id, u.username, u.email, u.display_name,
            u.department, u.job_title, u.is_active,
            u.last_login_at, u.created_at
        FROM users u
        LEFT JOIN user_roles ur ON ur.user_id = u.user_id
        LEFT JOIN roles r ON r.role_id = ur.role_id
        WHERE ($1::TEXT IS NULL OR u.display_name ILIKE '%' || $1 || '%'
               OR u.email ILIKE '%' || $1 || '%')
          AND ($2::TEXT IS NULL OR r.role_code = $2)
          AND ($3::BOOLEAN IS NULL OR u.is_active = $3)
        ORDER BY u.display_name ASC
        LIMIT $4
        OFFSET $5
        "#,
    )
    .bind(params.query.as_deref())
    .bind(params.role_code.as_deref())
    .bind(params.is_active)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(PaginatedResponse {
        data: items,
        total_count,
        page,
        page_size,
    }))
}

// ---------------------------------------------------------------------------
// get_user — GET /api/v1/users/{user_id}
// ---------------------------------------------------------------------------

/// Retrieve a user's profile and assigned roles.
/// Requires ADMIN role.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user_id}",
    params(("user_id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User details with roles", body = UserWithRoles),
        (status = 404, description = "User not found")
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn get_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<UserWithRoles>> {
    require_admin(&claims)?;

    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT user_id, username, email, display_name, first_name, last_name,
               department, job_title, entra_object_id, is_active,
               last_login_at, created_at, updated_at
        FROM users
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("user not found: {user_id}")))?;

    let roles = sqlx::query_as::<_, Role>(
        r#"
        SELECT r.role_id, r.role_code, r.role_name, r.description, r.is_system_role
        FROM roles r
        JOIN user_roles ur ON ur.role_id = r.role_id
        WHERE ur.user_id = $1
        ORDER BY r.role_name ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(UserWithRoles::from_user_and_roles(user, roles)))
}

// ---------------------------------------------------------------------------
// update_user — PUT /api/v1/users/{user_id}
// ---------------------------------------------------------------------------

/// Update a user's profile. Only provided fields are changed.
/// Requires ADMIN role.
#[utoipa::path(
    put,
    path = "/api/v1/users/{user_id}",
    params(("user_id" = Uuid, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = User),
        (status = 404, description = "User not found")
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn update_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> AppResult<Json<User>> {
    require_admin(&claims)?;

    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET display_name = COALESCE($1, display_name),
            department   = COALESCE($2, department),
            job_title    = COALESCE($3, job_title),
            is_active    = COALESCE($4, is_active),
            updated_at   = CURRENT_TIMESTAMP
        WHERE user_id = $5
        RETURNING user_id, username, email, display_name, first_name, last_name,
                  department, job_title, entra_object_id, is_active,
                  last_login_at, created_at, updated_at
        "#,
    )
    .bind(body.display_name.as_deref())
    .bind(body.department.as_deref())
    .bind(body.job_title.as_deref())
    .bind(body.is_active)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("user not found: {user_id}")))?;

    Ok(Json(user))
}

// ---------------------------------------------------------------------------
// assign_role — POST /api/v1/users/{user_id}/roles
// ---------------------------------------------------------------------------

/// Assign an RBAC role to a user.
/// Requires ADMIN role.
#[utoipa::path(
    post,
    path = "/api/v1/users/{user_id}/roles",
    params(("user_id" = Uuid, Path, description = "User ID")),
    request_body = AssignRoleRequest,
    responses(
        (status = 201, description = "Role assigned"),
        (status = 404, description = "User or role not found"),
        (status = 409, description = "Role already assigned")
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn assign_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AssignRoleRequest>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;

    // Verify user exists
    let user_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    if !user_exists {
        return Err(AppError::NotFound(format!("user not found: {user_id}")));
    }

    // Verify role exists
    let role_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM roles WHERE role_id = $1)",
    )
    .bind(body.role_id)
    .fetch_one(&state.pool)
    .await?;

    if !role_exists {
        return Err(AppError::NotFound(format!(
            "role not found: {}",
            body.role_id
        )));
    }

    // Check if already assigned
    let already_assigned = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_roles WHERE user_id = $1 AND role_id = $2)",
    )
    .bind(user_id)
    .bind(body.role_id)
    .fetch_one(&state.pool)
    .await?;

    if already_assigned {
        return Err(AppError::Conflict(
            "role is already assigned to this user".into(),
        ));
    }

    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)",
    )
    .bind(user_id)
    .bind(body.role_id)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::CREATED)
}

// ---------------------------------------------------------------------------
// remove_role — DELETE /api/v1/users/{user_id}/roles/{role_id}
// ---------------------------------------------------------------------------

/// Remove a role assignment from a user.
/// Requires ADMIN role.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{user_id}/roles/{role_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ("role_id" = Uuid, Path, description = "Role ID")
    ),
    responses(
        (status = 204, description = "Role removed"),
        (status = 404, description = "User-role assignment not found")
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn remove_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((user_id, role_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;

    let rows_affected = sqlx::query(
        "DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2",
    )
    .bind(user_id)
    .bind(role_id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(
            "user-role assignment not found".into(),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// list_roles — GET /api/v1/roles
// ---------------------------------------------------------------------------

/// List all available roles.
/// Requires authentication (no admin check).
#[utoipa::path(
    get,
    path = "/api/v1/roles",
    responses(
        (status = 200, description = "List roles", body = Vec<Role>)
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn list_roles(State(state): State<AppState>) -> AppResult<Json<Vec<Role>>> {
    let roles = sqlx::query_as::<_, Role>(
        r#"
        SELECT role_id, role_code, role_name, description, is_system_role
        FROM roles
        ORDER BY role_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(roles))
}
