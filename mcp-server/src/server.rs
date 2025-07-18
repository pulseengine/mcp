//! Generic MCP server implementation

use crate::{backend::McpBackend, handler::GenericServerHandler, middleware::MiddlewareStack};
use pulseengine_mcp_auth::{AuthConfig, AuthenticationManager};
use pulseengine_mcp_logging::{
    AlertConfig, AlertManager, DashboardConfig, DashboardManager, PerformanceProfiler,
    PersistenceConfig, ProfilingConfig, SanitizationConfig, StructuredLogger, TelemetryConfig,
    TelemetryManager,
};
use pulseengine_mcp_monitoring::{MetricsCollector, MonitoringConfig};
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_security::{SecurityConfig, SecurityMiddleware};
use pulseengine_mcp_transport::{Transport, TransportConfig};

use std::sync::Arc;
use thiserror::Error;
use tokio::signal;
use tracing::{error, info, warn};

/// Error type for server operations
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Server configuration error: {0}")]
    Configuration(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Server already running")]
    AlreadyRunning,

    #[error("Server not running")]
    NotRunning,

    #[error("Shutdown timeout")]
    ShutdownTimeout,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server implementation information
    pub server_info: ServerInfo,

    /// Authentication configuration
    pub auth_config: AuthConfig,

    /// Transport configuration
    pub transport_config: TransportConfig,

    /// Security configuration
    pub security_config: SecurityConfig,

    /// Monitoring configuration
    pub monitoring_config: MonitoringConfig,

    /// Log sanitization configuration
    pub sanitization_config: SanitizationConfig,

    /// Metrics persistence configuration
    pub persistence_config: Option<PersistenceConfig>,

    /// Telemetry configuration
    pub telemetry_config: TelemetryConfig,

    /// Alert configuration
    pub alert_config: AlertConfig,

    /// Dashboard configuration
    pub dashboard_config: DashboardConfig,

    /// Profiling configuration
    pub profiling_config: ProfilingConfig,

    /// Enable graceful shutdown
    pub graceful_shutdown: bool,

    /// Shutdown timeout in seconds
    pub shutdown_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_info: ServerInfo {
                protocol_version: ProtocolVersion::default(),
                capabilities: ServerCapabilities::default(),
                server_info: Implementation {
                    name: "MCP Server".to_string(),
                    version: "1.0.0".to_string(),
                },
                instructions: None,
            },
            auth_config: pulseengine_mcp_auth::default_config(),
            transport_config: pulseengine_mcp_transport::TransportConfig::default(),
            security_config: pulseengine_mcp_security::default_config(),
            monitoring_config: pulseengine_mcp_monitoring::default_config(),
            sanitization_config: SanitizationConfig::default(),
            persistence_config: None,
            telemetry_config: TelemetryConfig::default(),
            alert_config: AlertConfig::default(),
            dashboard_config: DashboardConfig::default(),
            profiling_config: ProfilingConfig::default(),
            graceful_shutdown: true,
            shutdown_timeout_secs: 30,
        }
    }
}

/// Generic MCP server with pluggable backend
pub struct McpServer<B: McpBackend> {
    backend: Arc<B>,
    handler: GenericServerHandler<B>,
    auth_manager: Arc<AuthenticationManager>,
    transport: Box<dyn Transport>,
    #[allow(dead_code)]
    middleware_stack: MiddlewareStack,
    monitoring_metrics: Arc<MetricsCollector>,
    #[allow(dead_code)]
    logging_metrics: Arc<pulseengine_mcp_logging::MetricsCollector>,
    #[allow(dead_code)]
    logger: StructuredLogger,
    telemetry: Option<TelemetryManager>,
    alert_manager: Arc<AlertManager>,
    dashboard_manager: Arc<DashboardManager>,
    profiler: Option<Arc<PerformanceProfiler>>,
    config: ServerConfig,
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl<B: McpBackend + 'static> McpServer<B> {
    /// Create a new MCP server with the given backend and configuration
    pub async fn new(backend: B, config: ServerConfig) -> std::result::Result<Self, ServerError> {
        // Initialize structured logging
        let logger = StructuredLogger::new();

        info!("Initializing MCP server with backend");

        // Initialize telemetry
        let telemetry = if config.telemetry_config.enabled {
            let mut telemetry_config = config.telemetry_config.clone();
            telemetry_config.service_name = config.server_info.server_info.name.clone();
            telemetry_config.service_version = config.server_info.server_info.version.clone();

            Some(TelemetryManager::new(telemetry_config).await.map_err(|e| {
                ServerError::Configuration(format!("Failed to initialize telemetry: {e}"))
            })?)
        } else {
            None
        };

        // Initialize authentication
        let auth_manager = Arc::new(
            AuthenticationManager::new(config.auth_config.clone())
                .await
                .map_err(|e| ServerError::Authentication(e.to_string()))?,
        );

        // Initialize transport
        let transport =
            pulseengine_mcp_transport::create_transport(config.transport_config.clone())
                .map_err(|e| ServerError::Transport(e.to_string()))?;

        // Initialize security middleware
        let security_middleware = SecurityMiddleware::new(config.security_config.clone());

        // Initialize monitoring
        let monitoring_metrics = Arc::new(MetricsCollector::new(config.monitoring_config.clone()));

        // Initialize logging metrics with optional persistence
        let logging_metrics = Arc::new(pulseengine_mcp_logging::MetricsCollector::new());
        if let Some(persistence_config) = config.persistence_config.clone() {
            logging_metrics
                .enable_persistence(persistence_config.clone())
                .await
                .map_err(|e| {
                    ServerError::Configuration(format!(
                        "Failed to initialize metrics persistence: {e}"
                    ))
                })?;
        }
        let middleware_stack = MiddlewareStack::new()
            .with_security(security_middleware)
            .with_monitoring(monitoring_metrics.clone())
            .with_auth(auth_manager.clone());

        // Create backend arc
        let backend = Arc::new(backend);

        // Initialize alert manager
        let alert_manager = Arc::new(AlertManager::new(config.alert_config.clone()));

        // Initialize dashboard manager
        let dashboard_manager = Arc::new(DashboardManager::new(config.dashboard_config.clone()));

        // Initialize profiler if enabled
        let profiler = if config.profiling_config.enabled {
            Some(Arc::new(PerformanceProfiler::new(
                config.profiling_config.clone(),
            )))
        } else {
            None
        };

        // Create handler
        let handler = GenericServerHandler::new(
            backend.clone(),
            auth_manager.clone(),
            middleware_stack.clone(),
        );

        Ok(Self {
            backend,
            handler,
            auth_manager,
            transport,
            middleware_stack,
            monitoring_metrics,
            logging_metrics,
            logger,
            telemetry,
            alert_manager,
            dashboard_manager,
            profiler,
            config,
            running: Arc::new(tokio::sync::RwLock::new(false)),
        })
    }

    /// Start the server
    #[tracing::instrument(skip(self))]
    pub async fn start(&mut self) -> std::result::Result<(), ServerError> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(ServerError::AlreadyRunning);
            }
            *running = true;
        }

        info!("Starting MCP server");

        // Call backend startup hook
        self.backend
            .on_startup()
            .await
            .map_err(|e| ServerError::Backend(e.to_string()))?;

        // Start background services
        self.auth_manager
            .start_background_tasks()
            .await
            .map_err(|e| ServerError::Authentication(e.to_string()))?;

        // Start alert manager
        self.alert_manager.start().await;

        // Start dashboard manager with metrics updates
        self.start_dashboard_metrics_update().await;

        // Start profiler if enabled
        if let Some(profiler) = &self.profiler {
            profiler
                .start_session(
                    format!("server_session_{}", chrono::Utc::now().timestamp()),
                    pulseengine_mcp_logging::ProfilingSessionType::Continuous,
                )
                .await
                .map_err(|e| {
                    ServerError::Configuration(format!("Failed to start profiling session: {e}"))
                })?;
        }

        // Metrics persistence is now handled internally by the logging metrics collector
        // No need for manual snapshot saving

        // Start transport
        let handler = self.handler.clone();
        self.transport
            .start(Box::new(move |request| {
                let handler = handler.clone();
                Box::pin(async move {
                    match handler.handle_request(request).await {
                        Ok(response) => response,
                        Err(error) => Response {
                            jsonrpc: "2.0".to_string(),
                            id: serde_json::Value::Null,
                            result: None,
                            error: Some(error.into()),
                        },
                    }
                })
            }))
            .await
            .map_err(|e| ServerError::Transport(e.to_string()))?;

        info!("MCP server started successfully");

        // Setup graceful shutdown if enabled
        if self.config.graceful_shutdown {
            let running = self.running.clone();
            tokio::spawn(async move {
                signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
                warn!("Shutdown signal received");
                let mut running = running.write().await;
                *running = false;
            });
        }

        Ok(())
    }

    /// Stop the server gracefully
    pub async fn stop(&mut self) -> std::result::Result<(), ServerError> {
        {
            let mut running = self.running.write().await;
            if !*running {
                return Err(ServerError::NotRunning);
            }
            *running = false;
        }

        info!("Stopping MCP server");

        // Stop transport
        self.transport
            .stop()
            .await
            .map_err(|e| ServerError::Transport(e.to_string()))?;

        // Stop background services
        self.monitoring_metrics.stop_collection().await;

        self.auth_manager
            .stop_background_tasks()
            .await
            .map_err(|e| ServerError::Authentication(e.to_string()))?;

        // Stop profiler if enabled
        if let Some(profiler) = &self.profiler {
            profiler.stop_session().await.map_err(|e| {
                ServerError::Configuration(format!("Failed to stop profiling session: {e}"))
            })?;
        }

        // Shutdown telemetry
        if let Some(telemetry) = &self.telemetry {
            telemetry.shutdown().await.map_err(|e| {
                ServerError::Configuration(format!("Failed to shutdown telemetry: {e}"))
            })?;
        }

        // Call backend shutdown hook
        self.backend
            .on_shutdown()
            .await
            .map_err(|e| ServerError::Backend(e.to_string()))?;

        info!("MCP server stopped");
        Ok(())
    }

    /// Run the server until shutdown signal
    pub async fn run(&mut self) -> std::result::Result<(), ServerError> {
        self.start().await?;

        // Wait for shutdown signal
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let running = self.running.read().await;
            if !*running {
                break;
            }
        }

        self.stop().await?;
        Ok(())
    }

    /// Get server health status
    pub async fn health_check(&self) -> std::result::Result<HealthStatus, ServerError> {
        // Check backend health
        let backend_healthy = self.backend.health_check().await.is_ok();

        // Check transport health
        let transport_healthy = self.transport.health_check().await.is_ok();

        // Check auth health
        let auth_healthy = self.auth_manager.health_check().await.is_ok();

        let overall_healthy = backend_healthy && transport_healthy && auth_healthy;

        Ok(HealthStatus {
            status: if overall_healthy {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            components: vec![
                ("backend".to_string(), backend_healthy),
                ("transport".to_string(), transport_healthy),
                ("auth".to_string(), auth_healthy),
            ]
            .into_iter()
            .collect(),
            uptime_seconds: self.monitoring_metrics.get_uptime_seconds(),
        })
    }

    /// Get server metrics
    pub async fn get_metrics(&self) -> ServerMetrics {
        self.monitoring_metrics.get_current_metrics().await
    }

    /// Get server information
    pub fn get_server_info(&self) -> &ServerInfo {
        &self.config.server_info
    }

    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get alert manager
    pub fn get_alert_manager(&self) -> Arc<AlertManager> {
        self.alert_manager.clone()
    }

    /// Get dashboard manager
    pub fn get_dashboard_manager(&self) -> Arc<DashboardManager> {
        self.dashboard_manager.clone()
    }

    /// Get profiler
    pub fn get_profiler(&self) -> Option<Arc<PerformanceProfiler>> {
        self.profiler.clone()
    }

    /// Start dashboard metrics update loop
    async fn start_dashboard_metrics_update(&self) {
        if !self.config.dashboard_config.enabled {
            return;
        }

        let logging_metrics = self.logging_metrics.clone();
        let dashboard_manager = self.dashboard_manager.clone();
        let refresh_interval = self.config.dashboard_config.refresh_interval_secs;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(refresh_interval));

            loop {
                interval.tick().await;

                // Get current metrics snapshot
                let metrics_snapshot = logging_metrics.get_metrics_snapshot().await;

                // Update dashboard with new metrics
                dashboard_manager.update_metrics(metrics_snapshot).await;
            }
        });
    }
}

/// Health status information
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub components: std::collections::HashMap<String, bool>,
    pub uptime_seconds: u64,
}

// Re-export monitoring metrics type
pub use pulseengine_mcp_monitoring::ServerMetrics;
