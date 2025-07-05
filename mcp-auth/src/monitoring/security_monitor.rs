//! Security Monitoring and Dashboard System
//!
//! This module provides comprehensive security monitoring capabilities including
//! real-time metrics, alerting, threat detection, and security dashboards.

use crate::{
    security::{SecuritySeverity, SecurityViolation, SecurityViolationType},
    session::Session,
    AuthContext,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Errors that can occur during security monitoring
#[derive(Debug, Error)]
pub enum MonitoringError {
    #[error("Alert not found: {alert_id}")]
    AlertNotFound { alert_id: String },

    #[error("Metric not found: {metric_name}")]
    MetricNotFound { metric_name: String },

    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Security event types for monitoring
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Authentication events
    AuthSuccess,
    AuthFailure,
    InvalidApiKey,
    ExpiredToken,

    /// Session events
    SessionCreated,
    SessionExpired,
    SessionTerminated,
    MaxSessionsExceeded,

    /// Security violations
    InjectionAttempt,
    SizeLimit,
    RateLimit,
    UnauthorizedAccess,

    /// Permission events
    PermissionDenied,
    RoleEscalation,

    /// System events
    SystemError,
    ConfigChange,
}

/// Security event details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Unique event identifier
    pub event_id: String,

    /// Event type
    pub event_type: SecurityEventType,

    /// Event severity
    pub severity: SecuritySeverity,

    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// User/session context
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub api_key_id: Option<String>,

    /// Request context
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub method: Option<String>,

    /// Event details
    pub description: String,
    pub metadata: HashMap<String, String>,

    /// Geographic information (if available)
    pub country: Option<String>,
    pub city: Option<String>,
}

impl SecurityEvent {
    /// Create a new security event
    pub fn new(
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        description: String,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            event_type,
            severity,
            timestamp: chrono::Utc::now(),
            user_id: None,
            session_id: None,
            api_key_id: None,
            client_ip: None,
            user_agent: None,
            method: None,
            description,
            metadata: HashMap::new(),
            country: None,
            city: None,
        }
    }

    /// Add user context to event
    pub fn with_user_context(mut self, auth_context: &AuthContext) -> Self {
        self.user_id = auth_context.user_id.clone();
        self.api_key_id = auth_context.api_key_id.clone();
        self
    }

    /// Add session context to event
    pub fn with_session_context(mut self, session: &Session) -> Self {
        self.session_id = Some(session.session_id.clone());
        self.user_id = Some(session.user_id.clone());
        self.client_ip = session.client_ip.clone();
        self.user_agent = session.user_agent.clone();
        self
    }

    /// Add request context to event
    pub fn with_request_context(
        mut self,
        client_ip: Option<String>,
        user_agent: Option<String>,
        method: Option<String>,
    ) -> Self {
        self.client_ip = client_ip;
        self.user_agent = user_agent;
        self.method = method;
        self
    }

    /// Add metadata to event
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Security metrics aggregated over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Time period for these metrics
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,

    /// Authentication metrics
    pub auth_success_count: u64,
    pub auth_failure_count: u64,
    pub invalid_api_key_count: u64,
    pub expired_token_count: u64,

    /// Session metrics
    pub sessions_created: u64,
    pub sessions_expired: u64,
    pub sessions_terminated: u64,
    pub active_sessions: u64,

    /// Security violation metrics
    pub injection_attempts: u64,
    pub size_limit_violations: u64,
    pub rate_limit_violations: u64,
    pub unauthorized_access_attempts: u64,

    /// Permission metrics
    pub permission_denied_count: u64,
    pub role_escalation_attempts: u64,

    /// Top source IPs by event count
    pub top_source_ips: Vec<(String, u64)>,

    /// Top user agents by event count
    pub top_user_agents: Vec<(String, u64)>,

    /// Top methods by event count
    pub top_methods: Vec<(String, u64)>,

    /// Geographic distribution
    pub country_distribution: HashMap<String, u64>,
}

impl Default for SecurityMetrics {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            period_start: now,
            period_end: now,
            auth_success_count: 0,
            auth_failure_count: 0,
            invalid_api_key_count: 0,
            expired_token_count: 0,
            sessions_created: 0,
            sessions_expired: 0,
            sessions_terminated: 0,
            active_sessions: 0,
            injection_attempts: 0,
            size_limit_violations: 0,
            rate_limit_violations: 0,
            unauthorized_access_attempts: 0,
            permission_denied_count: 0,
            role_escalation_attempts: 0,
            top_source_ips: Vec::new(),
            top_user_agents: Vec::new(),
            top_methods: Vec::new(),
            country_distribution: HashMap::new(),
        }
    }
}

/// Security alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique alert rule identifier
    pub rule_id: String,

    /// Alert rule name
    pub name: String,

    /// Alert description
    pub description: String,

    /// Event types to monitor
    pub event_types: Vec<SecurityEventType>,

    /// Minimum severity level
    pub min_severity: SecuritySeverity,

    /// Threshold for triggering alert
    pub threshold: AlertThreshold,

    /// Time window for threshold evaluation
    pub time_window: chrono::Duration,

    /// Alert cooldown period
    pub cooldown: chrono::Duration,

    /// Whether this rule is enabled
    pub enabled: bool,

    /// Alert actions to take
    pub actions: Vec<AlertAction>,
}

/// Alert threshold configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertThreshold {
    /// Count threshold (e.g., more than 10 events)
    Count(u64),

    /// Rate threshold (e.g., more than 5 events per minute)
    Rate {
        count: u64,
        duration: chrono::Duration,
    },

    /// Percentage threshold (e.g., more than 50% failures)
    Percentage {
        numerator_events: Vec<SecurityEventType>,
        denominator_events: Vec<SecurityEventType>,
        threshold: f64,
    },
}

/// Actions to take when alert is triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertAction {
    /// Log the alert
    Log { level: String },

    /// Send email notification
    Email { recipients: Vec<String> },

    /// Send webhook notification
    Webhook {
        url: String,
        payload_template: String,
    },

    /// Block IP address
    BlockIp { duration: chrono::Duration },

    /// Disable user
    DisableUser { user_id: String },

    /// Rate limit user
    RateLimit {
        user_id: String,
        limit: u32,
        duration: chrono::Duration,
    },
}

/// Active security alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlert {
    /// Unique alert identifier
    pub alert_id: String,

    /// Alert rule that triggered this alert
    pub rule_id: String,

    /// Alert rule name
    pub rule_name: String,

    /// Alert triggered timestamp
    pub triggered_at: chrono::DateTime<chrono::Utc>,

    /// Alert resolved timestamp (if resolved)
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Alert severity
    pub severity: SecuritySeverity,

    /// Alert description
    pub description: String,

    /// Events that triggered this alert
    pub triggering_events: Vec<String>, // Event IDs

    /// Alert metadata
    pub metadata: HashMap<String, String>,

    /// Actions taken for this alert
    pub actions_taken: Vec<String>,
}

/// Configuration for security monitoring
#[derive(Debug, Clone)]
pub struct SecurityMonitorConfig {
    /// Maximum number of events to keep in memory
    pub max_events_in_memory: usize,

    /// Maximum number of alerts to keep in memory
    pub max_alerts_in_memory: usize,

    /// How long to keep events in memory
    pub event_retention: chrono::Duration,

    /// How long to keep alerts in memory
    pub alert_retention: chrono::Duration,

    /// Metrics aggregation interval
    pub metrics_interval: chrono::Duration,

    /// Enable geographic IP lookup
    pub enable_geolocation: bool,

    /// Enable real-time monitoring
    pub enable_realtime: bool,

    /// Enable alert processing
    pub enable_alerts: bool,
}

impl Default for SecurityMonitorConfig {
    fn default() -> Self {
        Self {
            max_events_in_memory: 10000,
            max_alerts_in_memory: 1000,
            event_retention: chrono::Duration::days(7),
            alert_retention: chrono::Duration::days(30),
            metrics_interval: chrono::Duration::minutes(5),
            enable_geolocation: false,
            enable_realtime: true,
            enable_alerts: true,
        }
    }
}

/// Security monitoring and dashboard system
pub struct SecurityMonitor {
    config: SecurityMonitorConfig,
    events: Arc<RwLock<VecDeque<SecurityEvent>>>,
    alerts: Arc<RwLock<Vec<SecurityAlert>>>,
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
    metrics_cache: Arc<RwLock<HashMap<String, SecurityMetrics>>>,
    last_cleanup: Arc<RwLock<chrono::DateTime<chrono::Utc>>>,
}

impl SecurityMonitor {
    /// Create a new security monitor
    pub fn new(config: SecurityMonitorConfig) -> Self {
        Self {
            config,
            events: Arc::new(RwLock::new(VecDeque::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_rules: Arc::new(RwLock::new(Vec::new())),
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
            last_cleanup: Arc::new(RwLock::new(chrono::Utc::now())),
        }
    }

    /// Create with default configuration
    pub fn with_default_config() -> Self {
        Self::new(SecurityMonitorConfig::default())
    }

    /// Record a security event
    pub async fn record_event(&self, event: SecurityEvent) {
        debug!("Recording security event: {:?}", event.event_type);

        let mut events = self.events.write().await;
        events.push_back(event.clone());

        // Enforce memory limits
        while events.len() > self.config.max_events_in_memory {
            events.pop_front();
        }

        drop(events);

        // Process alerts if enabled
        if self.config.enable_alerts {
            self.process_alerts_for_event(&event).await;
        }

        // Update real-time metrics
        if self.config.enable_realtime {
            self.update_realtime_metrics(&event).await;
        }
    }

    /// Record a security violation
    pub async fn record_violation(&self, violation: &SecurityViolation) {
        let event_type = match violation.violation_type {
            SecurityViolationType::InjectionAttempt => SecurityEventType::InjectionAttempt,
            SecurityViolationType::SizeLimit => SecurityEventType::SizeLimit,
            SecurityViolationType::RateLimit => SecurityEventType::RateLimit,
            SecurityViolationType::UnauthorizedMethod => SecurityEventType::UnauthorizedAccess,
            _ => SecurityEventType::SystemError,
        };

        let mut event = SecurityEvent::new(
            event_type,
            violation.severity.clone(),
            violation.description.clone(),
        );

        if let Some(field) = &violation.field {
            event = event.with_metadata("field".to_string(), field.clone());
        }

        if let Some(value) = &violation.value {
            event = event.with_metadata("value".to_string(), value.clone());
        }

        self.record_event(event).await;
    }

    /// Record authentication event
    pub async fn record_auth_event(
        &self,
        event_type: SecurityEventType,
        auth_context: Option<&AuthContext>,
        client_ip: Option<String>,
        user_agent: Option<String>,
        description: String,
    ) {
        let severity = match event_type {
            SecurityEventType::AuthFailure | SecurityEventType::InvalidApiKey => {
                SecuritySeverity::Medium
            }
            SecurityEventType::ExpiredToken => SecuritySeverity::Low,
            SecurityEventType::AuthSuccess => SecuritySeverity::Low,
            _ => SecuritySeverity::Medium,
        };

        let mut event = SecurityEvent::new(event_type, severity, description)
            .with_request_context(client_ip, user_agent, None);

        if let Some(auth) = auth_context {
            event = event.with_user_context(auth);
        }

        self.record_event(event).await;
    }

    /// Record session event
    pub async fn record_session_event(
        &self,
        event_type: SecurityEventType,
        session: &Session,
        description: String,
    ) {
        let severity = match event_type {
            SecurityEventType::MaxSessionsExceeded => SecuritySeverity::High,
            SecurityEventType::SessionExpired => SecuritySeverity::Low,
            _ => SecuritySeverity::Low,
        };

        let event =
            SecurityEvent::new(event_type, severity, description).with_session_context(session);

        self.record_event(event).await;
    }

    /// Get recent security events
    pub async fn get_recent_events(&self, limit: Option<usize>) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        let limit = limit.unwrap_or(100);

        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get events by type
    pub async fn get_events_by_type(
        &self,
        event_type: SecurityEventType,
        since: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<usize>,
    ) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        let since = since.unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::hours(24));
        let limit = limit.unwrap_or(1000);

        events
            .iter()
            .filter(|e| e.event_type == event_type && e.timestamp >= since)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get events by user
    pub async fn get_events_by_user(
        &self,
        user_id: &str,
        since: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<usize>,
    ) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        let since = since.unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::hours(24));
        let limit = limit.unwrap_or(1000);

        events
            .iter()
            .filter(|e| {
                e.user_id.as_ref().map(|u| u == user_id).unwrap_or(false) && e.timestamp >= since
            })
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Generate security metrics for a time period
    pub async fn generate_metrics(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> SecurityMetrics {
        let events = self.events.read().await;
        let mut metrics = SecurityMetrics {
            period_start: start,
            period_end: end,
            ..Default::default()
        };

        let mut ip_counts = HashMap::new();
        let mut user_agent_counts = HashMap::new();
        let mut method_counts = HashMap::new();

        for event in events.iter() {
            if event.timestamp >= start && event.timestamp <= end {
                // Count by event type
                match event.event_type {
                    SecurityEventType::AuthSuccess => metrics.auth_success_count += 1,
                    SecurityEventType::AuthFailure => metrics.auth_failure_count += 1,
                    SecurityEventType::InvalidApiKey => metrics.invalid_api_key_count += 1,
                    SecurityEventType::ExpiredToken => metrics.expired_token_count += 1,
                    SecurityEventType::SessionCreated => metrics.sessions_created += 1,
                    SecurityEventType::SessionExpired => metrics.sessions_expired += 1,
                    SecurityEventType::SessionTerminated => metrics.sessions_terminated += 1,
                    SecurityEventType::InjectionAttempt => metrics.injection_attempts += 1,
                    SecurityEventType::SizeLimit => metrics.size_limit_violations += 1,
                    SecurityEventType::RateLimit => metrics.rate_limit_violations += 1,
                    SecurityEventType::UnauthorizedAccess => {
                        metrics.unauthorized_access_attempts += 1
                    }
                    SecurityEventType::PermissionDenied => metrics.permission_denied_count += 1,
                    SecurityEventType::RoleEscalation => metrics.role_escalation_attempts += 1,
                    _ => {}
                }

                // Aggregate IP addresses
                if let Some(ip) = &event.client_ip {
                    *ip_counts.entry(ip.clone()).or_insert(0) += 1;
                }

                // Aggregate user agents
                if let Some(ua) = &event.user_agent {
                    *user_agent_counts.entry(ua.clone()).or_insert(0) += 1;
                }

                // Aggregate methods
                if let Some(method) = &event.method {
                    *method_counts.entry(method.clone()).or_insert(0) += 1;
                }

                // Aggregate countries
                if let Some(country) = &event.country {
                    *metrics
                        .country_distribution
                        .entry(country.clone())
                        .or_insert(0) += 1;
                }
            }
        }

        // Sort and take top items
        metrics.top_source_ips = Self::top_items(ip_counts, 10);
        metrics.top_user_agents = Self::top_items(user_agent_counts, 10);
        metrics.top_methods = Self::top_items(method_counts, 10);

        metrics
    }

    /// Get current security dashboard data
    pub async fn get_dashboard_data(&self) -> SecurityDashboard {
        let now = chrono::Utc::now();
        let hour_ago = now - chrono::Duration::hours(1);
        let day_ago = now - chrono::Duration::days(1);

        let hourly_metrics = self.generate_metrics(hour_ago, now).await;
        let daily_metrics = self.generate_metrics(day_ago, now).await;
        let recent_events = self.get_recent_events(Some(50)).await;
        let active_alerts = self.get_active_alerts().await;

        SecurityDashboard {
            timestamp: now,
            hourly_metrics,
            daily_metrics,
            recent_events,
            active_alerts,
            system_health: self.get_system_health().await,
        }
    }

    /// Add alert rule
    pub async fn add_alert_rule(&self, rule: AlertRule) {
        let mut rules = self.alert_rules.write().await;
        rules.push(rule);
        info!("Added new alert rule");
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<SecurityAlert> {
        let alerts = self.alerts.read().await;
        alerts
            .iter()
            .filter(|a| a.resolved_at.is_none())
            .cloned()
            .collect()
    }

    /// Resolve alert
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<(), MonitoringError> {
        let mut alerts = self.alerts.write().await;

        if let Some(alert) = alerts.iter_mut().find(|a| a.alert_id == alert_id) {
            alert.resolved_at = Some(chrono::Utc::now());
            info!("Resolved alert: {}", alert_id);
            Ok(())
        } else {
            Err(MonitoringError::AlertNotFound {
                alert_id: alert_id.to_string(),
            })
        }
    }

    /// Start background monitoring tasks
    pub async fn start_background_tasks(&self) -> tokio::task::JoinHandle<()> {
        let monitor = self.clone();

        tokio::spawn(async move {
            let mut cleanup_interval =
                tokio::time::interval(chrono::Duration::hours(1).to_std().unwrap());
            let mut metrics_interval =
                tokio::time::interval(monitor.config.metrics_interval.to_std().unwrap());

            loop {
                tokio::select! {
                    _ = cleanup_interval.tick() => {
                        if let Err(e) = monitor.cleanup_old_data().await {
                            error!("Failed to cleanup old monitoring data: {}", e);
                        }
                    }
                    _ = metrics_interval.tick() => {
                        if let Err(e) = monitor.update_metrics_cache().await {
                            error!("Failed to update metrics cache: {}", e);
                        }
                    }
                }
            }
        })
    }

    // Helper methods

    fn top_items(mut counts: HashMap<String, u64>, limit: usize) -> Vec<(String, u64)> {
        let mut items: Vec<(String, u64)> = counts.drain().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(limit);
        items
    }

    async fn process_alerts_for_event(&self, event: &SecurityEvent) {
        let rules = self.alert_rules.read().await;

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            if rule.event_types.contains(&event.event_type) && event.severity >= rule.min_severity {
                // Check if threshold is met
                if self.check_alert_threshold(rule, event).await {
                    self.trigger_alert(rule, event).await;
                }
            }
        }
    }

    async fn check_alert_threshold(&self, rule: &AlertRule, _event: &SecurityEvent) -> bool {
        let now = chrono::Utc::now();
        let window_start = now - rule.time_window;

        let events = self.events.read().await;
        let relevant_events: Vec<&SecurityEvent> = events
            .iter()
            .filter(|e| {
                e.timestamp >= window_start
                    && rule.event_types.contains(&e.event_type)
                    && e.severity >= rule.min_severity
            })
            .collect();

        match &rule.threshold {
            AlertThreshold::Count(threshold) => relevant_events.len() as u64 >= *threshold,
            AlertThreshold::Rate { count, duration: _ } => relevant_events.len() as u64 >= *count,
            AlertThreshold::Percentage {
                numerator_events,
                denominator_events,
                threshold,
            } => {
                let numerator = relevant_events
                    .iter()
                    .filter(|e| numerator_events.contains(&e.event_type))
                    .count() as f64;

                let denominator = relevant_events
                    .iter()
                    .filter(|e| denominator_events.contains(&e.event_type))
                    .count() as f64;

                if denominator > 0.0 {
                    (numerator / denominator) * 100.0 >= *threshold
                } else {
                    false
                }
            }
        }
    }

    async fn trigger_alert(&self, rule: &AlertRule, event: &SecurityEvent) {
        let alert = SecurityAlert {
            alert_id: Uuid::new_v4().to_string(),
            rule_id: rule.rule_id.clone(),
            rule_name: rule.name.clone(),
            triggered_at: chrono::Utc::now(),
            resolved_at: None,
            severity: event.severity.clone(),
            description: format!("Alert triggered: {}", rule.description),
            triggering_events: vec![event.event_id.clone()],
            metadata: HashMap::new(),
            actions_taken: Vec::new(),
        };

        warn!(
            "Security alert triggered: {} - {}",
            alert.rule_name, alert.description
        );

        let mut alerts = self.alerts.write().await;
        alerts.push(alert);

        // Enforce memory limits
        while alerts.len() > self.config.max_alerts_in_memory {
            alerts.remove(0);
        }
    }

    async fn update_realtime_metrics(&self, _event: &SecurityEvent) {
        // Update real-time metrics cache
        // This would typically update counters, rates, etc.
        debug!("Updated real-time metrics");
    }

    async fn cleanup_old_data(&self) -> Result<(), MonitoringError> {
        let now = chrono::Utc::now();
        let event_cutoff = now - self.config.event_retention;
        let alert_cutoff = now - self.config.alert_retention;

        // Cleanup old events
        let mut events = self.events.write().await;
        let original_count = events.len();
        events.retain(|e| e.timestamp >= event_cutoff);
        let events_removed = original_count - events.len();

        drop(events);

        // Cleanup old alerts
        let mut alerts = self.alerts.write().await;
        let original_alert_count = alerts.len();
        alerts.retain(|a| a.triggered_at >= alert_cutoff);
        let alerts_removed = original_alert_count - alerts.len();

        if events_removed > 0 || alerts_removed > 0 {
            info!(
                "Cleaned up {} old events and {} old alerts",
                events_removed, alerts_removed
            );
        }

        Ok(())
    }

    async fn update_metrics_cache(&self) -> Result<(), MonitoringError> {
        let now = chrono::Utc::now();
        let hour_ago = now - chrono::Duration::hours(1);

        let metrics = self.generate_metrics(hour_ago, now).await;

        let mut cache = self.metrics_cache.write().await;
        cache.insert("hourly".to_string(), metrics);

        // Keep only recent metrics in cache
        let day_ago = now - chrono::Duration::days(1);
        cache.retain(|_, metrics| metrics.period_start >= day_ago);

        Ok(())
    }

    async fn get_system_health(&self) -> SystemHealth {
        let events = self.events.read().await;
        let alerts = self.alerts.read().await;

        SystemHealth {
            events_in_memory: events.len(),
            active_alerts: alerts.iter().filter(|a| a.resolved_at.is_none()).count(),
            last_event_time: events.back().map(|e| e.timestamp),
            memory_usage_mb: self.estimate_memory_usage().await,
        }
    }

    async fn estimate_memory_usage(&self) -> u64 {
        // Rough estimate of memory usage in MB
        let events = self.events.read().await;
        let alerts = self.alerts.read().await;

        let event_size_estimate = events.len() * 1024; // ~1KB per event
        let alert_size_estimate = alerts.len() * 512; // ~512B per alert

        ((event_size_estimate + alert_size_estimate) / 1024 / 1024) as u64
    }
}

impl Clone for SecurityMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            events: Arc::clone(&self.events),
            alerts: Arc::clone(&self.alerts),
            alert_rules: Arc::clone(&self.alert_rules),
            metrics_cache: Arc::clone(&self.metrics_cache),
            last_cleanup: Arc::clone(&self.last_cleanup),
        }
    }
}

/// Security dashboard data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityDashboard {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub hourly_metrics: SecurityMetrics,
    pub daily_metrics: SecurityMetrics,
    pub recent_events: Vec<SecurityEvent>,
    pub active_alerts: Vec<SecurityAlert>,
    pub system_health: SystemHealth,
}

/// System health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub events_in_memory: usize,
    pub active_alerts: usize,
    pub last_event_time: Option<chrono::DateTime<chrono::Utc>>,
    pub memory_usage_mb: u64,
}

/// Helper function to create default alert rules
pub fn create_default_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            rule_id: "high_auth_failures".to_string(),
            name: "High Authentication Failures".to_string(),
            description: "Multiple authentication failures detected".to_string(),
            event_types: vec![
                SecurityEventType::AuthFailure,
                SecurityEventType::InvalidApiKey,
            ],
            min_severity: SecuritySeverity::Medium,
            threshold: AlertThreshold::Count(10),
            time_window: chrono::Duration::minutes(5),
            cooldown: chrono::Duration::minutes(15),
            enabled: true,
            actions: vec![AlertAction::Log {
                level: "warn".to_string(),
            }],
        },
        AlertRule {
            rule_id: "injection_attempts".to_string(),
            name: "Injection Attempts".to_string(),
            description: "Potential injection attacks detected".to_string(),
            event_types: vec![SecurityEventType::InjectionAttempt],
            min_severity: SecuritySeverity::High,
            threshold: AlertThreshold::Count(3),
            time_window: chrono::Duration::minutes(10),
            cooldown: chrono::Duration::minutes(30),
            enabled: true,
            actions: vec![
                AlertAction::Log {
                    level: "error".to_string(),
                },
                AlertAction::BlockIp {
                    duration: chrono::Duration::hours(1),
                },
            ],
        },
        AlertRule {
            rule_id: "rate_limit_violations".to_string(),
            name: "Rate Limit Violations".to_string(),
            description: "Excessive rate limit violations".to_string(),
            event_types: vec![SecurityEventType::RateLimit],
            min_severity: SecuritySeverity::Medium,
            threshold: AlertThreshold::Count(20),
            time_window: chrono::Duration::minutes(5),
            cooldown: chrono::Duration::minutes(10),
            enabled: true,
            actions: vec![AlertAction::Log {
                level: "warn".to_string(),
            }],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_monitor_creation() {
        let monitor = SecurityMonitor::with_default_config();

        // Test that monitor was created successfully
        assert!(monitor.config.enable_realtime);
        assert!(monitor.config.enable_alerts);
    }

    #[tokio::test]
    async fn test_event_recording() {
        let monitor = SecurityMonitor::with_default_config();

        let event = SecurityEvent::new(
            SecurityEventType::AuthFailure,
            SecuritySeverity::Medium,
            "Test authentication failure".to_string(),
        );

        monitor.record_event(event).await;

        let events = monitor.get_recent_events(Some(10)).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, SecurityEventType::AuthFailure);
    }

    #[tokio::test]
    async fn test_metrics_generation() {
        let monitor = SecurityMonitor::with_default_config();

        // Record some test events
        monitor
            .record_event(SecurityEvent::new(
                SecurityEventType::AuthSuccess,
                SecuritySeverity::Low,
                "Success".to_string(),
            ))
            .await;

        monitor
            .record_event(SecurityEvent::new(
                SecurityEventType::AuthFailure,
                SecuritySeverity::Medium,
                "Failure".to_string(),
            ))
            .await;

        let now = chrono::Utc::now();
        let hour_ago = now - chrono::Duration::hours(1);

        let metrics = monitor.generate_metrics(hour_ago, now).await;

        assert_eq!(metrics.auth_success_count, 1);
        assert_eq!(metrics.auth_failure_count, 1);
    }

    #[tokio::test]
    async fn test_alert_rules() {
        let monitor = SecurityMonitor::with_default_config();

        let rule = AlertRule {
            rule_id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "Test alert rule".to_string(),
            event_types: vec![SecurityEventType::AuthFailure],
            min_severity: SecuritySeverity::Medium,
            threshold: AlertThreshold::Count(1),
            time_window: chrono::Duration::minutes(5),
            cooldown: chrono::Duration::minutes(1),
            enabled: true,
            actions: vec![AlertAction::Log {
                level: "warn".to_string(),
            }],
        };

        monitor.add_alert_rule(rule).await;

        // Record an event that should trigger the alert
        monitor
            .record_event(SecurityEvent::new(
                SecurityEventType::AuthFailure,
                SecuritySeverity::Medium,
                "Test failure".to_string(),
            ))
            .await;

        // Give some time for alert processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let active_alerts = monitor.get_active_alerts().await;
        assert!(!active_alerts.is_empty());
    }

    #[tokio::test]
    async fn test_dashboard_data() {
        let monitor = SecurityMonitor::with_default_config();

        // Record some events
        monitor
            .record_event(SecurityEvent::new(
                SecurityEventType::SessionCreated,
                SecuritySeverity::Low,
                "Session created".to_string(),
            ))
            .await;

        let dashboard = monitor.get_dashboard_data().await;

        assert!(!dashboard.recent_events.is_empty());
        assert_eq!(dashboard.hourly_metrics.sessions_created, 1);
    }
}
