//! Secure key generation and derivation
//!
//! This module provides secure key generation similar to Loxone's
//! approach, with URL-safe encoding and proper randomness.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distributions::Alphanumeric, Rng, RngCore};

/// Key derivation errors
#[derive(Debug, thiserror::Error)]
pub enum KeyDerivationError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Derivation failed: {0}")]
    DerivationFailed(String),
}

/// Generate a secure API key
///
/// This generates a URL-safe base64 encoded random key,
/// similar to Loxone's generate_api_key function.
pub fn generate_secure_key() -> String {
    // Generate 32 bytes of randomness (256 bits)
    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);

    // Encode as URL-safe base64 without padding
    URL_SAFE_NO_PAD.encode(&key_bytes)
}

/// Generate a secure key with custom length
pub fn generate_secure_key_with_length(bytes: usize) -> String {
    let mut key_bytes = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut key_bytes);

    URL_SAFE_NO_PAD.encode(&key_bytes)
}

/// Generate a human-friendly API key prefix
///
/// Format: lmcp_{role}_{timestamp}_{random}
/// This matches Loxone's key ID format
pub fn generate_key_id(role: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp();
    let random: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    format!("lmcp_{}_{timestamp}_{random}", role.to_lowercase())
}

/// Derive a key from user input using PBKDF2
///
/// This is for cases where we need to derive a key from a password
/// or other user input, with proper key stretching.
pub fn derive_key(
    input: &str,
    salt: &[u8],
    iterations: u32,
) -> Result<[u8; 32], KeyDerivationError> {
    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;

    if input.is_empty() {
        return Err(KeyDerivationError::InvalidInput("Empty input".to_string()));
    }

    if salt.is_empty() {
        return Err(KeyDerivationError::InvalidInput("Empty salt".to_string()));
    }

    if iterations == 0 {
        return Err(KeyDerivationError::InvalidInput(
            "Iterations must be > 0".to_string(),
        ));
    }

    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(input.as_bytes(), salt, iterations, &mut key);

    Ok(key)
}

/// Generate a master key from environment or secure storage
///
/// This is used to derive all other encryption keys
pub fn generate_master_key() -> Result<[u8; 32], KeyDerivationError> {
    generate_master_key_for_application(None)
}

/// Generate an application-specific master key from environment or secure storage
///
/// This checks for app-specific environment variables first, then falls back to generic ones
pub fn generate_master_key_for_application(app_name: Option<&str>) -> Result<[u8; 32], KeyDerivationError> {
    // In production, this should come from secure storage (HSM, vault, etc.)
    // For now, we'll check environment variable or generate a new one

    // First try app-specific environment variable if app_name is provided
    if let Some(app) = app_name {
        let app_specific_var = format!("PULSEENGINE_MCP_MASTER_KEY_{}", app.to_uppercase().replace('-', "_"));
        if let Ok(master_key_b64) = std::env::var(&app_specific_var) {
            return decode_master_key(&master_key_b64);
        }
    }

    // Fall back to generic environment variable
    if let Ok(master_key_b64) = std::env::var("PULSEENGINE_MCP_MASTER_KEY") {
        return decode_master_key(&master_key_b64);
    }

    // Generate a new master key
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);

    // Log warning about using generated key
    tracing::warn!(
        "Generated new master key. Set PULSEENGINE_MCP_MASTER_KEY={} for persistence",
        URL_SAFE_NO_PAD.encode(&key)
    );

    Ok(key)
}

/// Decode a base64-encoded master key
fn decode_master_key(master_key_b64: &str) -> Result<[u8; 32], KeyDerivationError> {
    let key_bytes = URL_SAFE_NO_PAD
        .decode(master_key_b64)
        .map_err(|e| KeyDerivationError::InvalidInput(format!("Invalid master key: {}", e)))?;

    if key_bytes.len() != 32 {
        return Err(KeyDerivationError::InvalidInput(format!(
            "Master key must be 32 bytes, got {}",
            key_bytes.len()
        )));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_key() {
        let key1 = generate_secure_key();
        let key2 = generate_secure_key();

        // Keys should be different
        assert_ne!(key1, key2);

        // Keys should be URL-safe base64 (43 chars for 32 bytes without padding)
        assert_eq!(key1.len(), 43);
        assert!(!key1.contains('+'));
        assert!(!key1.contains('/'));
        assert!(!key1.contains('='));
    }

    #[test]
    fn test_generate_key_id() {
        let id1 = generate_key_id("admin");
        let id2 = generate_key_id("admin");

        // IDs should be different (different timestamp/random)
        assert_ne!(id1, id2);

        // Check format
        assert!(id1.starts_with("lmcp_admin_"));
        assert!(id1.matches('_').count() == 3);
    }

    #[test]
    fn test_derive_key() {
        let password = "test-password";
        let salt = b"test-salt-1234567890";

        let key1 = derive_key(password, salt, 1000).unwrap();
        let key2 = derive_key(password, salt, 1000).unwrap();

        // Same input should produce same key
        assert_eq!(key1, key2);

        // Different salt should produce different key
        let key3 = derive_key(password, b"different-salt", 1000).unwrap();
        assert_ne!(key1, key3);

        // Different iterations should produce different key
        let key4 = derive_key(password, salt, 2000).unwrap();
        assert_ne!(key1, key4);
    }

    #[test]
    fn test_derive_key_validation() {
        // Empty input should fail
        assert!(derive_key("", b"salt", 1000).is_err());

        // Empty salt should fail
        assert!(derive_key("password", b"", 1000).is_err());

        // Zero iterations should fail
        assert!(derive_key("password", b"salt", 0).is_err());
    }
}
