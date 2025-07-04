//! Security Monitoring and Dashboard Module
//!
//! This module provides comprehensive security monitoring capabilities including
//! real-time metrics, alerting, and dashboard functionality.

pub mod security_monitor;
pub mod dashboard_server;

pub use security_monitor::{
    SecurityMonitor, SecurityEvent, SecurityEventType, SecurityMetrics, SecurityAlert,
    AlertRule, AlertThreshold, AlertAction, SecurityDashboard, SystemHealth,
    SecurityMonitorConfig, MonitoringError, create_default_alert_rules
};