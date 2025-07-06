//! Comprehensive unit tests for mcp-security lib module

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_default_config() {
        let config = default_config();

        // Verify all default values
        assert!(config.validate_requests);
        assert!(config.rate_limiting);
        assert_eq!(config.max_requests_per_minute, 60);
        assert!(!config.cors_enabled);
        assert_eq!(config.cors_origins, vec!["*"]);
    }

    #[test]
    fn test_reexports() {
        // Test that all public types are properly re-exported
        let _config = SecurityConfig::default();
        let _middleware = SecurityMiddleware::new(SecurityConfig::default());

        // Test that RequestValidator is accessible
        use crate::validation::RequestValidator;
        let _validator = RequestValidator;
    }

    #[test]
    fn test_default_config_consistency() {
        let config1 = default_config();
        let config2 = default_config();

        // Should return consistent defaults
        assert_eq!(config1.validate_requests, config2.validate_requests);
        assert_eq!(config1.rate_limiting, config2.rate_limiting);
        assert_eq!(
            config1.max_requests_per_minute,
            config2.max_requests_per_minute
        );
        assert_eq!(config1.cors_enabled, config2.cors_enabled);
        assert_eq!(config1.cors_origins, config2.cors_origins);
    }

    #[test]
    fn test_module_visibility() {
        // Test that modules are publicly accessible
        use crate::{config, middleware, validation};

        // Should be able to access module items
        let _ = config::SecurityConfig::default();
        let _ = middleware::SecurityMiddleware::new(config::SecurityConfig::default());
        let _ = validation::RequestValidator;
    }
}
