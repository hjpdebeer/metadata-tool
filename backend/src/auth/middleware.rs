use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::Claims;
use crate::db::AppState;
use crate::error::{AppError, AppResult};

/// Row type for API key lookup.
#[derive(sqlx::FromRow)]
struct ApiKeyLookupRow {
    key_id: uuid::Uuid,
    key_hash: String,
    scopes: Vec<String>,
    created_by: uuid::Uuid,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Row type for user info lookup (for API key auth).
#[derive(sqlx::FromRow)]
struct ApiKeyUserRow {
    email: String,
    display_name: String,
}

/// Middleware that validates authentication and injects [`Claims`] into request extensions.
///
/// Authentication methods (checked in order):
/// 1. `X-API-Key` header — validates against `api_keys` table using pgcrypto bcrypt.
///    Creates Claims using the key creator's user_id and the key's scopes as roles.
/// 2. `Authorization: Bearer <token>` header — validates JWT token.
///
/// # Errors
///
/// - `401 Unauthorized` if no valid authentication is provided.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    // -----------------------------------------------------------------
    // Strategy 1: X-API-Key header
    // -----------------------------------------------------------------
    if let Some(api_key) = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
    {
        let api_key = api_key.trim();
        if api_key.len() < 8 {
            return Err(AppError::Unauthorized("invalid API key format".into()));
        }

        let key_prefix = &api_key[..8];

        // Look up candidate keys by prefix
        let candidates = sqlx::query_as::<_, ApiKeyLookupRow>(
            r#"
            SELECT key_id, key_hash, scopes, created_by, expires_at
            FROM api_keys
            WHERE key_prefix = $1 AND is_active = TRUE
            "#,
        )
        .bind(key_prefix)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("API key lookup failed: {e}")))?;

        if candidates.is_empty() {
            return Err(AppError::Unauthorized("invalid API key".into()));
        }

        // Verify key hash using pgcrypto
        let mut matched_key: Option<&ApiKeyLookupRow> = None;
        for candidate in &candidates {
            let valid: bool = sqlx::query_scalar("SELECT crypt($1, $2) = $2")
                .bind(api_key)
                .bind(&candidate.key_hash)
                .fetch_one(&state.pool)
                .await
                .unwrap_or(false);

            if valid {
                matched_key = Some(candidate);
                break;
            }
        }

        let key = matched_key.ok_or_else(|| AppError::Unauthorized("invalid API key".into()))?;

        // Check expiry
        if let Some(expires_at) = key.expires_at
            && chrono::Utc::now() > expires_at
        {
            return Err(AppError::Unauthorized("API key has expired".into()));
        }

        // Update last_used_at (fire-and-forget — do not block the request)
        let pool = state.pool.clone();
        let key_id = key.key_id;
        tokio::spawn(async move {
            let _ = sqlx::query(
                "UPDATE api_keys SET last_used_at = CURRENT_TIMESTAMP WHERE key_id = $1",
            )
            .bind(key_id)
            .execute(&pool)
            .await;
        });

        // Look up the key creator's identity
        let user = sqlx::query_as::<_, ApiKeyUserRow>(
            "SELECT email, display_name FROM users WHERE user_id = $1",
        )
        .bind(key.created_by)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("user lookup failed: {e}")))?
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "API key creator user not found: {}",
                key.created_by
            ))
        })?;

        // Build Claims from API key data
        let now = chrono::Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: key.created_by,
            email: user.email,
            display_name: format!("{} [API Key]", user.display_name),
            roles: key.scopes.clone(),
            exp: now + 3600, // arbitrary — not used for API key auth
            iat: now,
        };

        request.extensions_mut().insert(claims);
        return Ok(next.run(request).await);
    }

    // -----------------------------------------------------------------
    // Strategy 2: Authorization: Bearer <JWT>
    // -----------------------------------------------------------------
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
    let claims = request.extensions().get::<Claims>().ok_or_else(|| {
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

/// Check that the authenticated user (from [`Claims`]) has a specific scope.
///
/// Used on ingestion endpoints to validate that API key scopes include
/// the required permission. For JWT-authenticated users, this checks the
/// roles array instead (ADMIN role has implicit access to all scopes).
///
/// Call at the beginning of a handler function:
/// ```ignore
/// require_scope(&claims, "ingest:elements")?;
/// ```
pub fn require_scope(claims: &Claims, scope: &str) -> AppResult<()> {
    // ADMIN role has implicit access to all scopes
    if claims.roles.iter().any(|r| r == "ADMIN") {
        return Ok(());
    }

    // Check if the scope is in the claims roles (API key scopes are stored as roles)
    if claims.roles.iter().any(|r| r == scope) {
        return Ok(());
    }

    Err(AppError::Forbidden(format!(
        "missing required scope: {scope}"
    )))
}
