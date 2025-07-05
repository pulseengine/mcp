//! Security Monitoring and Dashboard Module
//!
//! This module provides comprehensive security monitoring capabilities including
//! real-time metrics, alerting, and dashboard functionality.

pub mod dashboard_server;
pub mod security_monitor;

pub use security_monitor::{
    create_default_alert_rules, AlertAction, AlertRule, AlertThreshold, MonitoringError,
    SecurityAlert, SecurityDashboard, SecurityEvent, SecurityEventType, SecurityMetrics,
    SecurityMonitor, SecurityMonitorConfig, SystemHealth,
};
