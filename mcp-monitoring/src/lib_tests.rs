//! Comprehensive unit tests for mcp-monitoring lib module

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_default_config() {
        let config = default_config();

        // Verify all default values match MonitoringConfig::default()
        let expected = MonitoringConfig::default();
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

    #[test]
    fn test_default_config_consistency() {
        let config1 = default_config();
        let config2 = default_config();

        // Should return consistent defaults
        assert_eq!(config1.enabled, config2.enabled);
        assert_eq!(
            config1.collection_interval_secs,
            config2.collection_interval_secs
        );
        assert_eq!(
            config1.performance_monitoring,
            config2.performance_monitoring
        );
        assert_eq!(config1.health_checks, config2.health_checks);
    }

    #[test]
    fn test_reexports() {
        // Test that all public types are properly re-exported
        let _config = MonitoringConfig::default();
        let _collector = MetricsCollector::new(MonitoringConfig::default());
        let _metrics = ServerMetrics::default();
    }

    #[test]
    fn test_module_visibility() {
        // Test that modules are publicly accessible
        use crate::{collector, config, metrics};

        // Should be able to access module items
        let _ = config::MonitoringConfig::default();
        let _ = collector::MetricsCollector::new(config::MonitoringConfig::default());
        let _ = metrics::ServerMetrics::default();
    }

    #[test]
    fn test_default_config_values() {
        let config = default_config();

        // Test specific expected default values
        assert!(config.enabled);
        assert_eq!(config.collection_interval_secs, 60);
        assert!(config.performance_monitoring);
        assert!(config.health_checks);
    }
}
