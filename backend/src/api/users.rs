use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::db::AppState;
use crate::domain::users::*;
use crate::error::AppResult;

#[utoipa::path(
    get,
    path = "/api/v1/users",
    responses(
        (status = 200, description = "List users", body = Vec<User>)
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn list_users(State(_state): State<AppState>) -> AppResult<Json<Vec<User>>> {
    Ok(Json(vec![]))
}

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
    State(_state): State<AppState>,
    Path(_user_id): Path<Uuid>,
) -> AppResult<Json<UserWithRoles>> {
    Err(crate::error::AppError::NotFound("User not found".into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/roles",
    responses(
        (status = 200, description = "List roles", body = Vec<Role>)
    ),
    security(("bearer_auth" = [])),
    tag = "users"
)]
pub async fn list_roles(State(_state): State<AppState>) -> AppResult<Json<Vec<Role>>> {
    Ok(Json(vec![]))
}
