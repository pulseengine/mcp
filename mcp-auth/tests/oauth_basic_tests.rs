//! Basic unit tests for OAuth 2.1 components

use chrono::{Duration, Utc};
use pulseengine_mcp_auth::oauth::{
    models::{AuthorizationCode, OAuthClient, OAuthError, RefreshToken},
    pkce::{validate_code_challenge, validate_code_verifier, verify_pkce},
    storage::{InMemoryOAuthStorage, OAuthStorage, OAuthStorageError},
};

// ============================================================================
// PKCE Tests
// ============================================================================

#[test]
fn test_verify_pkce() {
    // Test vector from RFC 7636 Appendix B
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    assert!(verify_pkce(verifier, challenge));

    // Wrong verifier should fail
    let wrong_verifier = "wrong_verifier_123456789012345678901234567890";
    assert!(!verify_pkce(wrong_verifier, challenge));
}

#[test]
fn test_validate_code_verifier() {
    // Valid verifier
    assert!(validate_code_verifier(
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    ));

    // Too short
    assert!(!validate_code_verifier("short"));

    // Too long
    let long_str = "a".repeat(129);
    assert!(!validate_code_verifier(&long_str));

    // Invalid characters
    assert!(!validate_code_verifier(
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk="
    ));
}

#[test]
fn test_validate_code_challenge() {
    // Valid S256 challenge
    assert!(validate_code_challenge(
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    ));

    // Too short
    assert!(!validate_code_challenge("short"));

    // Too long (>128 chars)
    let long_str = "a".repeat(129);
    assert!(!validate_code_challenge(&long_str));

    // Invalid character '.'
    assert!(!validate_code_challenge(
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw.cM"
    ));
}

// ============================================================================
// OAuth Error Tests
// ============================================================================

#[test]
fn test_oauth_error_creation() {
    let err = OAuthError::invalid_request("Test message");
    assert_eq!(err.error, "invalid_request");
    assert_eq!(err.error_description.unwrap(), "Test message");

    let err2 = OAuthError::unauthorized_client("Client not registered");
    assert_eq!(err2.error, "unauthorized_client");

    let err3 = OAuthError::invalid_grant("Code expired");
    assert_eq!(err3.error, "invalid_grant");

    let err4 = OAuthError::invalid_client("Bad client");
    assert_eq!(err4.error, "invalid_client");

    let err5 = OAuthError::unsupported_grant_type("Unsupported");
    assert_eq!(err5.error, "unsupported_grant_type");
}

// ============================================================================
// Storage Tests - Client Operations
// ============================================================================

#[tokio::test]
async fn test_storage_save_and_get_client() {
    let storage = InMemoryOAuthStorage::new();

    let client = OAuthClient {
        client_id: "test_client_123".to_string(),
        client_secret: "secret_abc".to_string(),
        client_name: "Test Client".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };

    // Save client
    storage.save_client(&client).await.unwrap();

    // Retrieve client
    let retrieved = storage.get_client("test_client_123").await.unwrap();
    assert_eq!(retrieved.client_id, "test_client_123");
    assert_eq!(retrieved.client_name, "Test Client");
    assert_eq!(retrieved.redirect_uris.len(), 1);
}

#[tokio::test]
async fn test_storage_get_nonexistent_client() {
    let storage = InMemoryOAuthStorage::new();

    let result = storage.get_client("non_existent").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OAuthStorageError::ClientNotFound(_) => {}
        _ => panic!("Expected ClientNotFound error"),
    }
}

#[tokio::test]
async fn test_storage_verify_client_secret() {
    let storage = InMemoryOAuthStorage::new();

    let client = OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "correct_secret".to_string(),
        client_name: "Test".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };

    storage.save_client(&client).await.unwrap();

    // Correct secret
    let result = storage
        .verify_client_secret("test_client", "correct_secret")
        .await
        .unwrap();
    assert!(result);

    // Incorrect secret
    let result = storage
        .verify_client_secret("test_client", "wrong_secret")
        .await
        .unwrap();
    assert!(!result);
}

// ============================================================================
// Storage Tests - Authorization Code Operations
// ============================================================================

#[tokio::test]
async fn test_storage_authorization_code_lifecycle() {
    let storage = InMemoryOAuthStorage::new();

    let code = AuthorizationCode {
        code: "auth_code_123".to_string(),
        client_id: "test_client".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
        resource: None,
        scopes: vec!["read".to_string(), "write".to_string()],
        expires_at: Utc::now() + Duration::minutes(10),
        created_at: Utc::now(),
    };

    // Save authorization code
    storage.save_authorization_code(&code).await.unwrap();

    // Retrieve authorization code
    let retrieved = storage
        .get_authorization_code("auth_code_123")
        .await
        .unwrap();
    assert_eq!(retrieved.code, "auth_code_123");
    assert_eq!(retrieved.client_id, "test_client");
    assert_eq!(retrieved.scopes.len(), 2);

    // Delete authorization code
    storage
        .delete_authorization_code("auth_code_123")
        .await
        .unwrap();

    // Verify deletion
    let result = storage.get_authorization_code("auth_code_123").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_storage_authorization_code_expiration() {
    let storage = InMemoryOAuthStorage::new();

    // Create expired authorization code
    let expired_code = AuthorizationCode {
        code: "expired_code".to_string(),
        client_id: "test_client".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge: "challenge".to_string(),
        resource: None,
        scopes: vec![],
        expires_at: Utc::now() - Duration::seconds(1),
        created_at: Utc::now() - Duration::minutes(1),
    };

    storage
        .save_authorization_code(&expired_code)
        .await
        .unwrap();

    // Trying to retrieve should fail due to expiration
    let result = storage.get_authorization_code("expired_code").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OAuthStorageError::CodeExpired => {}
        _ => panic!("Expected CodeExpired error"),
    }
}

// ============================================================================
// Storage Tests - Refresh Token Operations
// ============================================================================

#[tokio::test]
async fn test_storage_refresh_token_lifecycle() {
    let storage = InMemoryOAuthStorage::new();

    let token = RefreshToken {
        token: "refresh_123".to_string(),
        client_id: "test_client".to_string(),
        resource: None,
        scopes: vec!["read".to_string(), "write".to_string()],
        expires_at: Utc::now() + Duration::days(30),
        created_at: Utc::now(),
    };

    // Save refresh token
    storage.save_refresh_token(&token).await.unwrap();

    // Get refresh token
    let retrieved = storage.get_refresh_token("refresh_123").await.unwrap();
    assert_eq!(retrieved.token, "refresh_123");
    assert_eq!(retrieved.client_id, "test_client");
    assert_eq!(retrieved.scopes.len(), 2);

    // Delete refresh token
    storage.delete_refresh_token("refresh_123").await.unwrap();

    // Verify deletion
    let result = storage.get_refresh_token("refresh_123").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_storage_refresh_token_expiration() {
    let storage = InMemoryOAuthStorage::new();

    let expired_token = RefreshToken {
        token: "expired_refresh".to_string(),
        client_id: "test_client".to_string(),
        resource: None,
        scopes: vec![],
        expires_at: Utc::now() - Duration::seconds(1),
        created_at: Utc::now() - Duration::days(1),
    };

    storage.save_refresh_token(&expired_token).await.unwrap();

    // Trying to retrieve should fail due to expiration
    let result = storage.get_refresh_token("expired_refresh").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OAuthStorageError::TokenExpired => {}
        _ => panic!("Expected TokenExpired error"),
    }
}

// ============================================================================
// Storage Tests - Cleanup
// ============================================================================

#[tokio::test]
async fn test_storage_cleanup_expired() {
    let storage = InMemoryOAuthStorage::new();

    // Add expired authorization code
    let expired_code = AuthorizationCode {
        code: "expired".to_string(),
        client_id: "test".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge: "challenge".to_string(),
        resource: None,
        scopes: vec![],
        expires_at: Utc::now() - Duration::seconds(1),
        created_at: Utc::now() - Duration::minutes(1),
    };
    storage
        .save_authorization_code(&expired_code)
        .await
        .unwrap();

    // Add expired refresh token
    let expired_token = RefreshToken {
        token: "expired_refresh".to_string(),
        client_id: "test".to_string(),
        resource: None,
        scopes: vec![],
        expires_at: Utc::now() - Duration::seconds(1),
        created_at: Utc::now() - Duration::days(1),
    };
    storage.save_refresh_token(&expired_token).await.unwrap();

    // Cleanup expired entries
    storage.cleanup_expired().await.unwrap();

    // Verify expired entries are gone
    assert!(storage.get_authorization_code("expired").await.is_err());
    assert!(storage.get_refresh_token("expired_refresh").await.is_err());
}

// ============================================================================
// Model Tests
// ============================================================================

#[test]
fn test_oauth_client_model() {
    let client = OAuthClient {
        client_id: "test_123".to_string(),
        client_secret: "secret_abc".to_string(),
        client_name: "Test Client".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };

    assert!(!client.redirect_uris.is_empty());
    assert!(!client.client_secret.is_empty());
}

#[test]
fn test_authorization_code_model() {
    let code = AuthorizationCode {
        code: "test_code".to_string(),
        client_id: "client_123".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge: "challenge_123".to_string(),
        resource: Some("https://api.example.com".to_string()),
        scopes: vec!["read".to_string(), "write".to_string()],
        expires_at: Utc::now() + Duration::minutes(10),
        created_at: Utc::now(),
    };

    assert_eq!(code.scopes.len(), 2);
    assert!(code.resource.is_some());
    assert!(code.expires_at > Utc::now());
}

#[test]
fn test_refresh_token_model() {
    let token = RefreshToken {
        token: "refresh_123".to_string(),
        client_id: "client_123".to_string(),
        resource: None,
        scopes: vec!["read".to_string()],
        expires_at: Utc::now() + Duration::days(30),
        created_at: Utc::now(),
    };

    assert!(!token.token.is_empty());
    assert_eq!(token.scopes.len(), 1);
    assert!(token.expires_at > Utc::now());
}
