//! OAuth 2.1 Token Endpoint
//!
//! Handles token exchange for authorization codes and refresh tokens

use crate::oauth::OAuthState;
use crate::oauth::models::{
    AccessTokenClaims, OAuthError, RefreshToken, TokenRequest, TokenResponse,
};
use crate::oauth::pkce::verify_pkce;
use axum::{Form, Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
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
    State(state): State<OAuthState>,
    Form(request): Form<TokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<OAuthError>)> {
    // Verify client credentials (client_id + client_secret)
    let is_valid = state
        .storage
        .verify_client_secret(&request.client_id, &request.client_secret)
        .await
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(OAuthError::invalid_client("Invalid client credentials")),
            )
        })?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(OAuthError::invalid_client("Invalid client credentials")),
        ));
    }

    match request.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_grant(state, request).await,
        "refresh_token" => handle_refresh_token_grant(state, request).await,
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
    state: OAuthState,
    request: TokenRequest,
) -> Result<(StatusCode, Json<TokenResponse>), (StatusCode, Json<OAuthError>)> {
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

    // Load authorization code from storage
    let auth_code = state
        .storage
        .get_authorization_code(&code)
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(OAuthError::invalid_grant(
                    "Invalid or expired authorization code",
                )),
            )
        })?;

    // Verify authorization code parameters
    if auth_code.client_id != request.client_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_grant("client_id mismatch")),
        ));
    }

    if auth_code.redirect_uri != redirect_uri {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_grant("redirect_uri mismatch")),
        ));
    }

    // Verify PKCE code_verifier against stored code_challenge
    if !verify_pkce(&code_verifier, &auth_code.code_challenge) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_grant("PKCE verification failed")),
        ));
    }

    // Delete authorization code (single use only per OAuth 2.1)
    state
        .storage
        .delete_authorization_code(&code)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OAuthError::invalid_request(format!(
                    "Failed to delete authorization code: {}",
                    e
                ))),
            )
        })?;

    // Generate tokens
    let access_token = generate_access_token(
        &request.client_id,
        &auth_code.scopes,
        auth_code.resource.as_deref(),
    )?;
    let refresh_token_value = generate_refresh_token();

    // Store refresh token in storage
    let refresh_token = RefreshToken {
        token: refresh_token_value.clone(),
        client_id: request.client_id.clone(),
        resource: auth_code.resource.clone(),
        scopes: auth_code.scopes.clone(),
        expires_at: Utc::now() + Duration::days(30), // 30 days expiration
        created_at: Utc::now(),
    };

    state
        .storage
        .save_refresh_token(&refresh_token)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OAuthError::invalid_request(format!(
                    "Failed to save refresh token: {}",
                    e
                ))),
            )
        })?;

    let response = TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600, // 1 hour
        refresh_token: Some(refresh_token_value),
        scope: Some(auth_code.scopes.join(" ")),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Handle refresh_token grant type
async fn handle_refresh_token_grant(
    state: OAuthState,
    request: TokenRequest,
) -> Result<(StatusCode, Json<TokenResponse>), (StatusCode, Json<OAuthError>)> {
    // Validate required parameters
    let refresh_token_value = request.refresh_token.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_request("refresh_token is required")),
        )
    })?;

    // Load refresh token from storage
    let old_refresh_token = state
        .storage
        .get_refresh_token(&refresh_token_value)
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(OAuthError::invalid_grant(
                    "Invalid or expired refresh token",
                )),
            )
        })?;

    // Verify refresh token matches client_id
    if old_refresh_token.client_id != request.client_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(OAuthError::invalid_grant("client_id mismatch")),
        ));
    }

    // Generate new tokens (refresh token rotation per OAuth 2.1)
    let access_token = generate_access_token(
        &request.client_id,
        &old_refresh_token.scopes,
        old_refresh_token.resource.as_deref(),
    )?;
    let new_refresh_token_value = generate_refresh_token();

    // Store new refresh token in storage
    let new_refresh_token = RefreshToken {
        token: new_refresh_token_value.clone(),
        client_id: request.client_id.clone(),
        resource: old_refresh_token.resource.clone(),
        scopes: old_refresh_token.scopes.clone(),
        expires_at: Utc::now() + Duration::days(30), // 30 days expiration
        created_at: Utc::now(),
    };

    state
        .storage
        .save_refresh_token(&new_refresh_token)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OAuthError::invalid_request(format!(
                    "Failed to save refresh token: {}",
                    e
                ))),
            )
        })?;

    // Delete old refresh token (rotation per OAuth 2.1)
    state
        .storage
        .delete_refresh_token(&refresh_token_value)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(OAuthError::invalid_request(format!(
                    "Failed to delete old refresh token: {}",
                    e
                ))),
            )
        })?;

    let response = TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600, // 1 hour
        refresh_token: Some(new_refresh_token_value),
        scope: Some(old_refresh_token.scopes.join(" ")),
    };

    Ok((StatusCode::OK, Json(response)))
}

/// Generate JWT access token
fn generate_access_token(
    client_id: &str,
    scopes: &[String],
    resource: Option<&str>,
) -> Result<String, (StatusCode, Json<OAuthError>)> {
    // TODO: Load JWT signing key from secure storage
    // For now, use a placeholder key (MUST be replaced with actual secret)
    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "REPLACE_THIS_WITH_SECURE_SECRET_FROM_ENV_OR_VAULT".to_string());

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

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
