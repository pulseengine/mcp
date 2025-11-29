//! PKCE (Proof Key for Code Exchange) Implementation
//!
//! RFC 7636: Proof Key for Code Exchange by OAuth Public Clients
//! OAuth 2.1 mandates PKCE with S256 code challenge method

use sha2::{Digest, Sha256};

/// Verify PKCE code_verifier against code_challenge using S256 method
///
/// OAuth 2.1 mandates S256 (SHA-256) hashing for PKCE.
/// Verification: BASE64URL(SHA256(ASCII(code_verifier))) == code_challenge
///
/// # Arguments
/// * `code_verifier` - The PKCE code verifier from token request
/// * `code_challenge` - The PKCE code challenge from authorization request
///
/// # Returns
/// * `true` if verification succeeds
/// * `false` if verification fails
pub fn verify_pkce(code_verifier: &str, code_challenge: &str) -> bool {
    // Compute SHA-256 hash of the code_verifier
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();

    // Base64-URL encode the hash (without padding)
    let computed_challenge = base64_url::encode(&hash);

    // Compare with provided code_challenge
    computed_challenge == code_challenge
}

/// Validate PKCE code_verifier format
///
/// RFC 7636 Section 4.1: code_verifier must be:
/// - 43-128 characters
/// - Contain only [A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"
pub fn validate_code_verifier(code_verifier: &str) -> bool {
    let len = code_verifier.len();
    if len < 43 || len > 128 {
        return false;
    }

    code_verifier
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
}

/// Validate PKCE code_challenge format
///
/// RFC 7636 Section 4.2: code_challenge must be:
/// - 43-128 characters (Base64-URL encoded SHA-256 = 43 chars)
/// - Contain only [A-Z] / [a-z] / [0-9] / "-" / "_"
pub fn validate_code_challenge(code_challenge: &str) -> bool {
    let len = code_challenge.len();
    if len < 43 || len > 128 {
        return false;
    }

    code_challenge
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_verification_success() {
        // Test vector from RFC 7636 Appendix B
        let code_verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let code_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

        assert!(verify_pkce(code_verifier, code_challenge));
    }

    #[test]
    fn test_pkce_verification_failure() {
        let code_verifier = "wrong_verifier_123456789012345678901234567890";
        let code_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

        assert!(!verify_pkce(code_verifier, code_challenge));
    }

    #[test]
    fn test_code_verifier_validation() {
        // Valid: 43-128 chars, unreserved characters
        assert!(validate_code_verifier(
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        ));

        // Invalid: too short (42 chars)
        assert!(!validate_code_verifier(
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOE"
        ));

        // Invalid: too long (129 chars)
        assert!(!validate_code_verifier(&"a".repeat(129)));

        // Invalid: contains invalid character '='
        assert!(!validate_code_verifier(
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk="
        ));
    }

    #[test]
    fn test_code_challenge_validation() {
        // Valid: Base64-URL encoded SHA-256 (43 chars)
        assert!(validate_code_challenge(
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        ));

        // Invalid: too short
        assert!(!validate_code_challenge("E9Melhoa2OwvFrEMTJguCHaoeK1t8URW"));

        // Invalid: contains '.' (not allowed in base64-url for challenges)
        assert!(!validate_code_challenge(
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw.cM"
        ));
    }
}
