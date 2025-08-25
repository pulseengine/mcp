//! Utility functions for security operations

use crate::error::{SecurityError, SecurityResult};
use base64::{Engine as _, engine::general_purpose};
use rand::{Rng, distributions::Alphanumeric};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Secure random generator for security operations
pub struct SecureRandom;

impl SecureRandom {
    /// Generate cryptographically secure random bytes
    pub fn bytes(length: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        (0..length).map(|_| rng.r#gen()).collect()
    }

    /// Generate cryptographically secure random string
    pub fn string(length: usize) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect()
    }

    /// Generate base64-encoded random string
    pub fn base64_string(byte_length: usize) -> String {
        let bytes = Self::bytes(byte_length);
        general_purpose::STANDARD.encode(bytes)
    }

    /// Generate URL-safe base64-encoded random string
    pub fn base64_url_string(byte_length: usize) -> String {
        let bytes = Self::bytes(byte_length);
        general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }
}

/// Generate a secure API key
///
/// API keys are prefixed with "mcp_" and contain 32 bytes of random data
/// encoded in base64-url format for URL safety.
///
/// # Example
/// ```rust
/// use pulseengine_mcp_security_middleware::generate_api_key;
///
/// let api_key = generate_api_key();
/// assert!(api_key.starts_with("mcp_"));
/// assert!(api_key.len() > 20); // At least 20 characters
/// ```
pub fn generate_api_key() -> String {
    let random_part = SecureRandom::base64_url_string(32);
    format!("mcp_{random_part}")
}

/// Generate a secure JWT secret
///
/// JWT secrets are 64 bytes of cryptographically secure random data
/// encoded in base64 format.
///
/// # Example
/// ```rust
/// use pulseengine_mcp_security_middleware::generate_jwt_secret;
///
/// let secret = generate_jwt_secret();
/// assert!(secret.len() >= 64); // At least 64 characters for security
/// ```
pub fn generate_jwt_secret() -> String {
    SecureRandom::base64_string(64)
}

/// Hash an API key for storage
///
/// Uses SHA-256 to hash API keys for secure storage. The original key
/// should never be stored, only the hash.
pub fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let result = hasher.finalize();
    general_purpose::STANDARD.encode(result)
}

/// Verify an API key against its hash
///
/// Compares the hash of the provided API key with the stored hash.
pub fn verify_api_key(api_key: &str, stored_hash: &str) -> bool {
    let computed_hash = hash_api_key(api_key);
    computed_hash == stored_hash
}

/// Get current Unix timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// Validate that a string looks like a valid API key
pub fn validate_api_key_format(key: &str) -> SecurityResult<()> {
    if !key.starts_with("mcp_") {
        return Err(SecurityError::invalid_token(
            "API key must start with 'mcp_'",
        ));
    }

    if key.len() < 20 {
        return Err(SecurityError::invalid_token("API key too short"));
    }

    if key.len() > 200 {
        return Err(SecurityError::invalid_token("API key too long"));
    }

    // Check that it contains only valid base64-url characters after prefix
    let key_part = &key[4..]; // Skip "mcp_" prefix
    for c in key_part.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' {
            return Err(SecurityError::invalid_token(
                "Invalid characters in API key",
            ));
        }
    }

    Ok(())
}

/// Generate a secure session ID
pub fn generate_session_id() -> String {
    format!("sess_{}", SecureRandom::base64_url_string(32))
}

/// Generate a secure request ID for tracing
pub fn generate_request_id() -> String {
    format!("req_{}", SecureRandom::base64_url_string(16))
}

/// Safe comparison function to prevent timing attacks
pub fn secure_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();

        // Keys should be different
        assert_ne!(key1, key2);

        // Both should start with prefix
        assert!(key1.starts_with("mcp_"));
        assert!(key2.starts_with("mcp_"));

        // Should have reasonable length
        assert!(key1.len() > 20);
        assert!(key2.len() > 20);
    }

    #[test]
    fn test_generate_jwt_secret() {
        let secret1 = generate_jwt_secret();
        let secret2 = generate_jwt_secret();

        // Secrets should be different
        assert_ne!(secret1, secret2);

        // Should have sufficient length for security
        assert!(secret1.len() >= 64);
        assert!(secret2.len() >= 64);
    }

    #[test]
    fn test_hash_and_verify_api_key() {
        let api_key = generate_api_key();
        let hash = hash_api_key(&api_key);

        // Verification should succeed
        assert!(verify_api_key(&api_key, &hash));

        // Wrong key should fail
        let wrong_key = generate_api_key();
        assert!(!verify_api_key(&wrong_key, &hash));
    }

    #[test]
    fn test_validate_api_key_format() {
        // Valid key should pass
        let valid_key = generate_api_key();
        assert!(validate_api_key_format(&valid_key).is_ok());

        // Invalid keys should fail
        assert!(validate_api_key_format("invalid").is_err());
        assert!(validate_api_key_format("api_too_short").is_err());
        assert!(validate_api_key_format("mcp_").is_err());
    }

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare("hello", "hello"));
        assert!(!secure_compare("hello", "world"));
        assert!(!secure_compare("hello", "hello world"));
        assert!(!secure_compare("", "hello"));
    }

    #[test]
    fn test_session_id_generation() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("sess_"));
        assert!(id2.starts_with("sess_"));
    }

    #[test]
    fn test_request_id_generation() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("req_"));
        assert!(id2.starts_with("req_"));
    }

    #[test]
    fn test_current_timestamp() {
        let ts1 = current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let ts2 = current_timestamp();

        assert!(ts2 >= ts1);
    }

    #[test]
    fn test_secure_random() {
        let bytes1 = SecureRandom::bytes(32);
        let bytes2 = SecureRandom::bytes(32);

        assert_eq!(bytes1.len(), 32);
        assert_eq!(bytes2.len(), 32);
        assert_ne!(bytes1, bytes2);

        let string1 = SecureRandom::string(20);
        let string2 = SecureRandom::string(20);

        assert_eq!(string1.len(), 20);
        assert_eq!(string2.len(), 20);
        assert_ne!(string1, string2);
    }

    #[test]
    fn test_secure_random_edge_cases() {
        // Test zero-length
        let bytes = SecureRandom::bytes(0);
        assert_eq!(bytes.len(), 0);

        let string = SecureRandom::string(0);
        assert_eq!(string.len(), 0);

        // Test single byte/char
        let bytes = SecureRandom::bytes(1);
        assert_eq!(bytes.len(), 1);

        let string = SecureRandom::string(1);
        assert_eq!(string.len(), 1);
    }

    #[test]
    fn test_validate_api_key_format_edge_cases() {
        // Test boundaries
        assert!(validate_api_key_format("").is_err());
        assert!(validate_api_key_format("a").is_err()); // Too short
        assert!(validate_api_key_format("ab").is_err()); // Too short  

        // Test without proper prefix
        assert!(validate_api_key_format("abc12345678901234567890").is_err());

        // Test with proper prefix but too short
        assert!(validate_api_key_format("mcp_abc").is_err());

        // Test exactly minimum length with prefix
        assert!(validate_api_key_format("mcp_1234567890123456").is_ok());

        // Test with different character types
        assert!(validate_api_key_format("mcp_123456789012345678").is_ok());
        assert!(validate_api_key_format("mcp_ABCDEFGHIJ1234567890").is_ok());
        assert!(validate_api_key_format("mcp_abcdefghij1234567890").is_ok());
        assert!(validate_api_key_format("mcp_a1B2c3D4e1234567890").is_ok());

        // Test whitespace
        assert!(validate_api_key_format("mcp_abc def1234567890").is_err());
        assert!(validate_api_key_format(" mcp_abcdef1234567890").is_err());
        assert!(validate_api_key_format("mcp_abcdef1234567890 ").is_err());
    }

    #[test]
    fn test_hash_and_verify_consistency() {
        let api_key = "test_key_12345";
        let hash1 = hash_api_key(api_key);
        let hash2 = hash_api_key(api_key);

        // Hashes should be the same (deterministic)
        assert_eq!(hash1, hash2);

        // Both should verify correctly
        assert!(verify_api_key(api_key, &hash1));
        assert!(verify_api_key(api_key, &hash2));

        // Wrong key should not verify
        assert!(!verify_api_key("wrong_key", &hash1));
        assert!(!verify_api_key("wrong_key", &hash2));
    }

    #[test]
    fn test_secure_compare_edge_cases() {
        // Test empty strings
        assert!(secure_compare("", ""));
        assert!(!secure_compare("", "a"));
        assert!(!secure_compare("a", ""));

        // Test same content
        assert!(secure_compare("hello", "hello"));

        // Test different lengths
        assert!(!secure_compare("short", "longer_string"));
        assert!(!secure_compare("longer_string", "short"));
    }

    #[test]
    fn test_timestamp_consistency() {
        let time1 = current_timestamp();
        let time2 = current_timestamp();

        // Should be very close in time
        assert!(time2 >= time1);
        assert!(time2 - time1 < 1000); // Less than 1 second difference
    }
}
