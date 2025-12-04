//! Full OAuth 2.1 flow integration tests
//!
//! Tests complete authorization code flow with PKCE

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use pulseengine_mcp_auth::oauth::{
    OAuthState,
    models::{AuthorizationCode, RefreshToken},
    oauth_router,
};
use rand::Rng;
use sha2::{Digest, Sha256};
use tower::util::ServiceExt;

/// Helper to create test app
fn test_app() -> (Router, OAuthState) {
    let state = OAuthState::new_in_memory();
    let app = oauth_router().with_state(state.clone());
    (app, state)
}

/// Generate PKCE code_verifier (43-128 chars of unreserved chars)
fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let len = 43; // Minimum length for PKCE verifier
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}

/// Generate PKCE code_challenge from verifier (S256 method)
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    base64_url::encode(&hash)
}

/// Helper for GET requests
async fn get_request(app: Router, uri: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .uri(uri)
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).to_string())
}

/// Helper for POST form requests to authorize endpoint - returns (status, location_header)
async fn post_form(app: Router, uri: &str, params: &[(&str, &str)]) -> (StatusCode, String) {
    let body = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let request = Request::builder()
        .uri(uri)
        .method("POST")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    // Extract Location header for redirects
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    (status, location)
}

/// Helper for POST form requests to token endpoint - returns (status, json_body)
async fn post_form_json(
    app: Router,
    uri: &str,
    params: &[(&str, &str)],
) -> (StatusCode, serde_json::Value) {
    let body = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let request = Request::builder()
        .uri(uri)
        .method("POST")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&body_bytes).unwrap_or(serde_json::json!({}));

    (status, json)
}

// ============================================================================
// Authorization Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_authorize_get_displays_consent_form() {
    let (app, state) = test_app();

    // First register a client
    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    let uri = format!(
        "/oauth/authorize?\
         response_type=code&\
         client_id=test_client&\
         redirect_uri={}&\
         state=xyz&\
         code_challenge={}&\
         code_challenge_method=S256&\
         scope={}",
        urlencoding::encode("https://example.com/callback"),
        urlencoding::encode(&code_challenge),
        urlencoding::encode("mcp:read mcp:write")
    );

    let (status, body) = get_request(app, &uri).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Authorization Request"));
    assert!(body.contains("test_client"));
    assert!(body.contains("https://example.com/callback"));
    assert!(body.contains("mcp:read"));
    assert!(body.contains("mcp:write"));
}

#[tokio::test]
async fn test_authorize_get_invalid_response_type() {
    let (app, state) = test_app();

    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    let uri = format!(
        "/oauth/authorize?\
         response_type=token&\
         client_id=test_client&\
         redirect_uri=https://example.com/callback&\
         code_challenge={}&\
         code_challenge_method=S256",
        urlencoding::encode(&code_challenge)
    );

    let (status, body) = get_request(app, &uri).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("response_type"));
}

#[tokio::test]
async fn test_authorize_get_invalid_code_challenge_method() {
    let (app, state) = test_app();

    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    let uri = "/oauth/authorize?\
         response_type=code&\
         client_id=test_client&\
         redirect_uri=https://example.com/callback&\
         code_challenge=test&\
         code_challenge_method=plain";

    let (status, body) = get_request(app, uri).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("S256"));
}

#[tokio::test]
async fn test_authorize_post_user_approval() {
    let (app, state) = test_app();

    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    let params = [
        ("client_id", "test_client"),
        ("redirect_uri", "https://example.com/callback"),
        ("code_challenge", &code_challenge),
        ("state", "xyz"),
        ("scope", "mcp:read mcp:write"),
        ("approved", "true"),
    ];

    let (status, location) = post_form(app, "/oauth/authorize", &params).await;

    // Should redirect to callback with code
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert!(location.contains("https://example.com/callback"));
    assert!(location.contains("code="));
    assert!(location.contains("state=xyz"));
}

#[tokio::test]
async fn test_authorize_post_user_denial() {
    let (app, state) = test_app();

    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    let params = [
        ("client_id", "test_client"),
        ("redirect_uri", "https://example.com/callback"),
        ("code_challenge", &code_challenge),
        ("state", "xyz"),
        ("approved", "false"),
    ];

    let (status, location) = post_form(app, "/oauth/authorize", &params).await;

    // Should redirect with error
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert!(location.contains("error=access_denied"));
    assert!(location.contains("state=xyz"));
}

// ============================================================================
// Token Endpoint Full Flow Tests
// ============================================================================

#[tokio::test]
async fn test_full_authorization_code_flow() {
    let (_, state) = test_app();

    // 1. Register client
    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    // 2. Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // 3. Create authorization code (simulating user approval)
    let auth_code = AuthorizationCode {
        code: "test_auth_code_123".to_string(),
        client_id: "test_client".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge: code_challenge.clone(),
        resource: None,
        scopes: vec!["mcp:read".to_string(), "mcp:write".to_string()],
        expires_at: Utc::now() + Duration::minutes(10),
        created_at: Utc::now(),
    };
    state
        .storage
        .save_authorization_code(&auth_code)
        .await
        .unwrap();

    // 4. Exchange code for tokens
    let app = oauth_router().with_state(state.clone());
    let params = [
        ("grant_type", "authorization_code"),
        ("code", "test_auth_code_123"),
        ("redirect_uri", "https://example.com/callback"),
        ("code_verifier", &code_verifier),
        ("client_id", "test_client"),
        ("client_secret", "test_secret"),
    ];

    let (status, json) = post_form_json(app, "/oauth/token", &params).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["access_token"].is_string());
    assert_eq!(json["token_type"], "Bearer");
    assert_eq!(json["expires_in"], 3600);
    assert!(json["refresh_token"].is_string());
    assert_eq!(json["scope"], "mcp:read mcp:write");
}

#[tokio::test]
async fn test_refresh_token_flow() {
    let (_, state) = test_app();

    // 1. Register client
    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    // 2. Create a refresh token
    let refresh_token = RefreshToken {
        token: "test_refresh_token_123".to_string(),
        client_id: "test_client".to_string(),
        resource: None,
        scopes: vec!["mcp:read".to_string()],
        expires_at: Utc::now() + Duration::days(30),
        created_at: Utc::now(),
    };
    state
        .storage
        .save_refresh_token(&refresh_token)
        .await
        .unwrap();

    // 3. Use refresh token to get new access token
    let app = oauth_router().with_state(state);
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", "test_refresh_token_123"),
        ("client_id", "test_client"),
        ("client_secret", "test_secret"),
    ];

    let (status, json) = post_form_json(app, "/oauth/token", &params).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["access_token"].is_string());
    assert_eq!(json["token_type"], "Bearer");
    assert!(json["refresh_token"].is_string());
    // Old refresh token should be different from new one (rotation)
    assert_ne!(json["refresh_token"], "test_refresh_token_123");
}

#[tokio::test]
async fn test_token_endpoint_wrong_code_verifier() {
    let (_, state) = test_app();

    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    let auth_code = AuthorizationCode {
        code: "test_code".to_string(),
        client_id: "test_client".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge,
        resource: None,
        scopes: vec![],
        expires_at: Utc::now() + Duration::minutes(10),
        created_at: Utc::now(),
    };
    state
        .storage
        .save_authorization_code(&auth_code)
        .await
        .unwrap();

    // Try with wrong verifier
    let wrong_verifier = generate_code_verifier();
    let app = oauth_router().with_state(state);
    let params = [
        ("grant_type", "authorization_code"),
        ("code", "test_code"),
        ("redirect_uri", "https://example.com/callback"),
        ("code_verifier", &wrong_verifier),
        ("client_id", "test_client"),
        ("client_secret", "test_secret"),
    ];

    let (status, json) = post_form_json(app, "/oauth/token", &params).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        json["error"]
            .as_str()
            .unwrap_or("")
            .contains("invalid_grant")
            || json["error_description"]
                .as_str()
                .unwrap_or("")
                .contains("PKCE")
    );
}

#[tokio::test]
async fn test_token_endpoint_expired_code() {
    let (_, state) = test_app();

    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Create expired code
    let auth_code = AuthorizationCode {
        code: "expired_code".to_string(),
        client_id: "test_client".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge,
        resource: None,
        scopes: vec![],
        expires_at: Utc::now() - Duration::seconds(1),
        created_at: Utc::now() - Duration::minutes(15),
    };
    state
        .storage
        .save_authorization_code(&auth_code)
        .await
        .unwrap();

    let app = oauth_router().with_state(state);
    let params = [
        ("grant_type", "authorization_code"),
        ("code", "expired_code"),
        ("redirect_uri", "https://example.com/callback"),
        ("code_verifier", &code_verifier),
        ("client_id", "test_client"),
        ("client_secret", "test_secret"),
    ];

    let (status, json) = post_form_json(app, "/oauth/token", &params).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        json["error"]
            .as_str()
            .unwrap_or("")
            .contains("invalid_grant")
            || json["error_description"]
                .as_str()
                .unwrap_or("")
                .contains("expired")
    );
}

#[tokio::test]
async fn test_token_endpoint_wrong_redirect_uri() {
    let (_, state) = test_app();

    let client = pulseengine_mcp_auth::oauth::models::OAuthClient {
        client_id: "test_client".to_string(),
        client_secret: "test_secret".to_string(),
        client_name: "Test App".to_string(),
        redirect_uris: vec!["https://example.com/callback".to_string()],
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };
    state.storage.save_client(&client).await.unwrap();

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    let auth_code = AuthorizationCode {
        code: "test_code".to_string(),
        client_id: "test_client".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        code_challenge,
        resource: None,
        scopes: vec![],
        expires_at: Utc::now() + Duration::minutes(10),
        created_at: Utc::now(),
    };
    state
        .storage
        .save_authorization_code(&auth_code)
        .await
        .unwrap();

    let app = oauth_router().with_state(state);
    let params = [
        ("grant_type", "authorization_code"),
        ("code", "test_code"),
        ("redirect_uri", "https://evil.com/callback"),
        ("code_verifier", &code_verifier),
        ("client_id", "test_client"),
        ("client_secret", "test_secret"),
    ];

    let (status, json) = post_form_json(app, "/oauth/token", &params).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        json["error"]
            .as_str()
            .unwrap_or("")
            .contains("invalid_grant")
            || json["error_description"]
                .as_str()
                .unwrap_or("")
                .contains("redirect_uri")
    );
}
