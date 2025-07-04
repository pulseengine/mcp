//! Encryption for API keys at rest
//!
//! This module provides AES-256-GCM encryption for storing API keys
//! securely, inspired by Loxone's RSA/AES encryption approach.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};

/// Encrypted data with nonce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Base64-encoded encrypted data
    pub ciphertext: String,
    /// Base64-encoded nonce (96 bits for AES-GCM)
    pub nonce: String,
    /// Encryption algorithm identifier
    pub algorithm: String,
}

/// Encryption errors
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Invalid data format: {0}")]
    InvalidFormat(String),
}

/// Encrypt data using AES-256-GCM
pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<EncryptedData, EncryptionError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

    Ok(EncryptedData {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce),
        algorithm: "AES-256-GCM".to_string(),
    })
}

/// Decrypt data using AES-256-GCM
pub fn decrypt_data(encrypted: &EncryptedData, key: &[u8; 32]) -> Result<Vec<u8>, EncryptionError> {
    if encrypted.algorithm != "AES-256-GCM" {
        return Err(EncryptionError::InvalidFormat(format!(
            "Unsupported algorithm: {}",
            encrypted.algorithm
        )));
    }

    let ciphertext = BASE64
        .decode(&encrypted.ciphertext)
        .map_err(|e| EncryptionError::InvalidFormat(format!("Invalid ciphertext base64: {e}")))?;

    let nonce_bytes = BASE64
        .decode(&encrypted.nonce)
        .map_err(|e| EncryptionError::InvalidFormat(format!("Invalid nonce base64: {e}")))?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))
}

/// Derive an encryption key from a master key and context
///
/// This uses HKDF (HMAC-based Key Derivation Function) to derive
/// context-specific keys from a master key.
pub fn derive_encryption_key(master_key: &[u8], context: &str) -> [u8; 32] {
    use hkdf::Hkdf;
    use sha2::Sha256;

    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    let info = format!("pulseengine-mcp-auth-{context}");
    hkdf.expand(info.as_bytes(), &mut okm)
        .expect("32 bytes is a valid length for HKDF-SHA256");

    okm
}

/// Generate a random encryption key
pub fn generate_encryption_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Zero out sensitive data in memory
pub fn secure_zero(data: &mut [u8]) {
    use zeroize::Zeroize;
    data.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let key = generate_encryption_key();
        let plaintext = b"sensitive-api-key-data";

        // Encrypt
        let encrypted = encrypt_data(plaintext, &key).unwrap();
        assert!(!encrypted.ciphertext.is_empty());
        assert!(!encrypted.nonce.is_empty());
        assert_eq!(encrypted.algorithm, "AES-256-GCM");

        // Decrypt
        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_with_wrong_key() {
        let key1 = generate_encryption_key();
        let key2 = generate_encryption_key();
        let plaintext = b"sensitive-api-key-data";

        // Encrypt with key1
        let encrypted = encrypt_data(plaintext, &key1).unwrap();

        // Try to decrypt with key2 - should fail
        let result = decrypt_data(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_derivation() {
        let master_key = b"master-key-material";

        let key1 = derive_encryption_key(master_key, "api-keys");
        let key2 = derive_encryption_key(master_key, "api-keys");
        let key3 = derive_encryption_key(master_key, "audit-logs");

        // Same context should produce same key
        assert_eq!(key1, key2);

        // Different context should produce different key
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_secure_zero() {
        let mut sensitive_data = b"sensitive-key".to_vec();
        let original = sensitive_data.clone();

        secure_zero(&mut sensitive_data);

        // Data should be zeroed
        assert_ne!(sensitive_data, original);
        assert!(sensitive_data.iter().all(|&b| b == 0));
    }
}
