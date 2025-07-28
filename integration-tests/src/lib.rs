//! Integration tests for the PulseEngine MCP framework
//!
//! This crate contains integration tests that verify the interaction between
//! different MCP framework components working together as a complete system.

#![allow(unused_imports)] // Allow unused imports in integration tests
#![allow(clippy::uninlined_format_args)] // Allow traditional format strings in tests

pub mod auth_server_integration;
pub mod cli_server_integration;
pub mod end_to_end_scenarios;
pub mod monitoring_integration;
pub mod transport_server_integration;

/// Common test utilities for integration tests
pub mod test_utils {
    use pulseengine_mcp_auth::{AuthConfig, config::StorageConfig};
    use pulseengine_mcp_monitoring::MonitoringConfig;
    use pulseengine_mcp_security::SecurityConfig;
    use std::time::Duration;

    /// Create a test-friendly auth config with memory storage
    pub fn test_auth_config() -> AuthConfig {
        AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false, // Disabled by default for tests
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 3,
            rate_limit_window_secs: 60,
        }
    }

    /// Create a test-friendly monitoring config
    pub fn test_monitoring_config() -> MonitoringConfig {
        MonitoringConfig {
            enabled: true,
            collection_interval_secs: 1, // Fast collection for tests
            performance_monitoring: true,
            health_checks: true,
        }
    }

    /// Create a test-friendly security config
    pub fn test_security_config() -> SecurityConfig {
        SecurityConfig {
            validate_requests: true,
            rate_limiting: true,
            max_requests_per_minute: 1000, // High limit for tests
            cors_enabled: true,
            cors_origins: vec!["http://localhost:3000".to_string()],
        }
    }

    /// Wait for a condition with timeout
    pub async fn wait_for_condition<F, Fut>(
        mut condition: F,
        timeout_duration: Duration,
        check_interval: Duration,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout_duration {
            if condition().await {
                return Ok(());
            }
            tokio::time::sleep(check_interval).await;
        }
        Err("Condition timeout".into())
    }
}
