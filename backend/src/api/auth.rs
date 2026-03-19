use axum::extract::{Query, Request, State};
use axum::response::Redirect;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::Claims;
use crate::db::AppState;
use crate::error::{AppError, AppResult};

/// Query parameters received from the Entra ID OAuth callback.
#[derive(Deserialize)]
pub struct AuthCallback {
    pub code: String,
    pub state: Option<String>,
}

/// Request body for dev-mode login with email and password.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DevLoginRequest {
    pub email: String,
    pub password: String,
}

/// JWT token response returned on successful authentication.
#[derive(Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Response containing the authenticated user's identity and roles.
#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
}

/// Dev-mode login with email and password.
///
/// Only available when `ENTRA_TENANT_ID` is not configured (dev mode).
/// Validates credentials against the users table using bcrypt (pgcrypto).
/// Returns a JWT token on success.
#[utoipa::path(
    post,
    path = "/api/v1/auth/dev-login",
    request_body = DevLoginRequest,
    responses(
        (status = 200, description = "Login successful", body = TokenResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Dev login disabled (Entra SSO configured)")
    ),
    tag = "auth"
)]
pub async fn dev_login(
    State(state): State<AppState>,
    Json(body): Json<DevLoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    // Block dev-login when Entra SSO is configured
    if state.config.entra.tenant_id != "your-tenant-id" && !state.config.entra.tenant_id.is_empty()
    {
        return Err(AppError::Forbidden(
            "dev login disabled — Entra SSO is configured".into(),
        ));
    }

    // Look up user by email with password hash, using pgcrypto's crypt() for bcrypt verification
    let row = sqlx::query_as::<_, UserAuthRow>(
        r#"
        SELECT u.user_id, u.email, u.display_name, u.password_hash, u.is_active,
               COALESCE(
                   array_agg(r.role_code) FILTER (WHERE r.role_code IS NOT NULL),
                   ARRAY[]::VARCHAR[]
               ) as roles
        FROM users u
        LEFT JOIN user_roles ur ON u.user_id = ur.user_id
            AND (ur.effective_to IS NULL OR ur.effective_to > NOW())
        LEFT JOIN roles r ON ur.role_id = r.role_id
        WHERE u.email = $1 AND u.deleted_at IS NULL
        GROUP BY u.user_id, u.email, u.display_name, u.password_hash, u.is_active
        "#,
    )
    .bind(&body.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?
    .ok_or_else(|| AppError::Unauthorized("invalid email or password".into()))?;

    if !row.is_active {
        return Err(AppError::Unauthorized("account is disabled".into()));
    }

    let password_hash = row
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::Unauthorized("no password set — use SSO login".into()))?;

    // Verify password using pgcrypto's crypt() via a DB query
    let valid: bool = sqlx::query_scalar("SELECT crypt($1, $2) = $2")
        .bind(&body.password)
        .bind(password_hash)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    if !valid {
        return Err(AppError::Unauthorized("invalid email or password".into()));
    }

    // Issue JWT
    let token = crate::auth::create_token(
        row.user_id,
        &row.email,
        &row.display_name,
        row.roles,
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("token creation failed: {e}")))?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: state.config.jwt_expiry_hours * 3600,
    }))
}

/// Initiate SSO login via Microsoft Entra ID.
///
/// Redirects to the Entra ID authorization endpoint. Not yet implemented —
/// returns a placeholder redirect. Will be implemented in Sprint 13.
#[utoipa::path(
    get,
    path = "/api/v1/auth/login",
    responses(
        (status = 302, description = "Redirect to Entra ID login")
    ),
    tag = "auth"
)]
pub async fn login(State(_state): State<AppState>) -> Redirect {
    // TODO (Sprint 13): Build Entra ID authorization URL with PKCE
    Redirect::temporary("/api/v1/auth/callback?code=placeholder")
}

/// Handle SSO callback from Microsoft Entra ID.
///
/// Exchanges the authorization code for tokens, creates/updates the user,
/// and issues a JWT. Not yet implemented — will be implemented in Sprint 13.
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
    // TODO (Sprint 13): Exchange code for tokens, create/update user, issue JWT
    Err(AppError::Internal(anyhow::anyhow!(
        "Entra SSO not yet implemented — use /api/v1/auth/dev-login"
    )))
}

/// Get current authenticated user info.
///
/// Returns the user ID, email, display name, and roles from the JWT token.
/// Requires a valid Bearer token in the Authorization header.
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
pub async fn me(request: Request) -> AppResult<Json<MeResponse>> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AppError::Unauthorized("not authenticated".into()))?
        .clone();

    Ok(Json(MeResponse {
        user_id: claims.sub,
        email: claims.email,
        display_name: claims.display_name,
        roles: claims.roles,
    }))
}

/// Internal row type for user authentication query.
#[derive(sqlx::FromRow)]
struct UserAuthRow {
    user_id: uuid::Uuid,
    email: String,
    display_name: String,
    password_hash: Option<String>,
    is_active: bool,
    roles: Vec<String>,
}
