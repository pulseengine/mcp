//! OAuth 2.1 Token Endpoint
//!
//! Handles token exchange for authorization codes and refresh tokens

use crate::oauth::models::{AccessTokenClaims, OAuthError, TokenRequest, TokenResponse};
use crate::oauth::pkce::verify_pkce;
use axum::{http::StatusCode, response::IntoResponse, Form, Json};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::Rng;

/// POST /oauth/token - Exchange authorization code or refresh token for access token
///
/// Supports two grant types:
/// 1. `authorization_code` - Exchange auth code for tokens (with PKCE verification)
/// 2. `refresh_token` - Exchange refresh token for new access token
///
/// # Authorization Code Grant
/// ```json
/// {
///   "grant_type": "authorization_code",
///   "code": "authorization_code_here",
///   "redirect_uri": "https://example.com/callback",
///   "code_verifier": "pkce_verifier_here",
///   "client_id": "client_id",
///   "client_secret": "client_secret"
/// }
/// ```
///
/// # Refresh Token Grant
/// ```json
/// {
///   "grant_type": "refresh_token",
///   "refresh_token": "refresh_token_here",
///   "client_id": "client_id",
///   "client_secret": "client_secret"
/// }
/// ```
///
/// # Response
/// ```json
/// {
///   "access_token": "jwt_token_here",
///   "token_type": "Bearer",
///   "expires_in": 3600,
///   "refresh_token": "new_refresh_token",
///   "scope": "mcp:read mcp:write"
/// }
/// ```
pub async fn token_endpoint(
    Form(request): Form<TokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // TODO: Verify client credentials (client_id + client_secret)
    // TODO: Load client from database and verify hashed secret

    match request.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_grant(request).await,
        "refresh_token" => handle_refresh_token_grant(request).await,
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::unsupported_grant_type(format!(
                "grant_type '{}' not supported",
                request.grant_type
            ))),
        )),
    }
}

/// Handle authorization_code grant type
async fn handle_authorization_code_grant(
    request: TokenRequest,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // Validate required parameters
    let code = request.code.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request("code is required")),
        )
    })?;

    let redirect_uri = request.redirect_uri.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request("redirect_uri is required")),
        )
    })?;

    let code_verifier = request.code_verifier.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request("code_verifier is required")),
        )
    })?;

    // TODO: Load authorization code from database
    // For now, we'll simulate the stored code_challenge
    let stored_code_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"; // Example from RFC 7636

    // Verify PKCE code_verifier against stored code_challenge
    if !verify_pkce(&code_verifier, stored_code_challenge) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_grant("PKCE verification failed")),
        ));
    }

    // TODO: Verify authorization code:
    // - Code exists in database
    // - Not expired (10 minutes max)
    // - redirect_uri matches
    // - client_id matches
    // - Mark code as used (single use only)

    // TODO: Load scopes and resource from stored authorization code
    let scopes = vec!["mcp:read".to_string(), "mcp:write".to_string()];
    let resource = request.resource.clone();

    // Generate tokens
    let access_token = generate_access_token(&request.client_id, &scopes, resource.as_deref())?;
    let refresh_token = generate_refresh_token();

    // TODO: Store refresh token in database with:
    // - token (hashed)
    // - client_id
    // - resource
    // - scopes
    // - expires_at (30 days from now)

    let response = TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600, // 1 hour
        refresh_token: Some(refresh_token),
        scope: Some(scopes.join(" ")),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Handle refresh_token grant type
async fn handle_refresh_token_grant(
    request: TokenRequest,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // Validate required parameters
    let refresh_token = request.refresh_token.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request("refresh_token is required")),
        )
    })?;

    // TODO: Load refresh token from database
    // TODO: Verify:
    // - Token exists and matches client_id
    // - Not expired
    // - Delete old refresh token (rotation)

    // TODO: Load scopes and resource from stored refresh token
    let scopes = vec!["mcp:read".to_string(), "mcp:write".to_string()];
    let resource = request.resource.clone();

    // Generate new tokens (refresh token rotation per OAuth 2.1)
    let access_token = generate_access_token(&request.client_id, &scopes, resource.as_deref())?;
    let new_refresh_token = generate_refresh_token();

    // TODO: Store new refresh token in database
    // TODO: Delete old refresh token

    let response = TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600, // 1 hour
        refresh_token: Some(new_refresh_token),
        scope: Some(scopes.join(" ")),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Generate JWT access token
fn generate_access_token(
    client_id: &str,
    scopes: &[String],
    resource: Option<&str>,
) -> Result<String, (StatusCode, Json<OAuthError>)> {
    // TODO: Load JWT signing key from secure storage
    // For now, use a placeholder key (MUST be replaced with actual secret)
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        "REPLACE_THIS_WITH_SECURE_SECRET_FROM_ENV_OR_VAULT".to_string()
    });

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let now = Utc::now().timestamp();
    let claims = AccessTokenClaims {
        sub: client_id.to_string(),
        aud: resource.map(|r| r.to_string()),
        exp: now + 3600, // 1 hour from now
        iat: now,
        iss: base_url,
        scope: scopes.join(" "),
        client_id: client_id.to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OAuthError::invalid_request(format!(
                "Failed to generate token: {}",
                e
            ))),
        )
    })
}

/// Generate cryptographically secure refresh token
fn generate_refresh_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut rng = rand::thread_rng();

    (0..64)
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
    fn test_generate_refresh_token_length() {
        let token = generate_refresh_token();
        assert_eq!(token.len(), 64);
    }

    #[test]
    fn test_generate_refresh_token_charset() {
        let token = generate_refresh_token();
        for c in token.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_');
        }
    }
}
