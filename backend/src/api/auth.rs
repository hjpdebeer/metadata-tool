use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::db::AppState;
use crate::error::AppResult;

#[derive(Deserialize)]
pub struct AuthCallback {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
}

/// Initiate SSO login via Microsoft Entra ID
#[utoipa::path(
    get,
    path = "/api/v1/auth/login",
    responses(
        (status = 302, description = "Redirect to Entra ID login")
    ),
    tag = "auth"
)]
pub async fn login(State(_state): State<AppState>) -> Redirect {
    // TODO: Build Entra ID authorization URL with PKCE
    // For now, redirect to a placeholder
    Redirect::temporary("/api/v1/auth/callback?code=placeholder")
}

/// Handle SSO callback from Microsoft Entra ID
#[utoipa::path(
    get,
    path = "/api/v1/auth/callback",
    responses(
        (status = 200, description = "Authentication successful", body = TokenResponse)
    ),
    tag = "auth"
)]
pub async fn callback(
    State(_state): State<AppState>,
    Query(_params): Query<AuthCallback>,
) -> AppResult<Json<TokenResponse>> {
    // TODO: Exchange code for tokens, create/update user, issue JWT
    Ok(Json(TokenResponse {
        access_token: "placeholder".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 28800,
    }))
}

/// Get current authenticated user info
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current user info", body = MeResponse),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn me(State(_state): State<AppState>) -> AppResult<Json<MeResponse>> {
    // TODO: Extract user from JWT token in Authorization header
    Err(crate::error::AppError::Unauthorized(
        "Not implemented yet".to_string(),
    ))
}
