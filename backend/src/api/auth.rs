use axum::Json;
use axum::extract::{Query, Request, State};
use axum::http::header;
use axum::response::{IntoResponse, Redirect, Response};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

/// Characters allowed in a PKCE code_verifier (RFC 7636 Section 4.1).
const PKCE_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// Generate a PKCE code_verifier and its S256 code_challenge.
fn generate_pkce() -> (String, String) {
    let mut rng = rand::rng();
    let verifier: String = (0..64)
        .map(|_| {
            let idx = rng.random_range(0..PKCE_CHARSET.len());
            PKCE_CHARSET[idx] as char
        })
        .collect();
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

/// Generate a random hex string for the OAuth `state` parameter.
fn generate_state() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 16] = rng.random();
    hex::encode(&bytes)
}

/// Simple hex encoding (avoids pulling in the `hex` crate).
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

// ---------------------------------------------------------------------------
// OAuth state cookie helpers
// ---------------------------------------------------------------------------

/// Payload stored inside the `oauth_state` cookie.
#[derive(Serialize, Deserialize)]
struct OAuthStateCookie {
    verifier: String,
    state: String,
    /// Expiry as seconds-since-epoch.
    exp: u64,
}

/// Build a `Set-Cookie` header value for the OAuth state cookie.
fn build_oauth_state_cookie(verifier: &str, state: &str, jwt_secret: &str) -> String {
    let exp = chrono::Utc::now().timestamp() as u64 + 600; // 10 minutes
    let payload = OAuthStateCookie {
        verifier: verifier.to_string(),
        state: state.to_string(),
        exp,
    };
    let json = serde_json::to_string(&payload).expect("serialise oauth state");
    let encoded = STANDARD.encode(json.as_bytes());

    // Sign with HMAC-SHA256 using JWT_SECRET so the cookie cannot be tampered with.
    let mut mac = Sha256::new();
    mac.update(jwt_secret.as_bytes());
    mac.update(encoded.as_bytes());
    let sig = URL_SAFE_NO_PAD.encode(mac.finalize());

    let value = format!("{encoded}.{sig}");
    format!("oauth_state={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age=600")
}

/// Parse and validate the `oauth_state` cookie. Returns `(verifier, state)`.
fn parse_oauth_state_cookie(
    raw_cookie_header: &str,
    jwt_secret: &str,
) -> Result<(String, String), AppError> {
    // Extract the oauth_state value from the Cookie header
    let cookie_value = raw_cookie_header
        .split(';')
        .filter_map(|segment| {
            let segment = segment.trim();
            segment.strip_prefix("oauth_state=")
        })
        .next()
        .ok_or_else(|| AppError::BadRequest("missing oauth_state cookie".into()))?;

    let (encoded, sig) = cookie_value
        .rsplit_once('.')
        .ok_or_else(|| AppError::BadRequest("malformed oauth_state cookie".into()))?;

    // Verify signature
    let mut mac = Sha256::new();
    mac.update(jwt_secret.as_bytes());
    mac.update(encoded.as_bytes());
    let expected_sig = URL_SAFE_NO_PAD.encode(mac.finalize());
    if sig != expected_sig {
        return Err(AppError::BadRequest(
            "oauth_state cookie signature invalid".into(),
        ));
    }

    let json_bytes = STANDARD
        .decode(encoded)
        .map_err(|_| AppError::BadRequest("oauth_state cookie decode failed".into()))?;
    let payload: OAuthStateCookie = serde_json::from_slice(&json_bytes)
        .map_err(|_| AppError::BadRequest("oauth_state cookie parse failed".into()))?;

    let now = chrono::Utc::now().timestamp() as u64;
    if now > payload.exp {
        return Err(AppError::BadRequest("oauth_state cookie expired".into()));
    }

    Ok((payload.verifier, payload.state))
}

// ---------------------------------------------------------------------------
// Entra ID token response
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct EntraTokenResponse {
    id_token: Option<String>,
    #[allow(dead_code)]
    access_token: Option<String>,
}

/// Minimal set of claims we extract from the Entra ID token.
#[derive(Deserialize)]
struct EntraIdClaims {
    /// Object ID
    #[allow(dead_code)]
    #[serde(default)]
    oid: Option<String>,
    /// Email address
    #[serde(default)]
    email: Option<String>,
    /// Preferred username (fallback for email)
    #[serde(default)]
    preferred_username: Option<String>,
    /// Display name
    #[serde(default)]
    name: Option<String>,
}

/// Decode the payload of a JWT without verifying the signature.
/// We trust the token because we just received it from Microsoft's token
/// endpoint over HTTPS.
fn decode_jwt_payload<T: serde::de::DeserializeOwned>(token: &str) -> Result<T, AppError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "ID token is not a valid JWT (expected 3 parts, got {})",
            parts.len()
        )));
    }
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| {
            // Microsoft sometimes uses standard base64 with padding in JWTs
            base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1])
        })
        .map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "failed to base64-decode ID token payload: {e}"
            ))
        })?;
    serde_json::from_slice(&payload_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to parse ID token payload: {e}")))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

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
    // SEC-025: Input length validation
    if body.email.len() > 320 {
        return Err(AppError::Validation("email exceeds maximum length".into()));
    }
    if body.password.len() > 128 {
        return Err(AppError::Validation(
            "password exceeds maximum length".into(),
        ));
    }

    // SEC-021: Block dev-login when Entra SSO is properly configured
    // Check that tenant ID is not empty, not the placeholder, and looks like a UUID
    let entra_configured = !state.config.entra.tenant_id.is_empty()
        && state.config.entra.tenant_id != "your-tenant-id"
        && uuid::Uuid::parse_str(&state.config.entra.tenant_id).is_ok();

    if entra_configured {
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
/// Generates PKCE code_verifier/challenge and state, stores them in a signed
/// cookie, and redirects the browser to the Entra ID authorization endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/auth/login",
    responses(
        (status = 302, description = "Redirect to Entra ID login")
    ),
    tag = "auth"
)]
pub async fn login(State(state): State<AppState>) -> Response {
    let (verifier, challenge) = generate_pkce();
    let oauth_state = generate_state();

    let cookie = build_oauth_state_cookie(&verifier, &oauth_state, &state.config.jwt_secret);

    let auth_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize\
         ?client_id={}\
         &response_type=code\
         &redirect_uri={}\
         &scope=openid%20email%20profile%20User.Read\
         &state={}\
         &code_challenge={}\
         &code_challenge_method=S256",
        state.config.entra.tenant_id,
        state.config.entra.client_id,
        urlencoding::encode(&state.config.entra.redirect_uri),
        oauth_state,
        challenge,
    );

    (
        [(header::SET_COOKIE, cookie)],
        Redirect::temporary(&auth_url),
    )
        .into_response()
}

/// Handle SSO callback from Microsoft Entra ID.
///
/// Validates the state parameter, exchanges the authorization code for tokens
/// using PKCE, decodes the ID token to extract user identity, auto-provisions
/// the user in our database if needed, and redirects to the frontend with a JWT.
#[utoipa::path(
    get,
    path = "/api/v1/auth/callback",
    responses(
        (status = 302, description = "Redirect to frontend with JWT token")
    ),
    tag = "auth"
)]
pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<AuthCallback>,
    request: Request,
) -> Result<Response, AppError> {
    // --- Extract and validate the oauth_state cookie -----------------------
    let cookie_header = request
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("missing cookie header".into()))?;

    let (verifier, expected_state) =
        parse_oauth_state_cookie(cookie_header, &state.config.jwt_secret)?;

    let received_state = params
        .state
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("missing state parameter".into()))?;

    if received_state != expected_state {
        return Err(AppError::BadRequest("state parameter mismatch".into()));
    }

    // --- Exchange authorization code for tokens ----------------------------
    let token_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        state.config.entra.tenant_id,
    );

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("http client error: {e}")))?;

    let token_response = client
        .post(&token_url)
        .form(&[
            ("client_id", state.config.entra.client_id.as_str()),
            ("client_secret", state.config.entra.client_secret.as_str()),
            ("code", params.code.as_str()),
            ("redirect_uri", state.config.entra.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Entra token exchange request failed");
            AppError::Internal(anyhow::anyhow!("Entra token exchange failed: {e}"))
        })?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await.unwrap_or_default();
        tracing::error!(
            status = %status,
            body = %body,
            "Entra token exchange returned error"
        );
        return Err(AppError::Internal(anyhow::anyhow!(
            "Entra token exchange returned {status}"
        )));
    }

    let entra_tokens: EntraTokenResponse = token_response.json().await.map_err(|e| {
        AppError::Internal(anyhow::anyhow!("failed to parse Entra token response: {e}"))
    })?;

    let id_token = entra_tokens.id_token.ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("Entra token response missing id_token"))
    })?;

    // --- Decode ID token (no signature verification — we trust the HTTPS channel) ---
    let id_claims: EntraIdClaims = decode_jwt_payload(&id_token)?;

    let email = id_claims
        .email
        .or(id_claims.preferred_username)
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "ID token contains neither email nor preferred_username"
            ))
        })?;

    let display_name = id_claims.name.unwrap_or_else(|| email.clone());

    // --- Fetch department and job title from Microsoft Graph API ---
    let (department, job_title) = if let Some(ref at) = entra_tokens.access_token {
        match client
            .get("https://graph.microsoft.com/v1.0/me?$select=department,jobTitle")
            .header("Authorization", format!("Bearer {at}"))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let dept = body.get("department").and_then(|v| v.as_str()).map(String::from);
                let title = body.get("jobTitle").and_then(|v| v.as_str()).map(String::from);
                tracing::debug!(department = ?dept, job_title = ?title, "fetched profile from Graph API");
                (dept, title)
            }
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    // --- Auto-provision or update user ------------------------------------
    let user_row = sqlx::query_as::<_, SsoUserRow>(
        r#"
        SELECT u.user_id, u.email, u.display_name, u.is_active,
               COALESCE(
                   array_agg(r.role_code) FILTER (WHERE r.role_code IS NOT NULL),
                   ARRAY[]::VARCHAR[]
               ) as roles
        FROM users u
        LEFT JOIN user_roles ur ON u.user_id = ur.user_id
            AND (ur.effective_to IS NULL OR ur.effective_to > NOW())
        LEFT JOIN roles r ON ur.role_id = r.role_id
        WHERE u.email = $1 AND u.deleted_at IS NULL
        GROUP BY u.user_id, u.email, u.display_name, u.is_active
        "#,
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    let (user_id, user_email, user_display_name, roles) = match user_row {
        Some(row) => {
            if !row.is_active {
                return Err(AppError::Forbidden("account is disabled".into()));
            }
            // Update last_login_at, department, job_title from Entra profile
            sqlx::query(
                "UPDATE users SET last_login_at = NOW(), department = COALESCE($2, department), job_title = COALESCE($3, job_title) WHERE user_id = $1"
            )
                .bind(row.user_id)
                .bind(department.as_deref())
                .bind(job_title.as_deref())
                .execute(&state.pool)
                .await
                .map_err(|e| AppError::Internal(e.into()))?;

            (row.user_id, row.email, row.display_name, row.roles)
        }
        None => {
            // Auto-provision new user
            tracing::info!(email = %email, display_name = %display_name, "auto-provisioning new SSO user");

            // Generate username from email (part before @)
            let username = email.split('@').next().unwrap_or(&email)
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");

            let new_user_id: uuid::Uuid = sqlx::query_scalar(
                r#"
                INSERT INTO users (username, email, display_name, department, job_title, is_active, roles_reviewed, last_login_at)
                VALUES ($1, $2, $3, $4, $5, TRUE, FALSE, NOW())
                RETURNING user_id
                "#,
            )
            .bind(&username)
            .bind(&email)
            .bind(&display_name)
            .bind(department.as_deref())
            .bind(job_title.as_deref())
            .fetch_one(&state.pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

            // Assign default role: DATA_CONSUMER
            sqlx::query(
                r#"
                INSERT INTO user_roles (user_id, role_id, granted_by, effective_from)
                SELECT $1, role_id, $1, NOW()
                FROM roles
                WHERE role_code = 'DATA_CONSUMER'
                "#,
            )
            .bind(new_user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

            (
                new_user_id,
                email.clone(),
                display_name.clone(),
                vec!["DATA_CONSUMER".to_string()],
            )
        }
    };

    // --- Issue our JWT -----------------------------------------------------
    let token = crate::auth::create_token(
        user_id,
        &user_email,
        &user_display_name,
        roles,
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("token creation failed: {e}")))?;

    // --- Redirect to frontend with token -----------------------------------
    let redirect_url = format!(
        "{}/auth/callback?token={}",
        state.config.frontend_url, token
    );

    // Clear the oauth_state cookie
    let clear_cookie = "oauth_state=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    Ok((
        [(header::SET_COOKIE, clear_cookie.to_string())],
        Redirect::temporary(&redirect_url),
    )
        .into_response())
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

// ---------------------------------------------------------------------------
// me_profile — GET /api/v1/auth/me/profile
// ---------------------------------------------------------------------------

/// Retrieve the full profile for the currently authenticated user.
/// Returns the same data as the admin user detail endpoint but without requiring
/// ADMIN role — users can only view their own profile.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me/profile",
    responses(
        (status = 200, description = "Current user profile with roles",
         body = crate::domain::users::UserWithRoles),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn me_profile(
    State(state): State<AppState>,
    request: Request,
) -> AppResult<Json<crate::domain::users::UserWithRoles>> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AppError::Unauthorized("not authenticated".into()))?
        .clone();

    let user = sqlx::query_as::<_, crate::domain::users::User>(
        r#"
        SELECT user_id, username, email, display_name, first_name, last_name,
               department, job_title, entra_object_id, is_active, roles_reviewed,
               last_login_at, created_at, updated_at
        FROM users
        WHERE user_id = $1
        "#,
    )
    .bind(claims.sub)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let roles = sqlx::query_as::<_, crate::domain::users::Role>(
        r#"
        SELECT r.role_id, r.role_code, r.role_name, r.description, r.is_system_role
        FROM roles r
        JOIN user_roles ur ON ur.role_id = r.role_id
        WHERE ur.user_id = $1
        ORDER BY r.role_name ASC
        "#,
    )
    .bind(claims.sub)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(crate::domain::users::UserWithRoles::from_user_and_roles(user, roles)))
}

/// Check whether Entra SSO is configured (returns true when the tenant ID is
/// a valid UUID and not the placeholder).
pub fn is_entra_configured(tenant_id: &str) -> bool {
    !tenant_id.is_empty()
        && tenant_id != "your-tenant-id"
        && uuid::Uuid::parse_str(tenant_id).is_ok()
}

/// Internal row type for user authentication query (dev-login with password hash).
#[derive(sqlx::FromRow)]
struct UserAuthRow {
    user_id: uuid::Uuid,
    email: String,
    display_name: String,
    password_hash: Option<String>,
    is_active: bool,
    roles: Vec<String>,
}

/// Internal row type for SSO user lookup (no password hash needed).
#[derive(sqlx::FromRow)]
struct SsoUserRow {
    user_id: uuid::Uuid,
    email: String,
    display_name: String,
    is_active: bool,
    roles: Vec<String>,
}
