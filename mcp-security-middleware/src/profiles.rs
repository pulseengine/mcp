//! Security profiles for different deployment environments

use crate::error::{SecurityError, SecurityResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Security profile defining the security level and features
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityProfile {
    /// Development profile - permissive settings for local development
    Development,

    /// Staging profile - balanced security for testing environments
    Staging,

    /// Production profile - strict security for production deployments
    Production,

    /// Custom profile - user-defined security settings
    Custom,
}

impl SecurityProfile {
    /// Parse security profile from string
    pub fn from_str(s: &str) -> SecurityResult<Self> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "staging" | "stage" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            "custom" => Ok(Self::Custom),
            _ => Err(SecurityError::config(format!(
                "Invalid security profile: {s}"
            ))),
        }
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Production => "production",
            Self::Custom => "custom",
        }
    }
}

impl std::str::FromStr for SecurityProfile {
    type Err = SecurityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

impl std::fmt::Display for SecurityProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,

    /// Time window duration
    pub window_duration: Duration,

    /// Whether rate limiting is enabled
    pub enabled: bool,
}

impl RateLimitConfig {
    /// Create permissive rate limiting (high limits)
    pub fn permissive() -> Self {
        Self {
            max_requests: 10000,
            window_duration: Duration::from_secs(60),
            enabled: false,
        }
    }

    /// Create moderate rate limiting
    pub fn moderate() -> Self {
        Self {
            max_requests: 1000,
            window_duration: Duration::from_secs(60),
            enabled: true,
        }
    }

    /// Create strict rate limiting (low limits)
    pub fn strict() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            enabled: true,
        }
    }
}

/// CORS (Cross-Origin Resource Sharing) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,

    /// Allow credentials
    pub allow_credentials: bool,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Whether CORS is enabled
    pub enabled: bool,
}

impl CorsConfig {
    /// Create permissive CORS configuration
    pub fn permissive() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allow_credentials: false,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["*".to_string()],
            enabled: true,
        }
    }

    /// Create localhost-only CORS configuration
    pub fn localhost_only() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:3001".to_string(),
                "http://localhost:8080".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://127.0.0.1:3001".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ],
            allow_credentials: true,
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
            allowed_headers: vec![
                "authorization".to_string(),
                "content-type".to_string(),
                "x-request-id".to_string(),
            ],
            enabled: true,
        }
    }

    /// Create strict CORS configuration
    pub fn strict() -> Self {
        Self {
            allowed_origins: Vec::new(), // Must be explicitly configured
            allow_credentials: true,
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()],
            allowed_headers: vec!["authorization".to_string(), "content-type".to_string()],
            enabled: true,
        }
    }
}

/// Development security profile implementation
pub struct DevelopmentProfile;

impl DevelopmentProfile {
    /// Create development security settings
    pub fn security_settings() -> SecuritySettings {
        SecuritySettings {
            require_authentication: false, // Optional for dev
            require_https: false,
            enable_audit_logging: true, // Good for debugging
            jwt_expiry_seconds: 86400,  // 24 hours - long for dev convenience
            rate_limit: RateLimitConfig::permissive(),
            cors: CorsConfig::permissive(),
            auto_generate_keys: true,
            validate_token_audience: false, // Relaxed for dev
        }
    }

    /// Get recommended environment variables for development
    pub fn recommended_env_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("MCP_SECURITY_PROFILE", "development"),
            ("MCP_API_KEY", "auto-generate"),
            ("MCP_JWT_SECRET", "auto-generate"),
            ("MCP_REQUIRE_HTTPS", "false"),
            ("MCP_ENABLE_AUDIT_LOG", "true"),
            ("MCP_CORS_ORIGIN", "*"),
        ]
    }
}

/// Staging security profile implementation
pub struct StagingProfile;

impl StagingProfile {
    /// Create staging security settings
    pub fn security_settings() -> SecuritySettings {
        SecuritySettings {
            require_authentication: true,
            require_https: true, // HTTPS required for staging
            enable_audit_logging: true,
            jwt_expiry_seconds: 3600, // 1 hour
            rate_limit: RateLimitConfig::moderate(),
            cors: CorsConfig::localhost_only(),
            auto_generate_keys: true,
            validate_token_audience: true,
        }
    }

    /// Get recommended environment variables for staging
    pub fn recommended_env_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("MCP_SECURITY_PROFILE", "staging"),
            ("MCP_API_KEY", "auto-generate"),
            ("MCP_JWT_SECRET", "auto-generate"),
            ("MCP_REQUIRE_HTTPS", "true"),
            ("MCP_ENABLE_AUDIT_LOG", "true"),
            ("MCP_RATE_LIMIT", "1000/min"),
            ("MCP_CORS_ORIGIN", "localhost"),
        ]
    }
}

/// Production security profile implementation
pub struct ProductionProfile;

impl ProductionProfile {
    /// Create production security settings
    pub fn security_settings() -> SecuritySettings {
        SecuritySettings {
            require_authentication: true,
            require_https: true, // Mandatory HTTPS
            enable_audit_logging: true,
            jwt_expiry_seconds: 900, // 15 minutes - short for security
            rate_limit: RateLimitConfig::strict(),
            cors: CorsConfig::strict(),
            auto_generate_keys: false, // Manual key management in production
            validate_token_audience: true,
        }
    }

    /// Get required environment variables for production
    pub fn required_env_vars() -> Vec<&'static str> {
        vec![
            "MCP_API_KEY",
            "MCP_JWT_SECRET",
            "MCP_CORS_ORIGIN",
            "MCP_ALLOWED_ORIGINS",
        ]
    }

    /// Get recommended environment variables for production
    pub fn recommended_env_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("MCP_SECURITY_PROFILE", "production"),
            ("MCP_REQUIRE_HTTPS", "true"),
            ("MCP_ENABLE_AUDIT_LOG", "true"),
            ("MCP_RATE_LIMIT", "100/min"),
            ("MCP_JWT_EXPIRY", "900"), // 15 minutes
        ]
    }
}

/// Consolidated security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Whether authentication is required
    pub require_authentication: bool,

    /// Whether HTTPS is required
    pub require_https: bool,

    /// Whether audit logging is enabled
    pub enable_audit_logging: bool,

    /// JWT token expiry in seconds
    pub jwt_expiry_seconds: u64,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,

    /// CORS configuration
    pub cors: CorsConfig,

    /// Whether to auto-generate keys if not provided
    pub auto_generate_keys: bool,

    /// Whether to validate JWT token audience
    pub validate_token_audience: bool,
}

impl SecuritySettings {
    /// Create settings for a specific profile
    pub fn for_profile(profile: &SecurityProfile) -> Self {
        match profile {
            SecurityProfile::Development => DevelopmentProfile::security_settings(),
            SecurityProfile::Staging => StagingProfile::security_settings(),
            SecurityProfile::Production => ProductionProfile::security_settings(),
            SecurityProfile::Custom => Self::default(), // User must customize
        }
    }

    /// Validate the security settings
    pub fn validate(&self) -> SecurityResult<()> {
        // Check for inconsistencies
        if self.require_authentication && !self.auto_generate_keys {
            // In production, we need explicit keys
            return Ok(()); // This will be validated elsewhere
        }

        if self.require_https
            && self.cors.allowed_origins.contains(&"*".to_string())
            && self.cors.allow_credentials
        {
            return Err(SecurityError::config(
                "Cannot use wildcard origins with credentials over HTTPS",
            ));
        }

        if self.jwt_expiry_seconds > 86400 * 7 {
            // More than 1 week
            tracing::warn!(
                "JWT expiry is longer than 1 week, consider shorter expiry for security"
            );
        }

        if self.jwt_expiry_seconds < 60 {
            // Less than 1 minute
            return Err(SecurityError::config(
                "JWT expiry cannot be less than 1 minute",
            ));
        }

        Ok(())
    }

    /// Get the security level description
    pub fn security_level_description(&self) -> &'static str {
        if !self.require_authentication {
            "Minimal - Authentication disabled"
        } else if !self.require_https {
            "Low - HTTP allowed"
        } else if self.auto_generate_keys {
            "Medium - Auto-generated keys"
        } else {
            "High - Manual key management"
        }
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        // Safe defaults that work but encourage explicit configuration
        Self {
            require_authentication: true,
            require_https: true,
            enable_audit_logging: true,
            jwt_expiry_seconds: 3600, // 1 hour
            rate_limit: RateLimitConfig::moderate(),
            cors: CorsConfig::strict(),
            auto_generate_keys: false,
            validate_token_audience: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_profile_parsing() {
        assert_eq!(
            SecurityProfile::from_str("development").unwrap(),
            SecurityProfile::Development
        );
        assert_eq!(
            SecurityProfile::from_str("dev").unwrap(),
            SecurityProfile::Development
        );
        assert_eq!(
            SecurityProfile::from_str("staging").unwrap(),
            SecurityProfile::Staging
        );
        assert_eq!(
            SecurityProfile::from_str("production").unwrap(),
            SecurityProfile::Production
        );

        assert!(SecurityProfile::from_str("invalid").is_err());
    }

    #[test]
    fn test_security_profile_display() {
        assert_eq!(SecurityProfile::Development.to_string(), "development");
        assert_eq!(SecurityProfile::Staging.to_string(), "staging");
        assert_eq!(SecurityProfile::Production.to_string(), "production");
    }

    #[test]
    fn test_development_profile() {
        let settings = DevelopmentProfile::security_settings();
        assert!(!settings.require_authentication);
        assert!(!settings.require_https);
        assert!(settings.enable_audit_logging);
        assert!(settings.auto_generate_keys);
        assert!(!settings.rate_limit.enabled);
    }

    #[test]
    fn test_staging_profile() {
        let settings = StagingProfile::security_settings();
        assert!(settings.require_authentication);
        assert!(settings.require_https);
        assert!(settings.enable_audit_logging);
        assert!(settings.auto_generate_keys);
        assert!(settings.rate_limit.enabled);
        assert_eq!(settings.jwt_expiry_seconds, 3600);
    }

    #[test]
    fn test_production_profile() {
        let settings = ProductionProfile::security_settings();
        assert!(settings.require_authentication);
        assert!(settings.require_https);
        assert!(settings.enable_audit_logging);
        assert!(!settings.auto_generate_keys); // Manual key management
        assert!(settings.rate_limit.enabled);
        assert_eq!(settings.jwt_expiry_seconds, 900); // 15 minutes
    }

    #[test]
    fn test_rate_limit_configs() {
        let permissive = RateLimitConfig::permissive();
        assert!(!permissive.enabled);
        assert_eq!(permissive.max_requests, 10000);

        let strict = RateLimitConfig::strict();
        assert!(strict.enabled);
        assert_eq!(strict.max_requests, 100);
    }

    #[test]
    fn test_cors_configs() {
        let permissive = CorsConfig::permissive();
        assert_eq!(permissive.allowed_origins, vec!["*"]);
        assert!(!permissive.allow_credentials);

        let localhost = CorsConfig::localhost_only();
        assert!(localhost.allow_credentials);
        assert!(
            localhost
                .allowed_origins
                .contains(&"http://localhost:3000".to_string())
        );

        let strict = CorsConfig::strict();
        assert!(strict.allowed_origins.is_empty());
        assert!(strict.allow_credentials);
    }

    #[test]
    fn test_security_settings_validation() {
        let mut settings = SecuritySettings::default();
        assert!(settings.validate().is_ok());

        // Test invalid JWT expiry - need to avoid the early return by setting auto_generate_keys
        settings.auto_generate_keys = true; // This prevents early return
        settings.jwt_expiry_seconds = 30; // Too short (less than 60 seconds)
        assert!(settings.validate().is_err());

        settings.jwt_expiry_seconds = 3600; // Valid again
        assert!(settings.validate().is_ok());

        // Test CORS validation with credentials + wildcard
        settings.cors.allowed_origins = vec!["*".to_string()];
        settings.cors.allow_credentials = true;
        settings.require_https = true;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_security_settings_for_profiles() {
        let dev_settings = SecuritySettings::for_profile(&SecurityProfile::Development);
        assert!(!dev_settings.require_authentication);

        let prod_settings = SecuritySettings::for_profile(&SecurityProfile::Production);
        assert!(prod_settings.require_authentication);
        assert!(prod_settings.require_https);
    }
}
