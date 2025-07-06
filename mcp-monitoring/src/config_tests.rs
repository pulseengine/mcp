//! Comprehensive unit tests for monitoring configuration

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json;

    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();

        assert!(config.enabled);
        assert_eq!(config.collection_interval_secs, 60);
        assert!(config.performance_monitoring);
        assert!(config.health_checks);
    }

    #[test]
    fn test_monitoring_config_clone() {
        let original = MonitoringConfig {
            enabled: false,
            collection_interval_secs: 30,
            performance_monitoring: false,
            health_checks: false,
        };

        let cloned = original.clone();

        assert_eq!(cloned.enabled, original.enabled);
        assert_eq!(
            cloned.collection_interval_secs,
            original.collection_interval_secs
        );
        assert_eq!(
            cloned.performance_monitoring,
            original.performance_monitoring
        );
        assert_eq!(cloned.health_checks, original.health_checks);
    }

    #[test]
    fn test_monitoring_config_serialization() {
        let config = MonitoringConfig {
            enabled: true,
            collection_interval_secs: 120,
            performance_monitoring: false,
            health_checks: true,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&config).unwrap();

        // Deserialize back
        let deserialized: MonitoringConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(
            deserialized.collection_interval_secs,
            config.collection_interval_secs
        );
        assert_eq!(
            deserialized.performance_monitoring,
            config.performance_monitoring
        );
        assert_eq!(deserialized.health_checks, config.health_checks);
    }

    #[test]
    fn test_monitoring_config_deserialization_with_defaults() {
        // Test that missing fields use defaults
        let json = r#"{"enabled": false}"#;
        let config: MonitoringConfig = serde_json::from_str(json).unwrap();

        assert!(!config.enabled);
        assert_eq!(config.collection_interval_secs, 60); // Should use default
        assert!(config.performance_monitoring); // Should use default
        assert!(config.health_checks); // Should use default
    }

    #[test]
    fn test_monitoring_config_edge_cases() {
        // Test with zero collection interval
        let config1 = MonitoringConfig {
            collection_interval_secs: 0,
            ..Default::default()
        };
        assert_eq!(config1.collection_interval_secs, 0);

        // Test with very large collection interval
        let config2 = MonitoringConfig {
            collection_interval_secs: u64::MAX,
            ..Default::default()
        };
        assert_eq!(config2.collection_interval_secs, u64::MAX);

        // Test with minimum interval (1 second)
        let config3 = MonitoringConfig {
            collection_interval_secs: 1,
            ..Default::default()
        };
        assert_eq!(config3.collection_interval_secs, 1);
    }

    #[test]
    fn test_monitoring_config_boolean_combinations() {
        // Test all boolean combinations
        let configs = vec![
            MonitoringConfig {
                enabled: true,
                performance_monitoring: true,
                health_checks: true,
                ..Default::default()
            },
            MonitoringConfig {
                enabled: true,
                performance_monitoring: true,
                health_checks: false,
                ..Default::default()
            },
            MonitoringConfig {
                enabled: true,
                performance_monitoring: false,
                health_checks: true,
                ..Default::default()
            },
            MonitoringConfig {
                enabled: true,
                performance_monitoring: false,
                health_checks: false,
                ..Default::default()
            },
            MonitoringConfig {
                enabled: false,
                performance_monitoring: true,
                health_checks: true,
                ..Default::default()
            },
            MonitoringConfig {
                enabled: false,
                performance_monitoring: false,
                health_checks: false,
                ..Default::default()
            },
        ];

        for config in configs {
            // Each configuration should be valid and serializable
            let json = serde_json::to_string(&config).unwrap();
            let recovered: MonitoringConfig = serde_json::from_str(&json).unwrap();

            assert_eq!(recovered.enabled, config.enabled);
            assert_eq!(
                recovered.performance_monitoring,
                config.performance_monitoring
            );
            assert_eq!(recovered.health_checks, config.health_checks);
        }
    }

    #[test]
    fn test_monitoring_config_json_roundtrip() {
        let configs = vec![
            MonitoringConfig::default(),
            MonitoringConfig {
                enabled: false,
                collection_interval_secs: 30,
                performance_monitoring: false,
                health_checks: true,
            },
            MonitoringConfig {
                enabled: true,
                collection_interval_secs: 3600,
                performance_monitoring: true,
                health_checks: false,
            },
        ];

        for config in configs {
            let json = serde_json::to_string(&config).unwrap();
            let recovered: MonitoringConfig = serde_json::from_str(&json).unwrap();

            assert_eq!(recovered.enabled, config.enabled);
            assert_eq!(
                recovered.collection_interval_secs,
                config.collection_interval_secs
            );
            assert_eq!(
                recovered.performance_monitoring,
                config.performance_monitoring
            );
            assert_eq!(recovered.health_checks, config.health_checks);
        }
    }

    #[test]
    fn test_monitoring_config_partial_json() {
        // Test partial JSON objects
        let test_cases = vec![
            (r#"{}"#, MonitoringConfig::default()),
            (
                r#"{"enabled": false}"#,
                MonitoringConfig {
                    enabled: false,
                    ..Default::default()
                },
            ),
            (
                r#"{"collection_interval_secs": 30}"#,
                MonitoringConfig {
                    collection_interval_secs: 30,
                    ..Default::default()
                },
            ),
            (
                r#"{"performance_monitoring": false}"#,
                MonitoringConfig {
                    performance_monitoring: false,
                    ..Default::default()
                },
            ),
            (
                r#"{"health_checks": false}"#,
                MonitoringConfig {
                    health_checks: false,
                    ..Default::default()
                },
            ),
        ];

        for (json, expected) in test_cases {
            let config: MonitoringConfig = serde_json::from_str(json).unwrap();
            assert_eq!(config.enabled, expected.enabled);
            assert_eq!(
                config.collection_interval_secs,
                expected.collection_interval_secs
            );
            assert_eq!(
                config.performance_monitoring,
                expected.performance_monitoring
            );
            assert_eq!(config.health_checks, expected.health_checks);
        }
    }

    #[test]
    fn test_monitoring_config_debug() {
        let config = MonitoringConfig::default();
        let debug_str = format!("{config:?}");

        assert!(debug_str.contains("MonitoringConfig"));
        assert!(debug_str.contains("enabled"));
        assert!(debug_str.contains("collection_interval_secs"));
        assert!(debug_str.contains("performance_monitoring"));
        assert!(debug_str.contains("health_checks"));
    }

    #[test]
    fn test_monitoring_config_send_sync() {
        // Ensure MonitoringConfig implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MonitoringConfig>();
    }

    #[test]
    fn test_collection_interval_practical_values() {
        // Test practical collection interval values
        let practical_intervals = vec![
            1,    // 1 second
            5,    // 5 seconds
            10,   // 10 seconds
            30,   // 30 seconds
            60,   // 1 minute (default)
            300,  // 5 minutes
            600,  // 10 minutes
            3600, // 1 hour
        ];

        for interval in practical_intervals {
            let config = MonitoringConfig {
                collection_interval_secs: interval,
                ..Default::default()
            };

            // Should serialize and deserialize correctly
            let json = serde_json::to_string(&config).unwrap();
            let recovered: MonitoringConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered.collection_interval_secs, interval);
        }
    }
}
