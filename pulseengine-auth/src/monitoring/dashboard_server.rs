//! Security Dashboard HTTP Server
//!
//! This module provides an HTTP server for the security dashboard with
//! REST API endpoints and real-time WebSocket updates.

use crate::monitoring::{SecurityDashboard, SecurityEventType, SecurityMonitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};

/// Errors that can occur in the dashboard server
#[derive(Debug, Error)]
pub enum DashboardError {
    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Authorization failed")]
    AuthorizationFailed,

    #[error("Invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("Monitoring error: {0}")]
    MonitoringError(String),
}

/// Configuration for the dashboard server
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Server bind address
    pub bind_address: SocketAddr,

    /// Enable authentication for dashboard access
    pub enable_auth: bool,

    /// Dashboard access tokens
    pub access_tokens: Vec<String>,

    /// Enable CORS
    pub enable_cors: bool,

    /// CORS allowed origins
    pub cors_origins: Vec<String>,

    /// Enable real-time WebSocket updates
    pub enable_websocket: bool,

    /// WebSocket update interval
    pub websocket_update_interval: chrono::Duration,

    /// Maximum concurrent WebSocket connections
    pub max_websocket_connections: usize,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".parse().unwrap(),
            enable_auth: true,
            access_tokens: vec!["dashboard-token-123".to_string()],
            enable_cors: true,
            cors_origins: vec!["http://localhost:3000".to_string()],
            enable_websocket: true,
            websocket_update_interval: chrono::Duration::seconds(5),
            max_websocket_connections: 100,
        }
    }
}

/// Dashboard API request/response types
#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardRequest {
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub event_types: Option<Vec<SecurityEventType>>,
    pub user_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventsResponse {
    pub events: Vec<crate::monitoring::SecurityEvent>,
    pub total_count: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub metrics: crate::monitoring::SecurityMetrics,
    pub trends: HashMap<String, Vec<f64>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertsResponse {
    pub active_alerts: Vec<crate::monitoring::SecurityAlert>,
    pub resolved_alerts: Vec<crate::monitoring::SecurityAlert>,
    pub alert_rules: Vec<crate::monitoring::AlertRule>,
}

/// Security dashboard HTTP server
pub struct DashboardServer {
    config: DashboardConfig,
    monitor: Arc<SecurityMonitor>,
    websocket_connections: Arc<tokio::sync::RwLock<Vec<WebSocketConnection>>>,
}

impl DashboardServer {
    /// Create a new dashboard server
    pub fn new(config: DashboardConfig, monitor: Arc<SecurityMonitor>) -> Self {
        Self {
            config,
            monitor,
            websocket_connections: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Create with default configuration
    pub fn with_default_config(monitor: Arc<SecurityMonitor>) -> Self {
        Self::new(DashboardConfig::default(), monitor)
    }

    /// Start the dashboard server
    pub async fn start(&self) -> Result<(), DashboardError> {
        info!(
            "Starting security dashboard server on {}",
            self.config.bind_address
        );

        // In a real implementation, this would start an HTTP server
        // For now, we'll simulate the server functionality

        if self.config.enable_websocket {
            self.start_websocket_updates().await;
        }

        info!("Security dashboard server started successfully");
        Ok(())
    }

    /// Handle dashboard data request
    pub async fn handle_dashboard_request(
        &self,
        auth_token: Option<&str>,
    ) -> Result<SecurityDashboard, DashboardError> {
        self.authenticate_request(auth_token)?;
        Ok(self.monitor.get_dashboard_data().await)
    }

    /// Handle events request
    pub async fn handle_events_request(
        &self,
        request: DashboardRequest,
        auth_token: Option<&str>,
    ) -> Result<EventsResponse, DashboardError> {
        self.authenticate_request(auth_token)?;
        let events = if let Some(event_type) =
            request.event_types.and_then(|types| types.first().cloned())
        {
            self.monitor
                .get_events_by_type(event_type, request.start_time, request.limit)
                .await
        } else if let Some(user_id) = &request.user_id {
            self.monitor
                .get_events_by_user(user_id, request.start_time, request.limit)
                .await
        } else {
            self.monitor.get_recent_events(request.limit).await
        };

        Ok(EventsResponse {
            total_count: events.len(),
            page: 1,
            per_page: request.limit.unwrap_or(100),
            events,
        })
    }

    /// Handle metrics request
    pub async fn handle_metrics_request(
        &self,
        request: DashboardRequest,
        auth_token: Option<&str>,
    ) -> Result<MetricsResponse, DashboardError> {
        self.authenticate_request(auth_token)?;
        let end_time = request.end_time.unwrap_or_else(chrono::Utc::now);
        let start_time = request
            .start_time
            .unwrap_or_else(|| end_time - chrono::Duration::hours(24));

        let metrics = self.monitor.generate_metrics(start_time, end_time).await;

        // Generate trend data (simplified)
        let trends = self.generate_trend_data(&metrics).await;

        Ok(MetricsResponse { metrics, trends })
    }

    /// Handle alerts request
    pub async fn handle_alerts_request(
        &self,
        auth_token: Option<&str>,
    ) -> Result<AlertsResponse, DashboardError> {
        self.authenticate_request(auth_token)?;
        let active_alerts = self.monitor.get_active_alerts().await;

        // For this implementation, we'll just return active alerts
        // In a real system, you'd also fetch resolved alerts from storage
        let resolved_alerts = Vec::new();
        let alert_rules = Vec::new(); // Would fetch from monitor

        Ok(AlertsResponse {
            active_alerts,
            resolved_alerts,
            alert_rules,
        })
    }

    /// Generate HTML dashboard page
    pub fn generate_dashboard_html(&self) -> String {
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MCP Security Dashboard</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        .header {
            background: #fff;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }
        .header h1 {
            margin: 0;
            color: #333;
        }
        .dashboard-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
        }
        .card {
            background: #fff;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .card h2 {
            margin-top: 0;
            color: #333;
            font-size: 1.2em;
        }
        .metric {
            display: flex;
            justify-content: space-between;
            margin: 10px 0;
            padding: 10px;
            background: #f8f9fa;
            border-radius: 4px;
        }
        .metric-value {
            font-weight: bold;
            color: #007bff;
        }
        .alert {
            padding: 10px;
            margin: 10px 0;
            border-radius: 4px;
            border-left: 4px solid #dc3545;
            background: #f8d7da;
        }
        .alert.warning {
            border-color: #ffc107;
            background: #fff3cd;
        }
        .alert.info {
            border-color: #17a2b8;
            background: #d1ecf1;
        }
        .events-list {
            max-height: 400px;
            overflow-y: auto;
        }
        .event {
            padding: 8px;
            margin: 5px 0;
            border-radius: 4px;
            background: #f8f9fa;
            font-size: 0.9em;
        }
        .event.high {
            background: #f8d7da;
        }
        .event.medium {
            background: #fff3cd;
        }
        .refresh-btn {
            background: #007bff;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 4px;
            cursor: pointer;
            margin-left: 10px;
        }
        .refresh-btn:hover {
            background: #0056b3;
        }
        .status-indicator {
            display: inline-block;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            margin-right: 5px;
        }
        .status-ok { background-color: #28a745; }
        .status-warning { background-color: #ffc107; }
        .status-error { background-color: #dc3545; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🛡️ MCP Security Dashboard</h1>
            <p>Real-time security monitoring and alerting system</p>
            <button class="refresh-btn" onclick="refreshDashboard()">Refresh</button>
            <span id="last-updated"></span>
        </div>

        <div class="dashboard-grid">
            <div class="card">
                <h2>📊 Security Metrics (24h)</h2>
                <div id="metrics-content">
                    <div class="metric">
                        <span>Authentication Success</span>
                        <span class="metric-value" id="auth-success">--</span>
                    </div>
                    <div class="metric">
                        <span>Authentication Failures</span>
                        <span class="metric-value" id="auth-failures">--</span>
                    </div>
                    <div class="metric">
                        <span>Security Violations</span>
                        <span class="metric-value" id="violations">--</span>
                    </div>
                    <div class="metric">
                        <span>Active Sessions</span>
                        <span class="metric-value" id="active-sessions">--</span>
                    </div>
                </div>
            </div>

            <div class="card">
                <h2>🚨 Active Alerts</h2>
                <div id="alerts-content">
                    <p>No active alerts</p>
                </div>
            </div>

            <div class="card">
                <h2>📈 System Health</h2>
                <div id="health-content">
                    <div class="metric">
                        <span><span class="status-indicator status-ok"></span>Events in Memory</span>
                        <span class="metric-value" id="events-memory">--</span>
                    </div>
                    <div class="metric">
                        <span><span class="status-indicator status-ok"></span>Memory Usage</span>
                        <span class="metric-value" id="memory-usage">-- MB</span>
                    </div>
                    <div class="metric">
                        <span><span class="status-indicator status-ok"></span>Last Event</span>
                        <span class="metric-value" id="last-event">--</span>
                    </div>
                </div>
            </div>

            <div class="card">
                <h2>📝 Recent Events</h2>
                <div class="events-list" id="events-content">
                    <p>Loading events...</p>
                </div>
            </div>

            <div class="card">
                <h2>🌍 Top Source IPs</h2>
                <div id="top-ips-content">
                    <p>No data available</p>
                </div>
            </div>

            <div class="card">
                <h2>🔧 Top User Agents</h2>
                <div id="top-agents-content">
                    <p>No data available</p>
                </div>
            </div>
        </div>
    </div>

    <script>
        async function refreshDashboard() {
            try {
                // This would make actual API calls to the dashboard endpoints
                // For demo purposes, we'll show static data

                document.getElementById('auth-success').textContent = '1,234';
                document.getElementById('auth-failures').textContent = '12';
                document.getElementById('violations').textContent = '3';
                document.getElementById('active-sessions').textContent = '89';

                document.getElementById('events-memory').textContent = '2,150';
                document.getElementById('memory-usage').textContent = '15.2';
                document.getElementById('last-event').textContent = 'Just now';

                document.getElementById('last-updated').textContent =
                    'Last updated: ' + new Date().toLocaleTimeString();

                // Update events list
                const eventsHtml = `
                    <div class="event medium">
                        <strong>Auth Failure</strong> - Invalid API key from 192.168.1.100
                        <br><small>${new Date().toLocaleString()}</small>
                    </div>
                    <div class="event low">
                        <strong>Session Created</strong> - New session for user admin
                        <br><small>${new Date().toLocaleString()}</small>
                    </div>
                    <div class="event high">
                        <strong>Injection Attempt</strong> - SQL injection detected in parameters
                        <br><small>${new Date().toLocaleString()}</small>
                    </div>
                `;
                document.getElementById('events-content').innerHTML = eventsHtml;

                // Update top IPs
                const topIpsHtml = `
                    <div class="metric">
                        <span>192.168.1.100</span>
                        <span class="metric-value">45 events</span>
                    </div>
                    <div class="metric">
                        <span>10.0.0.15</span>
                        <span class="metric-value">23 events</span>
                    </div>
                    <div class="metric">
                        <span>172.16.0.5</span>
                        <span class="metric-value">12 events</span>
                    </div>
                `;
                document.getElementById('top-ips-content').innerHTML = topIpsHtml;

            } catch (error) {
                console.error('Failed to refresh dashboard:', error);
            }
        }

        // Auto-refresh every 30 seconds
        setInterval(refreshDashboard, 30000);

        // Initial load
        refreshDashboard();
    </script>
</body>
</html>
        "#.to_string()
    }

    // Private helper methods

    async fn start_websocket_updates(&self) {
        let monitor = Arc::clone(&self.monitor);
        let connections = Arc::clone(&self.websocket_connections);
        let interval = self.config.websocket_update_interval;

        tokio::spawn(async move {
            let mut update_interval = tokio::time::interval(interval.to_std().unwrap());

            loop {
                update_interval.tick().await;

                let dashboard_data = monitor.get_dashboard_data().await;
                let connections_guard = connections.read().await;

                // In a real implementation, this would send updates to WebSocket clients
                debug!(
                    "Would send WebSocket update to {} connections with {} events, {} alerts",
                    connections_guard.len(),
                    dashboard_data.recent_events.len(),
                    dashboard_data.active_alerts.len()
                );
            }
        });
    }

    async fn generate_trend_data(
        &self,
        _metrics: &crate::monitoring::SecurityMetrics,
    ) -> HashMap<String, Vec<f64>> {
        // Generate simplified trend data
        let mut trends = HashMap::new();

        // Mock trend data for demonstration
        trends.insert(
            "auth_success".to_string(),
            vec![10.0, 15.0, 12.0, 18.0, 20.0],
        );
        trends.insert("auth_failures".to_string(), vec![2.0, 3.0, 1.0, 4.0, 2.0]);
        trends.insert("violations".to_string(), vec![0.0, 1.0, 0.0, 2.0, 1.0]);

        trends
    }

    fn authenticate_request(&self, token: Option<&str>) -> Result<(), DashboardError> {
        if !self.config.enable_auth {
            return Ok(());
        }

        let provided_token = token.ok_or(DashboardError::AuthenticationFailed)?;

        // Check if the provided token is in our list of valid access tokens
        if !self
            .config
            .access_tokens
            .contains(&provided_token.to_string())
        {
            debug!(
                "Invalid dashboard access token provided: {}",
                provided_token
            );
            return Err(DashboardError::AuthenticationFailed);
        }

        debug!("Dashboard authentication successful");
        Ok(())
    }

    /// Authenticate request with Bearer token
    pub fn authenticate_bearer_token(
        &self,
        auth_header: Option<&str>,
    ) -> Result<(), DashboardError> {
        if !self.config.enable_auth {
            return Ok(());
        }

        let header = auth_header.ok_or(DashboardError::AuthenticationFailed)?;

        // Extract token from "Bearer <token>" format
        if let Some(token) = header.strip_prefix("Bearer ") {
            self.authenticate_request(Some(token))
        } else {
            Err(DashboardError::AuthenticationFailed)
        }
    }

    /// Authenticate request with API key
    pub fn authenticate_api_key(&self, api_key: Option<&str>) -> Result<(), DashboardError> {
        // For now, treat API keys the same as access tokens
        // In a production system, you might have separate API key validation
        self.authenticate_request(api_key)
    }

    /// Generate a new access token for dashboard access
    pub fn generate_access_token(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let token: String = (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..62);
                match idx {
                    0..=25 => (b'a' + idx) as char,
                    26..=51 => (b'A' + (idx - 26)) as char,
                    52..=61 => (b'0' + (idx - 52)) as char,
                    _ => unreachable!(),
                }
            })
            .collect();

        format!("dashboard_{}", token)
    }

    /// Validate token format
    #[allow(dead_code)]
    fn is_valid_token_format(&self, token: &str) -> bool {
        // Basic validation - tokens should be alphanumeric and at least 16 characters
        token.len() >= 16
            && token
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }
}

/// WebSocket connection information
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub connection_id: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_ping: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::{SecurityMonitor, SecurityMonitorConfig};

    #[tokio::test]
    async fn test_dashboard_server_creation() {
        let monitor = Arc::new(SecurityMonitor::new(SecurityMonitorConfig::default()));
        let server = DashboardServer::with_default_config(monitor);

        assert!(server.config.enable_auth);
        assert!(server.config.enable_websocket);
    }

    #[tokio::test]
    async fn test_dashboard_request_handling() {
        let monitor = Arc::new(SecurityMonitor::new(SecurityMonitorConfig::default()));
        let server = DashboardServer::with_default_config(monitor);

        // Test with valid token
        let valid_token = Some("dashboard-token-123");
        let dashboard_data = server.handle_dashboard_request(valid_token).await;
        assert!(dashboard_data.is_ok());

        // Test with invalid token should fail
        let invalid_token = Some("invalid-token");
        let dashboard_data = server.handle_dashboard_request(invalid_token).await;
        assert!(dashboard_data.is_err());
    }

    #[tokio::test]
    async fn test_events_request_handling() {
        let monitor = Arc::new(SecurityMonitor::new(SecurityMonitorConfig::default()));
        let server = DashboardServer::with_default_config(monitor);

        let request = DashboardRequest {
            start_time: None,
            end_time: None,
            event_types: None,
            user_id: None,
            limit: Some(10),
        };

        let valid_token = Some("dashboard-token-123");
        let response = server.handle_events_request(request, valid_token).await;
        assert!(response.is_ok());
    }

    #[test]
    fn test_html_generation() {
        let monitor = Arc::new(SecurityMonitor::new(SecurityMonitorConfig::default()));
        let server = DashboardServer::with_default_config(monitor);

        let html = server.generate_dashboard_html();
        assert!(html.contains("MCP Security Dashboard"));
        assert!(html.contains("Security Metrics"));
    }

    #[test]
    fn test_authentication() {
        let monitor = Arc::new(SecurityMonitor::new(SecurityMonitorConfig::default()));
        let server = DashboardServer::with_default_config(monitor);

        // Test valid token
        assert!(
            server
                .authenticate_request(Some("dashboard-token-123"))
                .is_ok()
        );

        // Test invalid token
        assert!(server.authenticate_request(Some("invalid-token")).is_err());

        // Test missing token
        assert!(server.authenticate_request(None).is_err());

        // Test Bearer token authentication
        assert!(
            server
                .authenticate_bearer_token(Some("Bearer dashboard-token-123"))
                .is_ok()
        );
        assert!(
            server
                .authenticate_bearer_token(Some("Invalid format"))
                .is_err()
        );

        // Test API key authentication
        assert!(
            server
                .authenticate_api_key(Some("dashboard-token-123"))
                .is_ok()
        );
        assert!(server.authenticate_api_key(Some("invalid-key")).is_err());
    }

    #[test]
    fn test_token_generation() {
        let monitor = Arc::new(SecurityMonitor::new(SecurityMonitorConfig::default()));
        let server = DashboardServer::with_default_config(monitor);

        let token = server.generate_access_token();
        assert!(token.starts_with("dashboard_"));
        assert!(token.len() > 16);
        assert!(server.is_valid_token_format(&token));

        // Test invalid token formats
        assert!(!server.is_valid_token_format("short"));
        assert!(!server.is_valid_token_format("contains@invalid!chars"));
    }
}
