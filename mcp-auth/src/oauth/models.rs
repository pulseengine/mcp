//! OAuth 2.1 Data Models
//!
//! Database models for OAuth clients, authorization codes, and refresh tokens.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OAuth client registered via dynamic client registration (RFC 7591)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub client_secret_expires_at: Option<DateTime<Utc>>,
}

/// Authorization code issued during OAuth flow
#[derive(Debug, Clone)]
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,   // PKCE S256 challenge
    pub resource: Option<String>, // RFC 8707: Resource indicator
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Refresh token for obtaining new access tokens
#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub token: String,
    pub client_id: String,
    pub resource: Option<String>, // RFC 8707: Resource indicator
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Client registration request (RFC 7591)
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub grant_types: Vec<String>,
    #[serde(default)]
    pub response_types: Vec<String>,
    #[serde(default)]
    pub scope: String,
}

/// Client registration response (RFC 7591)
#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    pub client_secret: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub client_secret_expires_at: i64, // Unix timestamp, 0 = never expires
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
}

/// Authorization request parameters
#[derive(Debug, Deserialize)]
pub struct AuthorizeRequest {
    pub response_type: String, // Must be "code"
    pub client_id: String,
    pub redirect_uri: String,
    pub state: Option<String>,
    pub code_challenge: String,        // PKCE S256 code challenge
    pub code_challenge_method: String, // Must be "S256"
    pub resource: Option<String>,      // RFC 8707: Resource indicator
    pub scope: Option<String>,
}

/// Token request parameters
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,            // "authorization_code" or "refresh_token"
    pub code: Option<String>,          // For authorization_code grant
    pub redirect_uri: Option<String>,  // For authorization_code grant
    pub code_verifier: Option<String>, // PKCE S256 code verifier
    pub refresh_token: Option<String>, // For refresh_token grant
    pub client_id: String,
    pub client_secret: String,
    pub resource: Option<String>, // RFC 8707: Resource indicator
}

/// Token response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String, // Always "Bearer"
    pub expires_in: i64,    // Seconds until expiration
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Error response (RFC 6749 Section 5.2)
#[derive(Debug, Serialize)]
pub struct OAuthError {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}

impl OAuthError {
    pub fn invalid_request(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_request".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    pub fn invalid_client(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_client".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    pub fn invalid_grant(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_grant".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    pub fn unauthorized_client(description: impl Into<String>) -> Self {
        Self {
            error: "unauthorized_client".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    pub fn unsupported_grant_type(description: impl Into<String>) -> Self {
        Self {
            error: "unsupported_grant_type".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }

    pub fn invalid_scope(description: impl Into<String>) -> Self {
        Self {
            error: "invalid_scope".to_string(),
            error_description: Some(description.into()),
            error_uri: None,
        }
    }
}

/// JWT claims for access tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,         // Subject (client_id)
    pub aud: Option<String>, // Audience (resource server)
    pub exp: i64,            // Expiration time (Unix timestamp)
    pub iat: i64,            // Issued at (Unix timestamp)
    pub iss: String,         // Issuer (authorization server)
    pub scope: String,       // Space-separated scopes
    pub client_id: String,
}
