//! RFC 6750: Bearer Token Authentication
//!
//! Implements Bearer token validation for OAuth 2.1 resource servers.
//! MCP servers acting as resource servers must validate access tokens per OAuth 2.1 Section 5.2.

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, Validation, decode};

use super::models::AccessTokenClaims;

/// Bearer token error types per RFC 6750 Section 3.1
#[derive(Debug, Clone)]
pub enum BearerError {
    /// No token provided
    MissingToken,
    /// Token format invalid
    InvalidToken(String),
    /// Token expired
    ExpiredToken,
    /// Token doesn't have required scope
    InsufficientScope(String),
    /// Token was issued for different audience (RFC 8707)
    InvalidAudience(String),
}

impl BearerError {
    /// Get RFC 6750 error code
    pub fn error_code(&self) -> &'static str {
        match self {
            BearerError::MissingToken => "invalid_request",
            BearerError::InvalidToken(_) => "invalid_token",
            BearerError::ExpiredToken => "invalid_token",
            BearerError::InsufficientScope(_) => "insufficient_scope",
            BearerError::InvalidAudience(_) => "invalid_token",
        }
    }

    /// Get error description
    pub fn error_description(&self) -> String {
        match self {
            BearerError::MissingToken => "No access token provided".to_string(),
            BearerError::InvalidToken(msg) => msg.clone(),
            BearerError::ExpiredToken => "Access token has expired".to_string(),
            BearerError::InsufficientScope(scope) => {
                format!("Insufficient scope, required: {}", scope)
            }
            BearerError::InvalidAudience(aud) => {
                format!("Token not intended for this resource: {}", aud)
            }
        }
    }
}

/// WWW-Authenticate header builder per RFC 6750 Section 3
///
/// # Example Response
/// ```text
/// HTTP/1.1 401 Unauthorized
/// WWW-Authenticate: Bearer realm="mcp", error="invalid_token", error_description="Token expired"
/// ```
pub struct WwwAuthenticate {
    realm: String,
    error: Option<BearerError>,
    resource_metadata_url: Option<String>,
}

impl WwwAuthenticate {
    /// Create new WWW-Authenticate response
    pub fn new(realm: impl Into<String>) -> Self {
        Self {
            realm: realm.into(),
            error: None,
            resource_metadata_url: None,
        }
    }

    /// Add error information
    pub fn with_error(mut self, error: BearerError) -> Self {
        self.error = Some(error);
        self
    }

    /// Add RFC 9728 resource metadata URL
    pub fn with_resource_metadata(mut self, url: impl Into<String>) -> Self {
        self.resource_metadata_url = Some(url.into());
        self
    }

    /// Build header value
    pub fn to_header_value(&self) -> HeaderValue {
        let mut parts = vec![format!("Bearer realm=\"{}\"", self.realm)];

        if let Some(ref error) = self.error {
            parts.push(format!("error=\"{}\"", error.error_code()));
            parts.push(format!(
                "error_description=\"{}\"",
                error.error_description()
            ));
        }

        // RFC 9728 Section 5.1: Include resource metadata URL
        if let Some(ref url) = self.resource_metadata_url {
            parts.push(format!("resource_metadata=\"{}\"", url));
        }

        HeaderValue::from_str(&parts.join(", "))
            .unwrap_or_else(|_| HeaderValue::from_static("Bearer realm=\"mcp\""))
    }

    /// Build 401 response with WWW-Authenticate header
    pub fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(header::WWW_AUTHENTICATE, self.to_header_value());

        let body = if let Some(ref error) = self.error {
            serde_json::json!({
                "error": error.error_code(),
                "error_description": error.error_description()
            })
            .to_string()
        } else {
            "".to_string()
        };

        (StatusCode::UNAUTHORIZED, headers, body).into_response()
    }
}

/// Validated Bearer token with decoded claims
#[derive(Debug, Clone)]
pub struct BearerToken {
    pub claims: AccessTokenClaims,
    pub raw_token: String,
}

/// Configuration for Bearer token validation
#[derive(Debug, Clone)]
pub struct BearerTokenConfig {
    /// JWT secret key for validation
    pub jwt_secret: String,
    /// Expected audience (resource server identifier) - RFC 8707
    pub expected_audience: Option<String>,
    /// WWW-Authenticate realm
    pub realm: String,
    /// Resource metadata URL for error responses
    pub resource_metadata_url: Option<String>,
}

impl Default for BearerTokenConfig {
    fn default() -> Self {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "REPLACE_THIS_WITH_SECURE_SECRET".to_string()),
            expected_audience: Some(base_url.clone()),
            realm: "mcp".to_string(),
            resource_metadata_url: Some(format!(
                "{}/.well-known/oauth-protected-resource",
                base_url
            )),
        }
    }
}

/// Validate a Bearer token from Authorization header
pub fn validate_bearer_token(
    auth_header: &str,
    config: &BearerTokenConfig,
) -> Result<BearerToken, BearerError> {
    // Extract token from "Bearer <token>" format
    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or_else(|| BearerError::InvalidToken("Invalid authorization header format".into()))?;

    // Decode and validate JWT
    let mut validation = Validation::default();

    // Validate audience if configured (RFC 8707)
    if let Some(ref expected_aud) = config.expected_audience {
        validation.set_audience(&[expected_aud]);
    }

    let token_data = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => BearerError::ExpiredToken,
        jsonwebtoken::errors::ErrorKind::InvalidAudience => {
            BearerError::InvalidAudience(config.expected_audience.clone().unwrap_or_default())
        }
        _ => BearerError::InvalidToken(format!("Token validation failed: {}", e)),
    })?;

    Ok(BearerToken {
        claims: token_data.claims,
        raw_token: token.to_string(),
    })
}

/// Create 401 Unauthorized response with proper WWW-Authenticate header
pub fn unauthorized_response(error: BearerError, config: &BearerTokenConfig) -> Response {
    let mut www_auth = WwwAuthenticate::new(&config.realm).with_error(error);

    if let Some(ref url) = config.resource_metadata_url {
        www_auth = www_auth.with_resource_metadata(url);
    }

    www_auth.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_www_authenticate_header_basic() {
        let header = WwwAuthenticate::new("mcp").to_header_value();
        assert!(header.to_str().unwrap().contains("Bearer realm=\"mcp\""));
    }

    #[test]
    fn test_www_authenticate_header_with_error() {
        let header = WwwAuthenticate::new("mcp")
            .with_error(BearerError::ExpiredToken)
            .to_header_value();
        let header_str = header.to_str().unwrap();
        assert!(header_str.contains("error=\"invalid_token\""));
        assert!(header_str.contains("error_description="));
    }

    #[test]
    fn test_www_authenticate_header_with_resource_metadata() {
        let header = WwwAuthenticate::new("mcp")
            .with_resource_metadata("https://example.com/.well-known/oauth-protected-resource")
            .to_header_value();
        let header_str = header.to_str().unwrap();
        assert!(header_str.contains("resource_metadata="));
    }

    #[test]
    fn test_bearer_error_codes() {
        assert_eq!(BearerError::MissingToken.error_code(), "invalid_request");
        assert_eq!(
            BearerError::InvalidToken("test".into()).error_code(),
            "invalid_token"
        );
        assert_eq!(BearerError::ExpiredToken.error_code(), "invalid_token");
        assert_eq!(
            BearerError::InsufficientScope("test".into()).error_code(),
            "insufficient_scope"
        );
    }
}
