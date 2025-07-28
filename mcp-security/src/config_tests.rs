//! Comprehensive unit tests for security configuration

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();

        assert!(config.validate_requests);
        assert!(config.rate_limiting);
        assert_eq!(config.max_requests_per_minute, 60);
        assert!(!config.cors_enabled);
        assert_eq!(config.cors_origins, vec!["*"]);
    }

    #[test]
    fn test_security_config_clone() {
        let original = SecurityConfig {
            validate_requests: false,
            rate_limiting: false,
            max_requests_per_minute: 120,
            cors_enabled: true,
            cors_origins: vec!["https://example.com".to_string()],
        };

        let cloned = original.clone();

        assert_eq!(cloned.validate_requests, original.validate_requests);
        assert_eq!(cloned.rate_limiting, original.rate_limiting);
        assert_eq!(
            cloned.max_requests_per_minute,
            original.max_requests_per_minute
        );
        assert_eq!(cloned.cors_enabled, original.cors_enabled);
        assert_eq!(cloned.cors_origins, original.cors_origins);
    }

    #[test]
    fn test_security_config_serialization() {
        let config = SecurityConfig {
            validate_requests: true,
            rate_limiting: true,
            max_requests_per_minute: 100,
            cors_enabled: true,
            cors_origins: vec![
                "https://app.example.com".to_string(),
                "http://localhost:3000".to_string(),
            ],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&config).unwrap();

        // Deserialize back
        let deserialized: SecurityConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.validate_requests, config.validate_requests);
        assert_eq!(deserialized.rate_limiting, config.rate_limiting);
        assert_eq!(
            deserialized.max_requests_per_minute,
            config.max_requests_per_minute
        );
        assert_eq!(deserialized.cors_enabled, config.cors_enabled);
        assert_eq!(deserialized.cors_origins, config.cors_origins);
    }

    #[test]
    fn test_security_config_edge_cases() {
        // Test with empty CORS origins
        let config1 = SecurityConfig {
            cors_origins: vec![],
            ..Default::default()
        };
        assert!(config1.cors_origins.is_empty());

        // Test with zero max requests
        let config2 = SecurityConfig {
            max_requests_per_minute: 0,
            ..Default::default()
        };
        assert_eq!(config2.max_requests_per_minute, 0);

        // Test with very large max requests
        let config3 = SecurityConfig {
            max_requests_per_minute: u32::MAX,
            ..Default::default()
        };
        assert_eq!(config3.max_requests_per_minute, u32::MAX);
    }

    #[test]
    fn test_security_config_custom_values() {
        let config = SecurityConfig {
            validate_requests: false,
            rate_limiting: false,
            max_requests_per_minute: 30,
            cors_enabled: true,
            cors_origins: vec![
                "https://app1.example.com".to_string(),
                "https://app2.example.com".to_string(),
                "http://localhost:*".to_string(),
            ],
        };

        assert!(!config.validate_requests);
        assert!(!config.rate_limiting);
        assert_eq!(config.max_requests_per_minute, 30);
        assert!(config.cors_enabled);
        assert_eq!(config.cors_origins.len(), 3);
    }

    #[test]
    fn test_security_config_partial_deserialization() {
        // Test that missing fields use defaults
        let json = r#"{"validate_requests": false}"#;
        let config: SecurityConfig = serde_json::from_str(json).unwrap();

        assert!(!config.validate_requests);
        assert!(config.rate_limiting); // Should use default
        assert_eq!(config.max_requests_per_minute, 60); // Should use default
    }

    #[test]
    fn test_security_config_json_roundtrip() {
        let configs = vec![
            SecurityConfig::default(),
            SecurityConfig {
                validate_requests: false,
                rate_limiting: true,
                max_requests_per_minute: 120,
                cors_enabled: true,
                cors_origins: vec!["*".to_string()],
            },
            SecurityConfig {
                validate_requests: true,
                rate_limiting: false,
                max_requests_per_minute: 1,
                cors_enabled: false,
                cors_origins: vec![],
            },
        ];

        for config in configs {
            let json = serde_json::to_string(&config).unwrap();
            let recovered: SecurityConfig = serde_json::from_str(&json).unwrap();

            assert_eq!(recovered.validate_requests, config.validate_requests);
            assert_eq!(recovered.rate_limiting, config.rate_limiting);
            assert_eq!(
                recovered.max_requests_per_minute,
                config.max_requests_per_minute
            );
            assert_eq!(recovered.cors_enabled, config.cors_enabled);
            assert_eq!(recovered.cors_origins, config.cors_origins);
        }
    }

    #[test]
    fn test_cors_origin_patterns() {
        // Test various CORS origin patterns
        let config = SecurityConfig {
            cors_enabled: true,
            cors_origins: vec![
                "*".to_string(),
                "https://*.example.com".to_string(),
                "http://localhost:3000".to_string(),
                "https://app.example.com:8443".to_string(),
                "file://".to_string(),
            ],
            ..Default::default()
        };

        assert_eq!(config.cors_origins.len(), 5);
        assert!(config.cors_origins.contains(&"*".to_string()));
        assert!(
            config
                .cors_origins
                .contains(&"https://*.example.com".to_string())
        );
    }

    #[test]
    fn test_security_config_debug() {
        let config = SecurityConfig::default();
        let debug_str = format!("{config:?}");

        assert!(debug_str.contains("SecurityConfig"));
        assert!(debug_str.contains("validate_requests"));
        assert!(debug_str.contains("rate_limiting"));
    }

    #[test]
    fn test_security_config_send_sync() {
        // Ensure SecurityConfig implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SecurityConfig>();
    }
}
