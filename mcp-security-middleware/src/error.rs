//! Error types for the security middleware

use thiserror::Error;

/// Result type for security operations
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Errors that can occur in the security middleware
#[derive(Debug, Error)]
pub enum SecurityError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Authorization failed
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    /// Invalid token
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Token expired
    #[error("Token expired")]
    TokenExpired,

    /// Missing authentication
    #[error("Missing authentication credentials")]
    MissingAuth,

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Invalid API key
    #[error("Invalid API key")]
    InvalidApiKey,

    /// JWT validation error
    #[error("JWT validation error: {0}")]
    JwtValidation(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    Environment(#[from] std::env::VarError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Random generation error
    #[error("Random generation error: {0}")]
    Random(String),

    /// Cryptographic error
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Generic internal error
    #[error("Internal security error: {0}")]
    Internal(String),
}

impl SecurityError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    /// Create an authentication error
    pub fn auth<S: Into<String>>(msg: S) -> Self {
        Self::AuthenticationFailed(msg.into())
    }

    /// Create an authorization error
    pub fn authz<S: Into<String>>(msg: S) -> Self {
        Self::AuthorizationFailed(msg.into())
    }

    /// Create an invalid token error
    pub fn invalid_token<S: Into<String>>(msg: S) -> Self {
        Self::InvalidToken(msg.into())
    }

    /// Create a JWT validation error
    pub fn jwt<S: Into<String>>(msg: S) -> Self {
        Self::JwtValidation(msg.into())
    }

    /// Create a random generation error
    pub fn random<S: Into<String>>(msg: S) -> Self {
        Self::Random(msg.into())
    }

    /// Create a cryptographic error
    pub fn crypto<S: Into<String>>(msg: S) -> Self {
        Self::Crypto(msg.into())
    }

    /// Create an HTTP error
    pub fn http<S: Into<String>>(msg: S) -> Self {
        Self::Http(msg.into())
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }
}

// Convert jsonwebtoken errors to SecurityError
impl From<jsonwebtoken::errors::Error> for SecurityError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;

        match err.kind() {
            ErrorKind::ExpiredSignature => SecurityError::TokenExpired,
            ErrorKind::InvalidToken => SecurityError::InvalidToken("Invalid JWT token".to_string()),
            ErrorKind::InvalidSignature => {
                SecurityError::InvalidToken("Invalid JWT signature".to_string())
            }
            ErrorKind::InvalidAudience => {
                SecurityError::InvalidToken("Invalid JWT audience".to_string())
            }
            ErrorKind::InvalidIssuer => {
                SecurityError::InvalidToken("Invalid JWT issuer".to_string())
            }
            _ => SecurityError::JwtValidation(err.to_string()),
        }
    }
}
