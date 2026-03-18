use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::db::AppState;
use crate::error::{AppError, AppResult};

/// Middleware that validates JWT bearer tokens and injects Claims into request extensions.
pub async fn require_auth(
    State(_state): State<AppState>,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid Authorization header format".into()))?;

    // TODO: Validate JWT, extract claims, check expiry
    let _token = token;

    // TODO: Insert claims into request extensions
    // request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Middleware that checks if the authenticated user has any of the required roles.
pub async fn require_role(
    _required_roles: &[&str],
    request: Request,
    next: Next,
) -> AppResult<Response> {
    // TODO: Extract claims from extensions, check roles
    let _ = &request;
    Ok(next.run(request).await)
}
