//! RFC 7591: Dynamic Client Registration
//!
//! OAuth 2.1 dynamic client registration endpoint

use crate::oauth::{
    OAuthState,
    models::{ClientRegistrationRequest, ClientRegistrationResponse, OAuthClient, OAuthError},
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use rand::Rng;

/// RFC 7591: Dynamic Client Registration
///
/// Endpoint: POST /oauth/register
///
/// # Security Requirements (OAuth 2.1)
/// - Redirect URIs must be HTTPS or http://localhost (for development)
/// - Generate cryptographically secure client_id and client_secret
/// - Client secrets should be hashed before storage (TODO: bcrypt/argon2)
///
/// # Request
/// ```json
/// {
///   "client_name": "My MCP Client",
///   "redirect_uris": ["https://example.com/callback"],
///   "grant_types": ["authorization_code", "refresh_token"],
///   "response_types": ["code"]
/// }
/// ```
///
/// # Response
/// ```json
/// {
///   "client_id": "generated_client_id",
///   "client_secret": "generated_client_secret",
///   "client_name": "My MCP Client",
///   "redirect_uris": ["https://example.com/callback"],
///   "client_secret_expires_at": 0,
///   "grant_types": ["authorization_code", "refresh_token"],
///   "response_types": ["code"]
/// }
/// ```
pub async fn register_client(
    State(state): State<OAuthState>,
    Json(request): Json<ClientRegistrationRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // Validate redirect URIs (OAuth 2.1 Security BCP)
    if request.redirect_uris.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request(
                "At least one redirect_uri is required",
            )),
        ));
    }

    for uri in &request.redirect_uris {
        if !is_valid_redirect_uri(uri) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(OAuthError::invalid_request(format!(
                    "Invalid redirect_uri: {}. Must be HTTPS or http://localhost",
                    uri
                ))),
            ));
        }
    }

    // Validate grant_types
    let grant_types = if request.grant_types.is_empty() {
        vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ]
    } else {
        request.grant_types.clone()
    };

    for grant_type in &grant_types {
        if grant_type != "authorization_code" && grant_type != "refresh_token" {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(OAuthError::invalid_request(format!(
                    "Unsupported grant_type: {}. Only authorization_code and refresh_token are supported",
                    grant_type
                ))),
            ));
        }
    }

    // Validate response_types
    let response_types = if request.response_types.is_empty() {
        vec!["code".to_string()]
    } else {
        request.response_types.clone()
    };

    for response_type in &response_types {
        if response_type != "code" {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(OAuthError::invalid_request(format!(
                    "Unsupported response_type: {}. Only 'code' is supported",
                    response_type
                ))),
            ));
        }
    }

    // Generate client credentials
    let client_id = generate_token(32);
    let client_secret = generate_token(64);

    let client_name = request
        .client_name
        .unwrap_or_else(|| "Unnamed Client".to_string());

    // Create OAuth client
    let client = OAuthClient {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(), // TODO: Hash with bcrypt/argon2 in production
        client_name: client_name.clone(),
        redirect_uris: request.redirect_uris.clone(),
        created_at: Utc::now(),
        client_secret_expires_at: None,
    };

    // Save to storage
    state.storage.save_client(&client).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OAuthError::invalid_request(format!(
                "Failed to save client: {}",
                e
            ))),
        )
    })?;

    let response = ClientRegistrationResponse {
        client_id,
        client_secret,
        client_name,
        redirect_uris: request.redirect_uris,
        client_secret_expires_at: 0, // Never expires (0 per RFC 7591)
        grant_types,
        response_types,
    };

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// Validate redirect URI per OAuth 2.1 Security BCP
///
/// Requirements:
/// - HTTPS for production URIs
/// - http://localhost or http://127.0.0.1 allowed for development
/// - No custom schemes (security risk in OAuth 2.1)
fn is_valid_redirect_uri(uri: &str) -> bool {
    if uri.starts_with("https://") {
        return true;
    }

    // Allow localhost for development
    if uri.starts_with("http://localhost") || uri.starts_with("http://127.0.0.1") {
        return true;
    }

    false
}

/// Generate cryptographically secure random token
///
/// Uses OS random number generator via `rand::thread_rng()`
fn generate_token(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut rng = rand::thread_rng();

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_redirect_uris() {
        assert!(is_valid_redirect_uri("https://example.com/callback"));
        assert!(is_valid_redirect_uri("http://localhost:3000/callback"));
        assert!(is_valid_redirect_uri("http://127.0.0.1:8080/callback"));
    }

    #[test]
    fn test_invalid_redirect_uris() {
        assert!(!is_valid_redirect_uri("http://example.com/callback"));
        assert!(!is_valid_redirect_uri("custom-scheme://callback"));
        assert!(!is_valid_redirect_uri("ftp://example.com"));
    }

    #[test]
    fn test_generate_token_length() {
        let token = generate_token(32);
        assert_eq!(token.len(), 32);

        let token = generate_token(64);
        assert_eq!(token.len(), 64);
    }

    #[test]
    fn test_generate_token_charset() {
        let token = generate_token(100);
        for c in token.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_');
        }
    }
}
