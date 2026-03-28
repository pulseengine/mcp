//! Security Monitoring and Dashboard Module
//!
//! This module provides comprehensive security monitoring capabilities including
//! real-time metrics, alerting, and dashboard functionality.

pub mod dashboard_server;
pub mod security_monitor;

pub use security_monitor::{
    AlertAction, AlertRule, AlertThreshold, MonitoringError, SecurityAlert, SecurityDashboard,
    SecurityEvent, SecurityEventType, SecurityMetrics, SecurityMonitor, SecurityMonitorConfig,
    SystemHealth, create_default_alert_rules,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecuritySeverity;

    #[test]
    fn test_monitoring_module_exports() {
        // Test that all monitoring types are accessible

        let config = SecurityMonitorConfig::default();
        assert!(config.max_events_in_memory > 0); // Should be accessible

        let event = SecurityEvent::new(
            SecurityEventType::AuthSuccess,
            SecuritySeverity::Low,
            "Test event".to_string(),
        );

        assert_eq!(event.event_type, SecurityEventType::AuthSuccess);
        assert_eq!(event.severity, SecuritySeverity::Low);
        assert_eq!(event.description, "Test event");
        assert!(!event.event_id.is_empty());

        let threshold = AlertThreshold::Count(10);
        assert!(matches!(threshold, AlertThreshold::Count(10)));

        let action = AlertAction::Log {
            level: "info".to_string(),
        };
        assert!(matches!(action, AlertAction::Log { level: _ }));
    }

    #[test]
    fn test_security_event_types() {
        let event_types = vec![
            SecurityEventType::AuthSuccess,
            SecurityEventType::AuthFailure,
            SecurityEventType::PermissionDenied,
            SecurityEventType::RateLimit,
            SecurityEventType::SessionCreated,
            SecurityEventType::SessionExpired,
            SecurityEventType::InjectionAttempt,
            SecurityEventType::ConfigChange,
        ];

        for event_type in event_types {
            let event = SecurityEvent::new(
                event_type.clone(),
                SecuritySeverity::Medium,
                format!("Test {:?}", event_type),
            );

            assert_eq!(event.event_type, event_type);
            assert!(!event.description.is_empty());
            assert!(event.timestamp <= chrono::Utc::now());
        }
    }

    #[test]
    fn test_alert_thresholds() {
        let thresholds = vec![
            AlertThreshold::Count(5),
            AlertThreshold::Rate {
                count: 10,
                duration: chrono::Duration::minutes(5),
            },
            AlertThreshold::Percentage {
                numerator_events: vec![SecurityEventType::AuthFailure],
                denominator_events: vec![
                    SecurityEventType::AuthSuccess,
                    SecurityEventType::AuthFailure,
                ],
                threshold: 50.0,
            },
        ];

        for threshold in thresholds {
            match threshold {
                AlertThreshold::Count(count) => assert!(count > 0),
                AlertThreshold::Rate { count, duration } => {
                    assert!(count > 0);
                    assert!(duration > chrono::Duration::zero());
                }
                AlertThreshold::Percentage {
                    threshold: percentage,
                    ..
                } => {
                    assert!((0.0..=100.0).contains(&percentage));
                }
            }
        }
    }

    #[test]
    fn test_alert_actions() {
        let actions = vec![
            AlertAction::Log {
                level: "warn".to_string(),
            },
            AlertAction::Email {
                recipients: vec!["admin@example.com".to_string()],
            },
            AlertAction::Webhook {
                url: "https://example.com/webhook".to_string(),
                payload_template: "{}".to_string(),
            },
            AlertAction::BlockIp {
                duration: chrono::Duration::hours(1),
            },
        ];

        for action in actions {
            match action {
                AlertAction::Log { level } => assert!(!level.is_empty()),
                AlertAction::Email { recipients } => assert!(!recipients.is_empty()),
                AlertAction::Webhook {
                    url,
                    payload_template,
                } => {
                    assert!(!url.is_empty());
                    assert!(!payload_template.is_empty());
                }
                AlertAction::BlockIp { duration } => assert!(duration > chrono::Duration::zero()),
                _ => {} // Other variants are valid
            }
        }
    }

    #[tokio::test]
    async fn test_security_monitor_creation() {
        let config = SecurityMonitorConfig {
            max_events_in_memory: 1000,
            enable_realtime: true,
            enable_alerts: false,
            ..Default::default()
        };

        let monitor = SecurityMonitor::new(config);

        // Test basic event recording
        let event = SecurityEvent::new(
            SecurityEventType::AuthSuccess,
            SecuritySeverity::Low,
            "Test authentication success".to_string(),
        );

        monitor.record_event(event).await;
        // Should not panic or error
    }

    #[test]
    fn test_default_alert_rules() {
        let rules = create_default_alert_rules();
        assert!(!rules.is_empty());

        for rule in rules {
            assert!(!rule.name.is_empty());
            assert!(!rule.description.is_empty());
            assert!(!rule.actions.is_empty());

            // Verify threshold is reasonable
            match rule.threshold {
                AlertThreshold::Count(count) => assert!(count > 0 && count < 1000),
                AlertThreshold::Rate { count, duration } => {
                    assert!(count > 0 && count < 1000);
                    assert!(duration >= chrono::Duration::minutes(1));
                    assert!(duration <= chrono::Duration::hours(24));
                }
                AlertThreshold::Percentage {
                    threshold: percentage,
                    ..
                } => {
                    assert!((0.0..=100.0).contains(&percentage));
                }
            }
        }
    }

    #[test]
    fn test_security_metrics() {
        let now = chrono::Utc::now();
        let metrics = SecurityMetrics {
            period_start: now - chrono::Duration::hours(1),
            period_end: now,
            auth_success_count: 80,
            auth_failure_count: 20,
            invalid_api_key_count: 5,
            expired_token_count: 3,
            sessions_created: 15,
            sessions_expired: 2,
            sessions_terminated: 1,
            active_sessions: 25,
            injection_attempts: 0,
            size_limit_violations: 1,
            rate_limit_violations: 5,
            unauthorized_access_attempts: 2,
            permission_denied_count: 3,
            role_escalation_attempts: 0,
            top_source_ips: vec![("192.168.1.1".to_string(), 50)],
            top_user_agents: vec![("Mozilla/5.0".to_string(), 40)],
            top_methods: vec![("POST".to_string(), 60)],
            country_distribution: std::collections::HashMap::new(),
        };

        assert_eq!(metrics.auth_success_count, 80);
        assert_eq!(metrics.auth_failure_count, 20);
        assert_eq!(metrics.active_sessions, 25);
        assert_eq!(metrics.sessions_created, 15);
    }

    #[test]
    fn test_system_health() {
        let health = SystemHealth {
            events_in_memory: 1500,
            active_alerts: 2,
            last_event_time: Some(chrono::Utc::now()),
            memory_usage_mb: 512,
        };

        assert_eq!(health.events_in_memory, 1500);
        assert_eq!(health.active_alerts, 2);
        assert!(health.last_event_time.is_some());
        assert_eq!(health.memory_usage_mb, 512);
    }

    #[test]
    fn test_monitoring_error_types() {
        let errors = vec![
            MonitoringError::AlertNotFound {
                alert_id: "test-alert".to_string(),
            },
            MonitoringError::MetricNotFound {
                metric_name: "test-metric".to_string(),
            },
            MonitoringError::ConfigError {
                reason: "test config error".to_string(),
            },
            MonitoringError::StorageError("test storage error".to_string()),
            MonitoringError::SerializationError("test serialization error".to_string()),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            assert!(error_string.len() > 5);
        }
    }

    #[tokio::test]
    async fn test_security_dashboard_integration() {
        let config = SecurityMonitorConfig {
            max_events_in_memory: 1000,
            enable_realtime: true,
            enable_alerts: true,
            ..Default::default()
        };

        let monitor = SecurityMonitor::new(config);

        // Record some events
        let events = vec![
            SecurityEvent::new(
                SecurityEventType::AuthSuccess,
                SecuritySeverity::Low,
                "Auth 1".to_string(),
            ),
            SecurityEvent::new(
                SecurityEventType::AuthSuccess,
                SecuritySeverity::Low,
                "Auth 2".to_string(),
            ),
            SecurityEvent::new(
                SecurityEventType::AuthFailure,
                SecuritySeverity::Medium,
                "Failed auth".to_string(),
            ),
            SecurityEvent::new(
                SecurityEventType::RateLimit,
                SecuritySeverity::High,
                "Rate limit".to_string(),
            ),
        ];

        for event in events {
            monitor.record_event(event).await;
        }

        // Get dashboard data
        let dashboard_data = monitor.get_dashboard_data().await;

        // Verify dashboard contains expected data
        assert!(dashboard_data.hourly_metrics.auth_success_count >= 2);
        assert!(dashboard_data.hourly_metrics.auth_failure_count >= 1);
        assert!(dashboard_data.hourly_metrics.rate_limit_violations >= 1);

        // System health should be populated
        assert!(dashboard_data.system_health.events_in_memory >= 4);
        // Memory usage is u64, so always >= 0 - remove redundant check
    }

    #[test]
    fn test_monitoring_config_defaults() {
        let config = SecurityMonitorConfig::default();

        // Defaults should be reasonable
        assert!(config.max_events_in_memory > 0);
        assert!(config.max_alerts_in_memory > 0);
        assert!(config.event_retention > chrono::Duration::zero());
        assert!(config.alert_retention > chrono::Duration::zero());
        assert!(config.metrics_interval > chrono::Duration::zero());
    }
}
