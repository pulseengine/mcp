//! Alerting and notification system for MCP servers
//!
//! This module provides:
//! - Configurable alert rules and thresholds
//! - Multiple notification channels (email, webhook, Slack, etc.)
//! - Alert de-duplication and escalation
//! - Alert history and acknowledgment
//! - Integration with metrics system

use crate::metrics::MetricsSnapshot;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Alert states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AlertState {
    Active,
    Acknowledged,
    Resolved,
    Suppressed,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique rule ID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description of what triggers this alert
    pub description: String,

    /// Metric to monitor
    pub metric: MetricType,

    /// Comparison operator
    pub operator: ComparisonOperator,

    /// Threshold value
    pub threshold: f64,

    /// Duration the condition must persist before alerting
    pub duration_secs: u64,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Enable/disable this rule
    pub enabled: bool,

    /// Notification channels to use
    pub channels: Vec<String>,

    /// Custom labels for this alert
    pub labels: HashMap<String, String>,

    /// Suppress similar alerts for this duration
    pub suppress_duration_secs: u64,
}

/// Types of metrics that can be monitored
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    ErrorRate,
    ResponseTime,
    RequestCount,
    MemoryUsage,
    CpuUsage,
    DiskUsage,
    ActiveConnections,
    HealthCheckFailures,
    Custom(String),
}

/// Comparison operators for thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert ID
    pub id: Uuid,

    /// Rule that triggered this alert
    pub rule_id: String,

    /// Alert message
    pub message: String,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Current state
    pub state: AlertState,

    /// When the alert was first triggered
    pub triggered_at: DateTime<Utc>,

    /// When the alert was last updated
    pub updated_at: DateTime<Utc>,

    /// When the alert was acknowledged (if applicable)
    pub acknowledged_at: Option<DateTime<Utc>>,

    /// Who acknowledged the alert
    pub acknowledged_by: Option<String>,

    /// When the alert was resolved (if applicable)
    pub resolved_at: Option<DateTime<Utc>>,

    /// Current metric value that triggered the alert
    pub current_value: f64,

    /// Threshold that was exceeded
    pub threshold: f64,

    /// Labels associated with this alert
    pub labels: HashMap<String, String>,

    /// Number of times this alert has been triggered
    pub trigger_count: u64,

    /// Last notification sent timestamp
    pub last_notification_at: Option<DateTime<Utc>>,
}

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotificationChannel {
    Email {
        smtp_server: String,
        smtp_port: u16,
        username: String,
        password: String,
        from_address: String,
        to_addresses: Vec<String>,
        use_tls: bool,
    },
    Webhook {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        template: String,
        timeout_secs: u64,
    },
    Slack {
        webhook_url: String,
        channel: String,
        username: Option<String>,
        icon_emoji: Option<String>,
    },
    PagerDuty {
        integration_key: String,
        service_name: String,
    },
    Console {
        use_colors: bool,
    },
}

/// Alert manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enable alerting system
    pub enabled: bool,

    /// Alert rules
    pub rules: Vec<AlertRule>,

    /// Notification channels
    pub channels: HashMap<String, NotificationChannel>,

    /// Default notification channels
    pub default_channels: Vec<String>,

    /// Alert evaluation interval in seconds
    pub evaluation_interval_secs: u64,

    /// Maximum number of active alerts to keep
    pub max_active_alerts: usize,

    /// Maximum number of resolved alerts to keep in history
    pub max_alert_history: usize,

    /// Enable alert de-duplication
    pub deduplication_enabled: bool,

    /// Re-notification interval for unacknowledged alerts
    pub renotification_interval_secs: u64,
}

/// Alert manager
pub struct AlertManager {
    config: AlertConfig,
    active_alerts: Arc<RwLock<HashMap<Uuid, Alert>>>,
    alert_history: Arc<RwLock<HashMap<Uuid, Alert>>>,
    rule_states: Arc<RwLock<HashMap<String, RuleState>>>,
    suppressed_alerts: Arc<RwLock<HashSet<String>>>,
    notification_tx: mpsc::Sender<NotificationRequest>,
    notification_rx: Arc<RwLock<mpsc::Receiver<NotificationRequest>>>,
}

/// Internal rule state tracking
#[derive(Debug, Clone)]
struct RuleState {
    condition_start: Option<DateTime<Utc>>,
    last_evaluation: DateTime<Utc>,
    consecutive_failures: u32,
}

/// Notification request
#[derive(Debug, Clone)]
struct NotificationRequest {
    alert: Alert,
    channels: Vec<String>,
    #[allow(dead_code)]
    is_resolved: bool,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(config: AlertConfig) -> Self {
        let (notification_tx, notification_rx) = mpsc::channel(1000);

        Self {
            config,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(HashMap::new())),
            rule_states: Arc::new(RwLock::new(HashMap::new())),
            suppressed_alerts: Arc::new(RwLock::new(HashSet::new())),
            notification_tx,
            notification_rx: Arc::new(RwLock::new(notification_rx)),
        }
    }

    /// Start the alert manager
    pub async fn start(&self) {
        if !self.config.enabled {
            info!("Alert manager is disabled");
            return;
        }

        info!("Starting alert manager");

        // Start evaluation loop
        let active_alerts = self.active_alerts.clone();
        let alert_history = self.alert_history.clone();
        let rule_states = self.rule_states.clone();
        let suppressed_alerts = self.suppressed_alerts.clone();
        let notification_tx = self.notification_tx.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::evaluation_loop(
                active_alerts,
                alert_history,
                rule_states,
                suppressed_alerts,
                notification_tx,
                config,
            )
            .await;
        });

        // Start notification handler
        let notification_rx = self.notification_rx.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::notification_loop(notification_rx, config).await;
        });

        // Start cleanup tasks
        self.start_cleanup_tasks().await;
    }

    /// Main evaluation loop
    async fn evaluation_loop(
        active_alerts: Arc<RwLock<HashMap<Uuid, Alert>>>,
        alert_history: Arc<RwLock<HashMap<Uuid, Alert>>>,
        rule_states: Arc<RwLock<HashMap<String, RuleState>>>,
        suppressed_alerts: Arc<RwLock<HashSet<String>>>,
        notification_tx: mpsc::Sender<NotificationRequest>,
        config: AlertConfig,
    ) {
        let mut interval =
            tokio::time::interval(Duration::from_secs(config.evaluation_interval_secs));

        loop {
            interval.tick().await;

            // Get current metrics
            let metrics = crate::metrics::get_metrics().get_metrics_snapshot().await;

            // Evaluate each rule
            for rule in &config.rules {
                if !rule.enabled {
                    continue;
                }

                if let Err(e) = Self::evaluate_rule(
                    rule,
                    &metrics,
                    &active_alerts,
                    &alert_history,
                    &rule_states,
                    &suppressed_alerts,
                    &notification_tx,
                    &config,
                )
                .await
                {
                    error!("Error evaluating rule {}: {}", rule.id, e);
                }
            }

            // Check for resolved alerts
            Self::check_resolved_alerts(&active_alerts, &alert_history, &notification_tx, &config)
                .await;

            // Send re-notifications for unacknowledged alerts
            Self::send_renotifications(&active_alerts, &notification_tx, &config).await;
        }
    }

    /// Evaluate a single rule
    #[allow(clippy::too_many_arguments)]
    async fn evaluate_rule(
        rule: &AlertRule,
        metrics: &MetricsSnapshot,
        active_alerts: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        alert_history: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        rule_states: &Arc<RwLock<HashMap<String, RuleState>>>,
        suppressed_alerts: &Arc<RwLock<HashSet<String>>>,
        notification_tx: &mpsc::Sender<NotificationRequest>,
        config: &AlertConfig,
    ) -> Result<(), AlertError> {
        let current_value = Self::extract_metric_value(rule, metrics);
        let condition_met = Self::evaluate_condition(rule, current_value);

        let mut states = rule_states.write().await;
        let rule_state = states.entry(rule.id.clone()).or_insert_with(|| RuleState {
            condition_start: None,
            last_evaluation: Utc::now(),
            consecutive_failures: 0,
        });

        rule_state.last_evaluation = Utc::now();

        if condition_met {
            rule_state.consecutive_failures += 1;

            if rule_state.condition_start.is_none() {
                rule_state.condition_start = Some(Utc::now());
            }

            // Check if condition has persisted long enough
            if let Some(start_time) = rule_state.condition_start {
                let duration = Utc::now().signed_duration_since(start_time);
                if duration.num_seconds() >= rule.duration_secs as i64 {
                    // Trigger alert
                    Self::trigger_alert(
                        rule,
                        current_value,
                        active_alerts,
                        alert_history,
                        suppressed_alerts,
                        notification_tx,
                        config,
                    )
                    .await?;
                }
            }
        } else {
            rule_state.consecutive_failures = 0;
            rule_state.condition_start = None;
        }

        Ok(())
    }

    /// Extract metric value from snapshot
    fn extract_metric_value(rule: &AlertRule, metrics: &MetricsSnapshot) -> f64 {
        match &rule.metric {
            MetricType::ErrorRate => metrics.error_metrics.error_rate_5min,
            MetricType::ResponseTime => metrics.request_metrics.avg_response_time_ms,
            MetricType::RequestCount => metrics.request_metrics.total_requests as f64,
            MetricType::MemoryUsage => metrics.health_metrics.memory_usage_mb.unwrap_or(0.0),
            MetricType::CpuUsage => metrics.health_metrics.cpu_usage_percent.unwrap_or(0.0),
            MetricType::DiskUsage => metrics.health_metrics.disk_usage_percent.unwrap_or(0.0),
            MetricType::ActiveConnections => {
                metrics.health_metrics.connection_pool_active.unwrap_or(0) as f64
            }
            MetricType::HealthCheckFailures => {
                if metrics.health_metrics.last_health_check_success {
                    0.0
                } else {
                    1.0
                }
            }
            MetricType::Custom(_) => 0.0, // TODO: Support custom metrics
        }
    }

    /// Evaluate condition against threshold
    fn evaluate_condition(rule: &AlertRule, current_value: f64) -> bool {
        match rule.operator {
            ComparisonOperator::GreaterThan => current_value > rule.threshold,
            ComparisonOperator::GreaterThanOrEqual => current_value >= rule.threshold,
            ComparisonOperator::LessThan => current_value < rule.threshold,
            ComparisonOperator::LessThanOrEqual => current_value <= rule.threshold,
            ComparisonOperator::Equal => (current_value - rule.threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (current_value - rule.threshold).abs() >= f64::EPSILON,
        }
    }

    /// Trigger an alert
    async fn trigger_alert(
        rule: &AlertRule,
        current_value: f64,
        active_alerts: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        _alert_history: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        suppressed_alerts: &Arc<RwLock<HashSet<String>>>,
        notification_tx: &mpsc::Sender<NotificationRequest>,
        config: &AlertConfig,
    ) -> Result<(), AlertError> {
        // Check if this alert is suppressed
        let suppression_key = format!("{}:{}", rule.id, rule.threshold);
        {
            let suppressed = suppressed_alerts.read().await;
            if suppressed.contains(&suppression_key) {
                return Ok(());
            }
        }

        // Create alert
        let alert = Alert {
            id: Uuid::new_v4(),
            rule_id: rule.id.clone(),
            message: Self::format_alert_message(rule, current_value),
            severity: rule.severity.clone(),
            state: AlertState::Active,
            triggered_at: Utc::now(),
            updated_at: Utc::now(),
            acknowledged_at: None,
            acknowledged_by: None,
            resolved_at: None,
            current_value,
            threshold: rule.threshold,
            labels: rule.labels.clone(),
            trigger_count: 1,
            last_notification_at: None,
        };

        // Add to active alerts
        let mut active = active_alerts.write().await;

        // Check capacity
        if active.len() >= config.max_active_alerts {
            warn!("Active alerts at capacity, removing oldest");
            if let Some(oldest_id) = active.keys().next().cloned() {
                active.remove(&oldest_id);
            }
        }

        active.insert(alert.id, alert.clone());

        // Send notification
        let channels = if rule.channels.is_empty() {
            config.default_channels.clone()
        } else {
            rule.channels.clone()
        };

        let alert_id = alert.id;
        let notification = NotificationRequest {
            alert,
            channels,
            is_resolved: false,
        };

        if let Err(e) = notification_tx.send(notification).await {
            error!("Failed to send notification: {}", e);
        }

        // Add to suppression list
        if rule.suppress_duration_secs > 0 {
            let mut suppressed = suppressed_alerts.write().await;
            suppressed.insert(suppression_key.clone());

            // Remove from suppression after duration
            let suppressed_clone = suppressed_alerts.clone();
            let suppress_duration = rule.suppress_duration_secs;
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(suppress_duration)).await;
                let mut suppressed = suppressed_clone.write().await;
                suppressed.remove(&suppression_key);
            });
        }

        info!("Alert triggered: {} ({})", rule.name, alert_id);

        Ok(())
    }

    /// Format alert message
    fn format_alert_message(rule: &AlertRule, current_value: f64) -> String {
        format!(
            "{}: {} is {} {} (current: {:.2})",
            rule.name, rule.metric, rule.operator, rule.threshold, current_value
        )
    }

    /// Check for resolved alerts
    async fn check_resolved_alerts(
        _active_alerts: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        _alert_history: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        _notification_tx: &mpsc::Sender<NotificationRequest>,
        _config: &AlertConfig,
    ) {
        // TODO: Implement resolution logic based on metrics
        // For now, this is a placeholder
    }

    /// Send re-notifications for unacknowledged alerts
    async fn send_renotifications(
        active_alerts: &Arc<RwLock<HashMap<Uuid, Alert>>>,
        notification_tx: &mpsc::Sender<NotificationRequest>,
        config: &AlertConfig,
    ) {
        let renotify_threshold =
            Utc::now() - chrono::Duration::seconds(config.renotification_interval_secs as i64);

        let active = active_alerts.read().await;
        for alert in active.values() {
            if alert.state == AlertState::Active {
                let should_renotify = if let Some(last_notif) = alert.last_notification_at {
                    last_notif < renotify_threshold
                } else {
                    alert.triggered_at < renotify_threshold
                };

                if should_renotify {
                    let notification = NotificationRequest {
                        alert: alert.clone(),
                        channels: config.default_channels.clone(),
                        is_resolved: false,
                    };

                    if let Err(e) = notification_tx.send(notification).await {
                        error!("Failed to send re-notification: {}", e);
                    }
                }
            }
        }
    }

    /// Notification processing loop
    async fn notification_loop(
        notification_rx: Arc<RwLock<mpsc::Receiver<NotificationRequest>>>,
        config: AlertConfig,
    ) {
        let mut rx = notification_rx.write().await;

        while let Some(notification) = rx.recv().await {
            for channel_id in &notification.channels {
                if let Some(channel) = config.channels.get(channel_id) {
                    if let Err(e) = Self::send_notification(channel, &notification).await {
                        error!("Failed to send notification to {}: {}", channel_id, e);
                    }
                }
            }
        }
    }

    /// Send notification to a specific channel
    async fn send_notification(
        channel: &NotificationChannel,
        notification: &NotificationRequest,
    ) -> Result<(), AlertError> {
        match channel {
            NotificationChannel::Console { use_colors } => {
                let message = if *use_colors {
                    format!("\x1b[31m[ALERT]\x1b[0m {}", notification.alert.message)
                } else {
                    format!("[ALERT] {}", notification.alert.message)
                };
                println!("{message}");
            }
            NotificationChannel::Webhook { url, .. } => {
                info!("Sending webhook notification to {}", url);
                // TODO: Implement webhook sending
            }
            NotificationChannel::Email { .. } => {
                info!("Sending email notification");
                // TODO: Implement email sending
            }
            NotificationChannel::Slack { webhook_url, .. } => {
                info!("Sending Slack notification to {}", webhook_url);
                // TODO: Implement Slack notification
            }
            NotificationChannel::PagerDuty { .. } => {
                info!("Sending PagerDuty notification");
                // TODO: Implement PagerDuty notification
            }
        }

        Ok(())
    }

    /// Start cleanup tasks
    async fn start_cleanup_tasks(&self) {
        let alert_history = self.alert_history.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Cleanup every hour

            loop {
                interval.tick().await;

                let mut history = alert_history.write().await;
                if history.len() > config.max_alert_history {
                    // Remove oldest alerts
                    let mut alerts: Vec<_> = history.values().cloned().collect();
                    alerts.sort_by(|a, b| a.triggered_at.cmp(&b.triggered_at));

                    let to_remove = alerts.len() - config.max_alert_history;
                    for alert in alerts.iter().take(to_remove) {
                        history.remove(&alert.id);
                    }
                }
            }
        });
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let active = self.active_alerts.read().await;
        active.values().cloned().collect()
    }

    /// Get alert history
    pub async fn get_alert_history(&self) -> Vec<Alert> {
        let history = self.alert_history.read().await;
        history.values().cloned().collect()
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(
        &self,
        alert_id: Uuid,
        acknowledged_by: String,
    ) -> Result<(), AlertError> {
        let mut active = self.active_alerts.write().await;

        if let Some(alert) = active.get_mut(&alert_id) {
            alert.state = AlertState::Acknowledged;
            alert.acknowledged_at = Some(Utc::now());
            alert.acknowledged_by = Some(acknowledged_by);
            alert.updated_at = Utc::now();

            info!("Alert {} acknowledged", alert_id);
            Ok(())
        } else {
            Err(AlertError::AlertNotFound(alert_id))
        }
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: Uuid) -> Result<(), AlertError> {
        let mut active = self.active_alerts.write().await;

        if let Some(mut alert) = active.remove(&alert_id) {
            alert.state = AlertState::Resolved;
            alert.resolved_at = Some(Utc::now());
            alert.updated_at = Utc::now();

            // Move to history
            let mut history = self.alert_history.write().await;
            history.insert(alert_id, alert.clone());

            // Send resolved notification
            let notification = NotificationRequest {
                alert,
                channels: self.config.default_channels.clone(),
                is_resolved: true,
            };

            if let Err(e) = self.notification_tx.send(notification).await {
                error!("Failed to send resolved notification: {}", e);
            }

            info!("Alert {} resolved", alert_id);
            Ok(())
        } else {
            Err(AlertError::AlertNotFound(alert_id))
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: vec![
                AlertRule {
                    id: "high_error_rate".to_string(),
                    name: "High Error Rate".to_string(),
                    description: "Error rate exceeds 5%".to_string(),
                    metric: MetricType::ErrorRate,
                    operator: ComparisonOperator::GreaterThan,
                    threshold: 0.05,
                    duration_secs: 300,
                    severity: AlertSeverity::High,
                    enabled: true,
                    channels: vec![],
                    labels: HashMap::new(),
                    suppress_duration_secs: 3600,
                },
                AlertRule {
                    id: "high_response_time".to_string(),
                    name: "High Response Time".to_string(),
                    description: "Average response time exceeds 5 seconds".to_string(),
                    metric: MetricType::ResponseTime,
                    operator: ComparisonOperator::GreaterThan,
                    threshold: 5000.0,
                    duration_secs: 180,
                    severity: AlertSeverity::Medium,
                    enabled: true,
                    channels: vec![],
                    labels: HashMap::new(),
                    suppress_duration_secs: 1800,
                },
            ],
            channels: {
                let mut channels = HashMap::new();
                channels.insert(
                    "console".to_string(),
                    NotificationChannel::Console { use_colors: true },
                );
                channels
            },
            default_channels: vec!["console".to_string()],
            evaluation_interval_secs: 30,
            max_active_alerts: 1000,
            max_alert_history: 10000,
            deduplication_enabled: true,
            renotification_interval_secs: 3600,
        }
    }
}

/// Display implementations for better formatting
impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::ErrorRate => write!(f, "error_rate"),
            MetricType::ResponseTime => write!(f, "response_time"),
            MetricType::RequestCount => write!(f, "request_count"),
            MetricType::MemoryUsage => write!(f, "memory_usage"),
            MetricType::CpuUsage => write!(f, "cpu_usage"),
            MetricType::DiskUsage => write!(f, "disk_usage"),
            MetricType::ActiveConnections => write!(f, "active_connections"),
            MetricType::HealthCheckFailures => write!(f, "health_check_failures"),
            MetricType::Custom(name) => write!(f, "custom_{name}"),
        }
    }
}

impl std::fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonOperator::GreaterThan => write!(f, ">"),
            ComparisonOperator::GreaterThanOrEqual => write!(f, ">="),
            ComparisonOperator::LessThan => write!(f, "<"),
            ComparisonOperator::LessThanOrEqual => write!(f, "<="),
            ComparisonOperator::Equal => write!(f, "=="),
            ComparisonOperator::NotEqual => write!(f, "!="),
        }
    }
}

/// Alert system errors
#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("Alert not found: {0}")]
    AlertNotFound(Uuid),

    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("Notification failed: {0}")]
    NotificationFailed(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_rule_creation() {
        let rule = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Rule".to_string(),
            description: "Test description".to_string(),
            metric: MetricType::ErrorRate,
            operator: ComparisonOperator::GreaterThan,
            threshold: 0.1,
            duration_secs: 300,
            severity: AlertSeverity::High,
            enabled: true,
            channels: vec!["console".to_string()],
            labels: HashMap::new(),
            suppress_duration_secs: 3600,
        };

        assert_eq!(rule.id, "test_rule");
        assert_eq!(rule.severity, AlertSeverity::High);
        assert!(rule.enabled);
    }

    #[test]
    fn test_condition_evaluation() {
        let rule = AlertRule {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            metric: MetricType::ErrorRate,
            operator: ComparisonOperator::GreaterThan,
            threshold: 0.05,
            duration_secs: 300,
            severity: AlertSeverity::High,
            enabled: true,
            channels: vec![],
            labels: HashMap::new(),
            suppress_duration_secs: 3600,
        };

        assert!(AlertManager::evaluate_condition(&rule, 0.1));
        assert!(!AlertManager::evaluate_condition(&rule, 0.01));
    }

    #[tokio::test]
    async fn test_alert_manager_creation() {
        let config = AlertConfig::default();
        let manager = AlertManager::new(config);

        let alerts = manager.get_active_alerts().await;
        assert!(alerts.is_empty());
    }
}
