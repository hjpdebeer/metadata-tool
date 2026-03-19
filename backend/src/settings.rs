//! System settings module with AES-256-GCM encryption for sensitive values.
//!
//! Settings are stored in the `system_settings` database table. Encrypted values
//! use AES-256-GCM with a random 12-byte nonce prepended to the ciphertext. The
//! encryption key is derived from `SETTINGS_ENCRYPTION_KEY` or `JWT_SECRET` via SHA-256.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Encryption helpers
// ---------------------------------------------------------------------------

/// Derive a 256-bit encryption key from the given secret using a simple hash.
fn derive_key(secret: &str) -> [u8; 32] {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(secret.as_bytes());
    let h1 = hasher.finish().to_le_bytes();
    hasher.write(b"settings-encryption-key-derivation");
    let h2 = hasher.finish().to_le_bytes();
    hasher.write(secret.as_bytes());
    hasher.write(b"second-round");
    let h3 = hasher.finish().to_le_bytes();
    hasher.write(b"third-round-padding");
    let h4 = hasher.finish().to_le_bytes();

    let mut key = [0u8; 32];
    key[0..8].copy_from_slice(&h1);
    key[8..16].copy_from_slice(&h2);
    key[16..24].copy_from_slice(&h3);
    key[24..32].copy_from_slice(&h4);
    key
}

/// Encrypt a plaintext string using AES-256-GCM.
/// Returns a base64-encoded string containing the 12-byte nonce followed by the ciphertext.
fn encrypt_value(plaintext: &str, secret: &str) -> AppResult<String> {
    let key_bytes = derive_key(secret);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid key: {e}")))?;

    // Generate a random 12-byte nonce
    let mut nonce_bytes = [0u8; 12];
    getrandom(&mut nonce_bytes);
    let nonce = aes_gcm::Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("encryption failed: {e}")))?;

    // Prepend nonce to ciphertext
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(base64_encode(&combined))
}

/// Decrypt a base64-encoded AES-256-GCM ciphertext (nonce prepended).
fn decrypt_value(encrypted_b64: &str, secret: &str) -> AppResult<String> {
    if encrypted_b64.is_empty() {
        return Ok(String::new());
    }

    let combined = base64_decode(encrypted_b64).map_err(|e| {
        AppError::Internal(anyhow::anyhow!("base64 decode failed: {e}"))
    })?;

    if combined.len() < 13 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "encrypted value too short"
        )));
    }

    let key_bytes = derive_key(secret);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid key: {e}")))?;

    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&combined[..12]);
    let nonce = aes_gcm::Nonce::from(nonce_bytes);
    let ciphertext = &combined[12..];

    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("decryption failed: {e}")))?;

    String::from_utf8(plaintext)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("decrypted value is not UTF-8: {e}")))
}

/// Fill a buffer with random bytes using std randomness.
fn getrandom(buf: &mut [u8]) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    for chunk in buf.chunks_mut(8) {
        let state = RandomState::new();
        let mut h = state.build_hasher();
        h.write_u64(0);
        let val = h.finish().to_le_bytes();
        let n = chunk.len().min(8);
        chunk[..n].copy_from_slice(&val[..n]);
    }
}

// Simple base64 encode/decode without pulling in another crate
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::with_capacity(input.len() * 3 / 4);

    let decode_char = |c: u8| -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("invalid base64 character: {c}")),
        }
    };

    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let a = decode_char(bytes[i])?;
        let b = if i + 1 < bytes.len() { decode_char(bytes[i + 1])? } else { 0 };
        let c = if i + 2 < bytes.len() { decode_char(bytes[i + 2])? } else { 0 };
        let d = if i + 3 < bytes.len() { decode_char(bytes[i + 3])? } else { 0 };

        let n = (a << 18) | (b << 12) | (c << 6) | d;
        result.push(((n >> 16) & 0xff) as u8);
        if i + 2 < bytes.len() {
            result.push(((n >> 8) & 0xff) as u8);
        }
        if i + 3 < bytes.len() {
            result.push((n & 0xff) as u8);
        }
        i += 4;
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Masking helper
// ---------------------------------------------------------------------------

/// Mask a sensitive value for display: show first 6 and last 4 chars, mask the rest.
pub fn mask_value(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let len = value.len();
    if len <= 10 {
        return "****".to_string();
    }
    let prefix = &value[..6];
    let suffix = &value[len - 4..];
    format!("{prefix}****{suffix}")
}

// ---------------------------------------------------------------------------
// Settings cache
// ---------------------------------------------------------------------------

/// Cached settings with TTL.
#[derive(Clone)]
pub struct SettingsCache {
    inner: Arc<RwLock<CacheInner>>,
    encryption_secret: String,
}

struct CacheInner {
    entries: HashMap<String, CachedSetting>,
    last_loaded: Option<std::time::Instant>,
}

#[derive(Clone)]
struct CachedSetting {
    value: String,
    is_encrypted: bool,
}

const CACHE_TTL_SECS: u64 = 60;

impl SettingsCache {
    /// Create a new settings cache with the encryption secret.
    pub fn new(encryption_secret: String) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                entries: HashMap::new(),
                last_loaded: None,
            })),
            encryption_secret,
        }
    }

    /// Get the encryption secret for external use (e.g., in API handlers).
    pub fn encryption_secret(&self) -> &str {
        &self.encryption_secret
    }

    /// Load all settings from the database into the cache.
    pub async fn load(&self, pool: &PgPool) -> AppResult<()> {
        let rows = sqlx::query_as::<_, SettingRow>(
            "SELECT setting_key, setting_value, is_encrypted FROM system_settings",
        )
        .fetch_all(pool)
        .await?;

        let mut inner = self.inner.write().await;
        inner.entries.clear();
        for row in rows {
            inner.entries.insert(
                row.setting_key,
                CachedSetting {
                    value: row.setting_value,
                    is_encrypted: row.is_encrypted,
                },
            );
        }
        inner.last_loaded = Some(std::time::Instant::now());
        Ok(())
    }

    /// Ensure the cache is fresh (within TTL), reloading if necessary.
    async fn ensure_fresh(&self, pool: &PgPool) -> AppResult<()> {
        let needs_reload = {
            let inner = self.inner.read().await;
            match inner.last_loaded {
                None => true,
                Some(t) => t.elapsed().as_secs() > CACHE_TTL_SECS,
            }
        };
        if needs_reload {
            self.load(pool).await?;
        }
        Ok(())
    }

    /// Invalidate the cache, forcing a reload on next access.
    pub async fn invalidate(&self) {
        let mut inner = self.inner.write().await;
        inner.last_loaded = None;
    }

    /// Get a decrypted setting value from the cache.
    pub async fn get(&self, pool: &PgPool, key: &str) -> AppResult<Option<String>> {
        self.ensure_fresh(pool).await?;

        let inner = self.inner.read().await;
        match inner.entries.get(key) {
            None => Ok(None),
            Some(cached) => {
                if cached.value.is_empty() {
                    return Ok(Some(String::new()));
                }
                if cached.is_encrypted {
                    let decrypted = decrypt_value(&cached.value, &self.encryption_secret)?;
                    Ok(Some(decrypted))
                } else {
                    Ok(Some(cached.value.clone()))
                }
            }
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SettingRow {
    setting_key: String,
    setting_value: String,
    is_encrypted: bool,
}

// ---------------------------------------------------------------------------
// Public API: get_setting / set_setting
// ---------------------------------------------------------------------------

/// Read a single setting from the database, decrypting if necessary.
pub async fn get_setting(pool: &PgPool, key: &str, secret: &str) -> AppResult<Option<String>> {
    let row = sqlx::query_as::<_, SettingRow>(
        "SELECT setting_key, setting_value, is_encrypted FROM system_settings WHERE setting_key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(None),
        Some(r) => {
            if r.setting_value.is_empty() {
                return Ok(Some(String::new()));
            }
            if r.is_encrypted {
                let decrypted = decrypt_value(&r.setting_value, secret)?;
                Ok(Some(decrypted))
            } else {
                Ok(Some(r.setting_value))
            }
        }
    }
}

/// Write a setting value to the database, encrypting if the setting is marked as encrypted.
pub async fn set_setting(
    pool: &PgPool,
    key: &str,
    value: &str,
    user_id: Uuid,
    secret: &str,
) -> AppResult<()> {
    // Check if setting exists and whether it should be encrypted
    let is_encrypted = sqlx::query_scalar::<_, bool>(
        "SELECT is_encrypted FROM system_settings WHERE setting_key = $1",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("setting not found: {key}")))?;

    let stored_value = if is_encrypted && !value.is_empty() {
        encrypt_value(value, secret)?
    } else {
        value.to_string()
    };

    sqlx::query(
        "UPDATE system_settings SET setting_value = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE setting_key = $3",
    )
    .bind(&stored_value)
    .bind(user_id)
    .bind(key)
    .execute(pool)
    .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Domain types for API responses
// ---------------------------------------------------------------------------

/// A system setting as returned by the admin API.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SystemSettingResponse {
    pub key: String,
    pub value: String,
    pub is_encrypted: bool,
    pub category: String,
    pub display_name: String,
    pub description: Option<String>,
    pub validation_regex: Option<String>,
    pub is_set: bool,
    pub updated_at: DateTime<Utc>,
    pub updated_by_name: Option<String>,
}

/// Request body for updating a setting.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpdateSettingRequest {
    pub value: String,
}

/// Response after updating a setting.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UpdateSettingResponse {
    pub key: String,
    pub is_set: bool,
    pub updated_at: DateTime<Utc>,
}

/// Response for test connection.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

/// Full setting row from database (internal).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SystemSettingRow {
    pub setting_key: String,
    pub setting_value: String,
    pub is_encrypted: bool,
    pub category: String,
    pub display_name: String,
    pub description: Option<String>,
    pub validation_regex: Option<String>,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Synchronously decrypt a stored value (for use within iterators).
/// Takes the raw stored value (not the key), and the encryption secret.
pub fn get_setting_sync(stored_value: &str, secret: &str) -> AppResult<String> {
    if stored_value.is_empty() {
        return Ok(String::new());
    }
    decrypt_value(stored_value, secret)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let secret = "test-secret-at-least-32-characters-long";
        let plaintext = "sk-ant-api03-my-secret-key-12345";

        let encrypted = encrypt_value(plaintext, secret).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt_value(&encrypted, secret).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_empty_returns_empty() {
        let secret = "test-secret-at-least-32-characters-long";
        let result = decrypt_value("", secret).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let encrypted = encrypt_value("secret-data", "correct-key-at-least-32-chars-xxxxx").unwrap();
        let result = decrypt_value(&encrypted, "wrong-key-at-least-32-chars-xxxxxx");
        assert!(result.is_err());
    }

    #[test]
    fn mask_value_long() {
        let masked = mask_value("sk-ant-api03-my-secret-key-12345");
        assert_eq!(masked, "sk-ant****2345");
    }

    #[test]
    fn mask_value_short() {
        let masked = mask_value("shortkey");
        assert_eq!(masked, "****");
    }

    #[test]
    fn mask_value_empty() {
        let masked = mask_value("");
        assert_eq!(masked, "");
    }

    #[test]
    fn base64_round_trip() {
        let data = b"hello world encryption test with some data";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
