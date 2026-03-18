pub mod middleware;

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Claims extracted from a validated JWT token.
///
/// Issued by the dev-login endpoint (or Entra ID callback in production).
/// Injected into request extensions by the `require_auth` middleware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// User ID (UUID)
    pub sub: Uuid,
    /// User email
    pub email: String,
    /// User display name
    pub display_name: String,
    /// Role codes assigned to the user (e.g. ["ADMIN", "DATA_STEWARD"])
    pub roles: Vec<String>,
    /// Expiry timestamp (seconds since epoch)
    pub exp: usize,
    /// Issued at timestamp (seconds since epoch)
    pub iat: usize,
}

/// Create a signed JWT token for the given user.
pub fn create_token(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    roles: Vec<String>,
    secret: &str,
    expiry_hours: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        display_name: display_name.to_string(),
        roles,
        exp: now + (expiry_hours as usize * 3600),
        iat: now,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate a JWT token and extract claims.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_validate_token_round_trip() {
        let user_id = Uuid::new_v4();
        let token = create_token(
            user_id,
            "admin@example.com",
            "Admin User",
            vec!["ADMIN".to_string()],
            "test-secret-key-at-least-32-chars-long",
            8,
        )
        .expect("token creation should succeed");

        let claims = validate_token(&token, "test-secret-key-at-least-32-chars-long")
            .expect("validation should succeed");

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, "admin@example.com");
        assert_eq!(claims.display_name, "Admin User");
        assert_eq!(claims.roles, vec!["ADMIN"]);
    }

    #[test]
    fn validate_rejects_wrong_secret() {
        let token = create_token(
            Uuid::new_v4(),
            "user@example.com",
            "User",
            vec![],
            "correct-secret-key-at-least-32-chars",
            8,
        )
        .unwrap();

        let result = validate_token(&token, "wrong-secret-key-at-least-32-chars!");
        assert!(result.is_err());
    }

    #[test]
    fn validate_rejects_expired_token() {
        let now = Utc::now().timestamp() as usize;
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "user@example.com".to_string(),
            display_name: "User".to_string(),
            roles: vec![],
            exp: now - 3600, // expired 1 hour ago
            iat: now - 7200,
        };
        let token = encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &EncodingKey::from_secret(b"test-secret-key-at-least-32-chars-long"),
        )
        .unwrap();

        let result = validate_token(&token, "test-secret-key-at-least-32-chars-long");
        assert!(result.is_err());
    }
}
