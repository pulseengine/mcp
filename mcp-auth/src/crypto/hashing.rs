//! Secure hashing for API keys
//!
//! This module implements secure hashing using SHA256 HMAC and salt,
//! following best practices from the Loxone MCP implementation.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fmt;

/// Salt for key derivation (32 bytes = 256 bits)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Salt(pub [u8; 32]);

impl Default for Salt {
    fn default() -> Self {
        Self::new()
    }
}

impl Salt {
    /// Create a new random salt
    pub fn new() -> Self {
        let mut salt = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);
        Salt(salt)
    }

    /// Create a salt from a base64 string
    pub fn from_base64(s: &str) -> Result<Self, HashingError> {
        let bytes = BASE64
            .decode(s)
            .map_err(|e| HashingError::InvalidSalt(format!("Invalid base64: {e}")))?;

        if bytes.len() != 32 {
            return Err(HashingError::InvalidSalt(format!(
                "Salt must be 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut salt = [0u8; 32];
        salt.copy_from_slice(&bytes);
        Ok(Salt(salt))
    }

    /// Convert salt to base64 string
    pub fn to_base64(&self) -> String {
        BASE64.encode(&self.0)
    }
}

impl fmt::Display for Salt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base64())
    }
}

/// Hashing errors
#[derive(Debug, thiserror::Error)]
pub enum HashingError {
    #[error("Invalid salt: {0}")]
    InvalidSalt(String),

    #[error("Invalid hash format: {0}")]
    InvalidHash(String),

    #[error("Hash verification failed")]
    VerificationFailed,
}

/// Generate a new random salt
pub fn generate_salt() -> Salt {
    Salt::new()
}

/// Hash an API key with salt using SHA256
///
/// This implements a similar approach to Loxone's password hashing:
/// hash = SHA256(key + ":" + salt)
pub fn hash_api_key(api_key: &str, salt: &Salt) -> String {
    // Combine key and salt with separator (like Loxone's pwd_salt)
    let salted = format!("{}:{}", api_key, salt.to_base64());

    // Hash using SHA256
    let mut hasher = Sha256::new();
    hasher.update(salted.as_bytes());
    let hash = hasher.finalize();

    // Return as base64 (more compact than hex)
    BASE64.encode(&hash)
}

/// Verify an API key against a stored hash
pub fn verify_api_key(api_key: &str, stored_hash: &str, salt: &Salt) -> Result<bool, HashingError> {
    let computed_hash = hash_api_key(api_key, salt);

    // Constant-time comparison to prevent timing attacks
    use subtle::ConstantTimeEq;
    let stored_bytes = stored_hash.as_bytes();
    let computed_bytes = computed_hash.as_bytes();

    if stored_bytes.len() != computed_bytes.len() {
        return Ok(false);
    }

    Ok(stored_bytes.ct_eq(computed_bytes).into())
}

/// Hash data using HMAC-SHA256 (for token generation)
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salt_generation() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();

        // Salts should be different
        assert_ne!(salt1.0, salt2.0);

        // Test base64 round trip
        let base64 = salt1.to_base64();
        let salt1_restored = Salt::from_base64(&base64).unwrap();
        assert_eq!(salt1, salt1_restored);
    }

    #[test]
    fn test_api_key_hashing() {
        let api_key = "test-api-key-12345";
        let salt = generate_salt();

        let hash1 = hash_api_key(api_key, &salt);
        let hash2 = hash_api_key(api_key, &salt);

        // Same input should produce same hash
        assert_eq!(hash1, hash2);

        // Different salt should produce different hash
        let salt2 = generate_salt();
        let hash3 = hash_api_key(api_key, &salt2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_api_key_verification() {
        let api_key = "test-api-key-12345";
        let salt = generate_salt();
        let hash = hash_api_key(api_key, &salt);

        // Correct key should verify
        assert!(verify_api_key(api_key, &hash, &salt).unwrap());

        // Wrong key should not verify
        assert!(!verify_api_key("wrong-key", &hash, &salt).unwrap());

        // Wrong salt should not verify
        let wrong_salt = generate_salt();
        assert!(!verify_api_key(api_key, &hash, &wrong_salt).unwrap());
    }

    #[test]
    fn test_hmac_sha256() {
        let key = b"test-key";
        let data = b"test-data";

        let hmac1 = hmac_sha256(key, data);
        let hmac2 = hmac_sha256(key, data);

        // Same input should produce same HMAC
        assert_eq!(hmac1, hmac2);

        // Different key should produce different HMAC
        let hmac3 = hmac_sha256(b"different-key", data);
        assert_ne!(hmac1, hmac3);
    }
}
