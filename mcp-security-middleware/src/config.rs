//! Configuration management for security middleware

use crate::auth::{ApiKeyValidator, TokenValidator};
use crate::error::{SecurityError, SecurityResult};
use crate::profiles::{SecurityProfile, SecuritySettings};
use crate::utils::{generate_api_key, generate_jwt_secret};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Main security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Security profile
    pub profile: SecurityProfile,
    
    /// Security settings
    pub settings: SecuritySettings,
    
    /// API key for simple authentication
    pub api_key: Option<String>,
    
    /// JWT secret for token validation
    pub jwt_secret: Option<String>,
    
    /// JWT issuer
    pub jwt_issuer: String,
    
    /// JWT audience
    pub jwt_audience: String,
    
    /// Configuration file path (if loaded from file)
    pub config_file_path: Option<PathBuf>,
}

impl SecurityConfig {
    /// Create a new security configuration with custom settings
    pub fn new(profile: SecurityProfile, settings: SecuritySettings) -> Self {
        Self {
            profile: profile.clone(),
            settings,
            api_key: None,
            jwt_secret: None,
            jwt_issuer: "mcp-security-middleware".to_string(),
            jwt_audience: "mcp-server".to_string(),
            config_file_path: None,
        }
    }

    /// Create development configuration
    pub fn development() -> Self {
        let profile = SecurityProfile::Development;
        let settings = SecuritySettings::for_profile(&profile);
        let mut config = Self::new(profile, settings);
        
        // Auto-generate keys for development
        if config.settings.auto_generate_keys {
            config.api_key = Some(generate_api_key());
            config.jwt_secret = Some(generate_jwt_secret());
        }
        
        config
    }

    /// Create staging configuration
    pub fn staging() -> Self {
        let profile = SecurityProfile::Staging;
        let settings = SecuritySettings::for_profile(&profile);
        let mut config = Self::new(profile, settings);
        
        // Auto-generate keys for staging
        if config.settings.auto_generate_keys {
            config.api_key = Some(generate_api_key());
            config.jwt_secret = Some(generate_jwt_secret());
        }
        
        config
    }

    /// Create production configuration
    pub fn production() -> Self {
        let profile = SecurityProfile::Production;
        let settings = SecuritySettings::for_profile(&profile);
        Self::new(profile, settings)
    }

    /// Create configuration from environment variables
    pub fn from_env() -> SecurityResult<Self> {
        // Get security profile from environment
        let profile = env::var("MCP_SECURITY_PROFILE")
            .unwrap_or_else(|_| "production".to_string())
            .parse::<SecurityProfile>()?;

        // Start with profile defaults
        let mut config = match profile {
            SecurityProfile::Development => Self::development(),
            SecurityProfile::Staging => Self::staging(),
            SecurityProfile::Production => Self::production(),
            SecurityProfile::Custom => Self::new(profile, SecuritySettings::default()),
        };

        // Override with environment variables
        config.load_from_env()?;
        
        Ok(config)
    }

    /// Load configuration overrides from environment variables
    pub fn load_from_env(&mut self) -> SecurityResult<()> {
        // API Key
        if let Ok(api_key) = env::var("MCP_API_KEY") {
            if api_key == "auto-generate" {
                if self.settings.auto_generate_keys {
                    self.api_key = Some(generate_api_key());
                    tracing::info!("Auto-generated API key for MCP server");
                } else {
                    return Err(SecurityError::config(
                        "Auto-generation disabled for this security profile"
                    ));
                }
            } else {
                self.api_key = Some(api_key);
            }
        }

        // JWT Secret
        if let Ok(jwt_secret) = env::var("MCP_JWT_SECRET") {
            if jwt_secret == "auto-generate" {
                if self.settings.auto_generate_keys {
                    self.jwt_secret = Some(generate_jwt_secret());
                    tracing::info!("Auto-generated JWT secret for MCP server");
                } else {
                    return Err(SecurityError::config(
                        "Auto-generation disabled for this security profile"
                    ));
                }
            } else {
                self.jwt_secret = Some(jwt_secret);
            }
        }

        // JWT Issuer
        if let Ok(issuer) = env::var("MCP_JWT_ISSUER") {
            self.jwt_issuer = issuer;
        }

        // JWT Audience
        if let Ok(audience) = env::var("MCP_JWT_AUDIENCE") {
            self.jwt_audience = audience;
        }

        // HTTPS requirement
        if let Ok(require_https) = env::var("MCP_REQUIRE_HTTPS") {
            self.settings.require_https = require_https.parse()
                .unwrap_or(self.settings.require_https);
        }

        // Authentication requirement
        if let Ok(require_auth) = env::var("MCP_REQUIRE_AUTH") {
            self.settings.require_authentication = require_auth.parse()
                .unwrap_or(self.settings.require_authentication);
        }

        // Audit logging
        if let Ok(audit_log) = env::var("MCP_ENABLE_AUDIT_LOG") {
            self.settings.enable_audit_logging = audit_log.parse()
                .unwrap_or(self.settings.enable_audit_logging);
        }

        // JWT expiry
        if let Ok(jwt_expiry) = env::var("MCP_JWT_EXPIRY") {
            if let Ok(expiry_seconds) = jwt_expiry.parse::<u64>() {
                self.settings.jwt_expiry_seconds = expiry_seconds;
            }
        }

        // Rate limiting
        if let Ok(rate_limit) = env::var("MCP_RATE_LIMIT") {
            if let Some(config) = parse_rate_limit(&rate_limit) {
                self.settings.rate_limit = config;
            }
        }

        // CORS origins
        if let Ok(cors_origin) = env::var("MCP_CORS_ORIGIN") {
            if cors_origin == "*" {
                self.settings.cors.allowed_origins = vec!["*".to_string()];
                self.settings.cors.allow_credentials = false; // Can't use credentials with wildcard
            } else if cors_origin == "localhost" {
                self.settings.cors = crate::profiles::CorsConfig::localhost_only();
            } else {
                // Parse comma-separated origins
                self.settings.cors.allowed_origins = cors_origin
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
        }

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> SecurityResult<()> {
        // Validate security settings
        self.settings.validate()?;

        // Check required keys for authentication
        if self.settings.require_authentication {
            if self.api_key.is_none() && self.jwt_secret.is_none() {
                return Err(SecurityError::config(
                    "Authentication is required but no API key or JWT secret provided. \
                     Set MCP_API_KEY or MCP_JWT_SECRET environment variables, \
                     or use MCP_API_KEY=auto-generate for development."
                ));
            }
        }

        // Validate JWT configuration if JWT secret is provided
        if let Some(ref secret) = self.jwt_secret {
            if secret.len() < 32 {
                return Err(SecurityError::config(
                    "JWT secret must be at least 32 characters long for security"
                ));
            }
            
            if self.jwt_issuer.is_empty() {
                return Err(SecurityError::config("JWT issuer cannot be empty"));
            }
            
            if self.jwt_audience.is_empty() {
                return Err(SecurityError::config("JWT audience cannot be empty"));
            }
        }

        // Validate API key format if provided
        if let Some(ref api_key) = self.api_key {
            crate::utils::validate_api_key_format(api_key)?;
        }

        Ok(())
    }

    /// Create an API key validator from this configuration
    pub fn create_api_key_validator(&self) -> SecurityResult<Option<ApiKeyValidator>> {
        if let Some(ref api_key) = self.api_key {
            let mut validator = ApiKeyValidator::new();
            validator.add_api_key(api_key, "default-user".to_string())?;
            Ok(Some(validator))
        } else {
            Ok(None)
        }
    }

    /// Create a token validator from this configuration
    pub fn create_token_validator(&self) -> SecurityResult<Option<TokenValidator>> {
        if let Some(ref jwt_secret) = self.jwt_secret {
            let validator = TokenValidator::new(
                jwt_secret,
                self.jwt_issuer.clone(),
                self.jwt_audience.clone(),
            );
            Ok(Some(validator))
        } else {
            Ok(None)
        }
    }

    /// Create security middleware from this configuration
    pub async fn create_middleware(&self) -> SecurityResult<crate::middleware::SecurityMiddleware> {
        self.validate()?;
        
        let api_key_validator = self.create_api_key_validator()?;
        let token_validator = self.create_token_validator()?;
        
        let middleware = crate::middleware::SecurityMiddleware::new(
            self.clone(),
            api_key_validator,
            token_validator,
        );
        
        Ok(middleware)
    }

    /// Get configuration summary for logging
    pub fn summary(&self) -> ConfigSummary {
        ConfigSummary {
            profile: self.profile.clone(),
            security_level: self.settings.security_level_description().to_string(),
            authentication_enabled: self.settings.require_authentication,
            https_required: self.settings.require_https,
            rate_limiting_enabled: self.settings.rate_limit.enabled,
            cors_enabled: self.settings.cors.enabled,
            audit_logging_enabled: self.settings.enable_audit_logging,
            has_api_key: self.api_key.is_some(),
            has_jwt_secret: self.jwt_secret.is_some(),
            jwt_expiry_minutes: self.settings.jwt_expiry_seconds / 60,
        }
    }

    /// Set API key
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set JWT secret
    pub fn with_jwt_secret<S: Into<String>>(mut self, jwt_secret: S) -> Self {
        self.jwt_secret = Some(jwt_secret.into());
        self
    }

    /// Set JWT issuer
    pub fn with_jwt_issuer<S: Into<String>>(mut self, issuer: S) -> Self {
        self.jwt_issuer = issuer.into();
        self
    }

    /// Set JWT audience
    pub fn with_jwt_audience<S: Into<String>>(mut self, audience: S) -> Self {
        self.jwt_audience = audience.into();
        self
    }
}

/// Configuration summary for logging and display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    pub profile: SecurityProfile,
    pub security_level: String,
    pub authentication_enabled: bool,
    pub https_required: bool,
    pub rate_limiting_enabled: bool,
    pub cors_enabled: bool,
    pub audit_logging_enabled: bool,
    pub has_api_key: bool,
    pub has_jwt_secret: bool,
    pub jwt_expiry_minutes: u64,
}

impl std::fmt::Display for ConfigSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "MCP Security Configuration Summary:")?;
        writeln!(f, "  Profile: {}", self.profile)?;
        writeln!(f, "  Security Level: {}", self.security_level)?;
        writeln!(f, "  Authentication: {}", if self.authentication_enabled { "✓ Enabled" } else { "✗ Disabled" })?;
        writeln!(f, "  HTTPS Required: {}", if self.https_required { "✓ Yes" } else { "✗ No" })?;
        writeln!(f, "  Rate Limiting: {}", if self.rate_limiting_enabled { "✓ Enabled" } else { "✗ Disabled" })?;
        writeln!(f, "  CORS: {}", if self.cors_enabled { "✓ Enabled" } else { "✗ Disabled" })?;
        writeln!(f, "  Audit Logging: {}", if self.audit_logging_enabled { "✓ Enabled" } else { "✗ Disabled" })?;
        writeln!(f, "  API Key: {}", if self.has_api_key { "✓ Configured" } else { "✗ Not set" })?;
        writeln!(f, "  JWT Secret: {}", if self.has_jwt_secret { "✓ Configured" } else { "✗ Not set" })?;
        write!(f, "  JWT Expiry: {} minutes", self.jwt_expiry_minutes)
    }
}

/// Parse rate limit string like "100/min" or "1000/hour"
fn parse_rate_limit(rate_limit: &str) -> Option<crate::profiles::RateLimitConfig> {
    let parts: Vec<&str> = rate_limit.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let max_requests = parts[0].parse::<u32>().ok()?;
    let window_duration = match parts[1] {
        "sec" | "second" | "s" => std::time::Duration::from_secs(1),
        "min" | "minute" | "m" => std::time::Duration::from_secs(60),
        "hour" | "h" => std::time::Duration::from_secs(3600),
        "day" | "d" => std::time::Duration::from_secs(86400),
        _ => return None,
    };

    Some(crate::profiles::RateLimitConfig {
        max_requests,
        window_duration,
        enabled: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_development_config() {
        let config = SecurityConfig::development();
        assert_eq!(config.profile, SecurityProfile::Development);
        assert!(!config.settings.require_authentication);
        assert!(config.api_key.is_some());
        assert!(config.jwt_secret.is_some());
    }

    #[test]
    fn test_production_config() {
        let config = SecurityConfig::production();
        assert_eq!(config.profile, SecurityProfile::Production);
        assert!(config.settings.require_authentication);
        assert!(config.settings.require_https);
        assert!(config.api_key.is_none()); // Must be explicitly configured
        assert!(config.jwt_secret.is_none());
    }

    #[test]
    fn test_config_validation() {
        let mut config = SecurityConfig::development();
        assert!(config.validate().is_ok());

        // Remove keys but require authentication
        config.api_key = None;
        config.jwt_secret = None;
        config.settings.require_authentication = true;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_env_config_loading() {
        unsafe {
            env::set_var("MCP_SECURITY_PROFILE", "development");
            env::set_var("MCP_API_KEY", "test-key-123");
            env::set_var("MCP_JWT_SECRET", "test-secret-very-long-secret-key-for-testing");
        }
        
        let config = SecurityConfig::from_env().unwrap();
        assert_eq!(config.profile, SecurityProfile::Development);
        assert_eq!(config.api_key.as_ref().unwrap(), "test-key-123");
        
        // Clean up
        unsafe {
            env::remove_var("MCP_SECURITY_PROFILE");
            env::remove_var("MCP_API_KEY");
            env::remove_var("MCP_JWT_SECRET");
        }
    }

    #[test]
    fn test_parse_rate_limit() {
        let config = parse_rate_limit("100/min").unwrap();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_duration, std::time::Duration::from_secs(60));

        let config = parse_rate_limit("1000/hour").unwrap();
        assert_eq!(config.max_requests, 1000);
        assert_eq!(config.window_duration, std::time::Duration::from_secs(3600));

        assert!(parse_rate_limit("invalid").is_none());
    }

    #[test]
    fn test_config_summary() {
        let config = SecurityConfig::development();
        let summary = config.summary();
        
        assert_eq!(summary.profile, SecurityProfile::Development);
        assert!(!summary.authentication_enabled);
        assert!(summary.has_api_key);
        assert!(summary.has_jwt_secret);
    }

    #[test]
    fn test_config_builder_methods() {
        let config = SecurityConfig::development()
            .with_api_key("custom-key")
            .with_jwt_secret("custom-secret-very-long-for-security")
            .with_jwt_issuer("custom-issuer")
            .with_jwt_audience("custom-audience");

        assert_eq!(config.api_key.as_ref().unwrap(), "custom-key");
        assert_eq!(config.jwt_issuer, "custom-issuer");
        assert_eq!(config.jwt_audience, "custom-audience");
    }
}