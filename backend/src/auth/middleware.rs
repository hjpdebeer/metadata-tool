use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::Claims;
use crate::db::AppState;
use crate::error::{AppError, AppResult};

/// Middleware that validates JWT bearer tokens and injects [`Claims`] into request extensions.
///
/// Extracts the `Authorization: Bearer <token>` header, validates the JWT against
/// the configured `JWT_SECRET`, and inserts the decoded [`Claims`] into request
/// extensions for downstream handlers to access.
///
/// # Errors
///
/// - `401 Unauthorized` if the Authorization header is missing, malformed, or the token is invalid/expired.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing Authorization header".into()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("invalid Authorization header format".into()))?;

    let claims = crate::auth::validate_token(token, &state.config.jwt_secret)
        .map_err(|e| AppError::Unauthorized(format!("invalid token: {e}")))?;

    // Inject claims into request extensions for downstream handlers
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Create a role-checking middleware layer for the given required roles.
///
/// Returns a middleware that checks if the authenticated user (from [`Claims`] in
/// request extensions) has **at least one** of the specified roles. If not, returns
/// `403 Forbidden`.
///
/// Must be applied **after** [`require_auth`] in the middleware stack.
///
/// # Usage
///
/// ```ignore
/// use axum::middleware;
/// router.layer(middleware::from_fn_with_state(state.clone(), require_auth))
///       .layer(middleware::from_fn(|req, next| require_roles(req, next, &["ADMIN", "DATA_STEWARD"])))
/// ```
pub async fn require_roles(
    request: Request,
    next: Next,
    required_roles: &[&str],
) -> AppResult<Response> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| {
            AppError::Unauthorized("authentication required before role check".into())
        })?;

    let has_role = claims
        .roles
        .iter()
        .any(|r| required_roles.contains(&r.as_str()));

    if !has_role {
        return Err(AppError::Forbidden(format!(
            "requires one of: {}",
            required_roles.join(", ")
        )));
    }

    Ok(next.run(request).await)
}
