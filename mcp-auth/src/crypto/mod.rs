//! Cryptographic utilities for secure authentication
//!
//! This module provides encryption, hashing, and key derivation functions
//! for secure API key management, inspired by Loxone MCP's security model.

pub mod encryption;
pub mod hashing;
pub mod keys;

pub use encryption::{decrypt_data, encrypt_data, EncryptionError};
pub use hashing::{generate_salt, hash_api_key, verify_api_key, HashingError};
pub use keys::{derive_key, generate_secure_key, KeyDerivationError};

pub use encryption::EncryptedData;
/// Re-export common types
pub use hashing::Salt;

/// Initialize the crypto module (perform any necessary setup)
pub fn init() -> Result<(), CryptoError> {
    // Ensure we have good randomness available
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut test_bytes = [0u8; 32];
    rng.fill_bytes(&mut test_bytes);

    // Verify we got non-zero random bytes
    if test_bytes.iter().all(|&b| b == 0) {
        return Err(CryptoError::RandomnessError(
            "Failed to generate random bytes".into(),
        ));
    }

    Ok(())
}

/// General crypto error type
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),

    #[error("Hashing error: {0}")]
    Hashing(#[from] HashingError),

    #[error("Key derivation error: {0}")]
    KeyDerivation(#[from] KeyDerivationError),

    #[error("Randomness error: {0}")]
    RandomnessError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_init() {
        assert!(init().is_ok());
    }
}
