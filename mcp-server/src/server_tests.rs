//! Tests for MCP server implementation

use crate::backend::{BackendError, McpBackend};
use crate::server::{HealthStatus, McpServer, ServerConfig, ServerError};
use async_trait::async_trait;
use pulseengine_mcp_auth::{config::StorageConfig, AuthConfig};
use pulseengine_mcp_monitoring::MonitoringConfig;
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_security::SecurityConfig;
use pulseengine_mcp_transport::TransportConfig;
use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;
use tokio::time::timeout;

// Mock backend for server testing
#[derive(Clone)]
struct MockServerBackend {
    should_fail_health: bool,
    should_fail_startup: bool,
    should_fail_shutdown: bool,
    server_name: String,
}

#[derive(Debug)]
struct MockServerError(String);

impl fmt::Display for MockServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mock server error: {}", self.0)
    }
}

impl StdError for MockServerError {}

impl From<BackendError> for MockServerError {
    fn from(err: BackendError) -> Self {
        MockServerError(err.to_string())
    }
}

impl From<MockServerError> for Error {
    fn from(err: MockServerError) -> Self {
        Error::internal_error(err.to_string())
    }
}

#[async_trait]
impl McpBackend for MockServerBackend {
    type Error = MockServerError;
    type Config = (bool, bool, bool, String);

    async fn initialize(
        (should_fail_health, should_fail_startup, should_fail_shutdown, server_name): Self::Config,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            should_fail_health,
            should_fail_startup,
            should_fail_shutdown,
            server_name,
        })
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: self.server_name.clone(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Mock server backend for testing".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        if self.should_fail_health {
            Err(MockServerError("Backend health check failed".to_string()))
        } else {
            Ok(())
        }
    }

    async fn on_startup(&self) -> std::result::Result<(), Self::Error> {
        if self.should_fail_startup {
            Err(MockServerError("Backend startup failed".to_string()))
        } else {
            Ok(())
        }
    }

    async fn on_shutdown(&self) -> std::result::Result<(), Self::Error> {
        if self.should_fail_shutdown {
            Err(MockServerError("Backend shutdown failed".to_string()))
        } else {
            Ok(())
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        Ok(ListToolsResult {
            tools: vec![],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        _request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        Ok(CallToolResult {
            content: vec![],
            is_error: Some(false),
        })
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        Err(BackendError::not_supported(format!("Resource not found: {}", request.uri)).into())
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        Err(BackendError::not_supported(format!("Prompt not found: {}", request.name)).into())
    }
}

#[test]
fn test_server_error_types() {
    let config_err = ServerError::Configuration("Config failed".to_string());
    assert!(config_err
        .to_string()
        .contains("Server configuration error: Config failed"));

    let transport_err = ServerError::Transport("Transport failed".to_string());
    assert!(transport_err
        .to_string()
        .contains("Transport error: Transport failed"));

    let auth_err = ServerError::Authentication("Auth failed".to_string());
    assert!(auth_err
        .to_string()
        .contains("Authentication error: Auth failed"));

    let backend_err = ServerError::Backend("Backend failed".to_string());
    assert!(backend_err
        .to_string()
        .contains("Backend error: Backend failed"));

    assert!(ServerError::AlreadyRunning
        .to_string()
        .contains("Server already running"));
    assert!(ServerError::NotRunning
        .to_string()
        .contains("Server not running"));
    assert!(ServerError::ShutdownTimeout
        .to_string()
        .contains("Shutdown timeout"));
}

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();

    assert_eq!(config.server_info.server_info.name, "MCP Server");
    assert_eq!(config.server_info.server_info.version, "1.0.0");
    assert!(config.graceful_shutdown);
    assert_eq!(config.shutdown_timeout_secs, 30);
}

#[test]
fn test_server_config_custom() {
    let mut config = ServerConfig::default();
    config.server_info.server_info.name = "Custom Server".to_string();
    config.graceful_shutdown = false;
    config.shutdown_timeout_secs = 60;

    assert_eq!(config.server_info.server_info.name, "Custom Server");
    assert!(!config.graceful_shutdown);
    assert_eq!(config.shutdown_timeout_secs, 60);
}

#[tokio::test]
async fn test_server_creation() {
    let backend = MockServerBackend::initialize((false, false, false, "Test Server".to_string()))
        .await
        .unwrap();
    let config = ServerConfig {
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await;
    if let Err(e) = &server {
        println!("Server creation failed: {e:?}");
    }
    assert!(server.is_ok());

    let server = server.unwrap();
    assert_eq!(server.get_server_info().server_info.name, "MCP Server"); // Uses config, not backend
    assert!(!server.is_running().await);
}

#[tokio::test]
async fn test_server_creation_with_custom_config() {
    let backend =
        MockServerBackend::initialize((false, false, false, "Backend Server".to_string()))
            .await
            .unwrap();

    let mut config = ServerConfig::default();
    config.server_info.server_info.name = "Custom Server".to_string();
    config.server_info.server_info.version = "2.0.0".to_string();
    config.auth_config = AuthConfig {
        storage: StorageConfig::Memory,
        enabled: false,
        cache_size: 100,
        session_timeout_secs: 3600,
        max_failed_attempts: 5,
        rate_limit_window_secs: 900,
    };

    let server = McpServer::new(backend, config).await.unwrap();

    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "Custom Server");
    assert_eq!(server_info.server_info.version, "2.0.0");
}

#[tokio::test]
async fn test_server_health_check() {
    let backend =
        MockServerBackend::initialize((false, false, false, "Healthy Server".to_string()))
            .await
            .unwrap();
    let config = ServerConfig {
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    let health = server.health_check().await.unwrap();
    // Transport health check may fail for stdio - that's expected
    // As long as we get a health response with all components, that's good
    assert!(health.components.contains_key("backend"));
    assert!(health.components.contains_key("transport"));
    assert!(health.components.contains_key("auth"));

    // Backend should be healthy since we created it with should_fail=false
    assert_eq!(health.components.get("backend"), Some(&true));

    // Auth should be healthy since it's disabled
    assert_eq!(health.components.get("auth"), Some(&true));
}

#[tokio::test]
async fn test_server_health_check_unhealthy_backend() {
    let backend =
        MockServerBackend::initialize((true, false, false, "Unhealthy Server".to_string()))
            .await
            .unwrap();
    let config = ServerConfig {
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    let health = server.health_check().await.unwrap();
    assert_eq!(health.status, "unhealthy");
    assert_eq!(health.components.get("backend"), Some(&false));
}

#[tokio::test]
async fn test_server_get_metrics() {
    let backend =
        MockServerBackend::initialize((false, false, false, "Metrics Server".to_string()))
            .await
            .unwrap();
    let config = ServerConfig {
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    let metrics = server.get_metrics().await;
    // Just verify we can get metrics without error
    // Just verify we can get metrics without error (remove redundant comparison)
    let _ = metrics.requests_total;
}

#[tokio::test]
async fn test_server_start_stop() {
    let backend =
        MockServerBackend::initialize((false, false, false, "Start Stop Server".to_string()))
            .await
            .unwrap();
    // Use stdio transport to avoid port conflicts
    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        graceful_shutdown: false, // Disable signal handling for test
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let mut server = McpServer::new(backend, config).await.unwrap();

    // Server should not be running initially
    assert!(!server.is_running().await);

    // Start the server
    let start_result = server.start().await;
    assert!(start_result.is_ok());
    assert!(server.is_running().await);

    // Try to start again - should fail
    let start_again_result = server.start().await;
    assert!(start_again_result.is_err());
    assert!(matches!(
        start_again_result.unwrap_err(),
        ServerError::AlreadyRunning
    ));

    // Stop the server
    let stop_result = server.stop().await;
    assert!(stop_result.is_ok());
    assert!(!server.is_running().await);

    // Try to stop again - should fail
    let stop_again_result = server.stop().await;
    assert!(stop_again_result.is_err());
    assert!(matches!(
        stop_again_result.unwrap_err(),
        ServerError::NotRunning
    ));
}

#[tokio::test]
async fn test_server_startup_failure() {
    let backend =
        MockServerBackend::initialize((false, true, false, "Startup Fail Server".to_string()))
            .await
            .unwrap();
    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let mut server = McpServer::new(backend, config).await.unwrap();

    let start_result = server.start().await;
    assert!(start_result.is_err());
    assert!(matches!(start_result.unwrap_err(), ServerError::Backend(_)));
}

#[tokio::test]
async fn test_server_run_with_timeout() {
    let backend = MockServerBackend::initialize((false, false, false, "Run Server".to_string()))
        .await
        .unwrap();
    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        graceful_shutdown: false,
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let mut server = McpServer::new(backend, config).await.unwrap();

    // Run the server with a timeout
    let run_result = timeout(Duration::from_millis(100), server.run()).await;

    // Should timeout since the server runs indefinitely
    assert!(run_result.is_err());
}

#[tokio::test]
async fn test_server_with_different_transports() {
    let backend =
        MockServerBackend::initialize((false, false, false, "Transport Server".to_string()))
            .await
            .unwrap();

    // Test with Stdio transport
    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend.clone(), config).await;
    assert!(server.is_ok());

    // Test with HTTP transport (should work with default port)
    let config = ServerConfig {
        transport_config: TransportConfig::Http {
            host: Some("127.0.0.1".to_string()),
            port: 0, // Use random port
        },
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await;
    assert!(server.is_ok());
}

#[tokio::test]
async fn test_server_with_auth_config() {
    let backend = MockServerBackend::initialize((false, false, false, "Auth Server".to_string()))
        .await
        .unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false, // Keep disabled for tests
            cache_size: 1000,
            session_timeout_secs: 3600, // 60 minutes
            max_failed_attempts: 5,
            rate_limit_window_secs: 60,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await;
    assert!(server.is_ok());
}

#[tokio::test]
async fn test_server_with_security_config() {
    let backend =
        MockServerBackend::initialize((false, false, false, "Security Server".to_string()))
            .await
            .unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        security_config: SecurityConfig {
            validate_requests: true,
            rate_limiting: true,
            max_requests_per_minute: 100,
            cors_enabled: true,
            cors_origins: vec!["http://localhost:3000".to_string()],
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await;
    assert!(server.is_ok());
}

#[tokio::test]
async fn test_server_with_monitoring_config() {
    let backend =
        MockServerBackend::initialize((false, false, false, "Monitoring Server".to_string()))
            .await
            .unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        monitoring_config: MonitoringConfig {
            enabled: true,
            collection_interval_secs: 10,
            performance_monitoring: true,
            health_checks: true,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await;
    assert!(server.is_ok());
}

#[test]
fn test_health_status_serialization() {
    use std::collections::HashMap;

    let mut components = HashMap::new();
    components.insert("backend".to_string(), true);
    components.insert("transport".to_string(), false);

    let health = HealthStatus {
        status: "degraded".to_string(),
        components,
        uptime_seconds: 3600,
    };

    let serialized = serde_json::to_string(&health).unwrap();
    assert!(serialized.contains("degraded"));
    assert!(serialized.contains("backend"));
    assert!(serialized.contains("3600"));

    let deserialized: HealthStatus = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.status, "degraded");
    assert_eq!(deserialized.uptime_seconds, 3600);
    assert_eq!(deserialized.components.len(), 2);
}

#[test]
fn test_server_config_debug() {
    let config = ServerConfig::default();
    let debug_str = format!("{config:?}");
    assert!(debug_str.contains("ServerConfig"));
    assert!(debug_str.contains("MCP Server"));
}

#[test]
fn test_server_config_clone() {
    let config = ServerConfig::default();
    let cloned = config.clone();

    assert_eq!(
        config.server_info.server_info.name,
        cloned.server_info.server_info.name
    );
    assert_eq!(config.graceful_shutdown, cloned.graceful_shutdown);
    assert_eq!(config.shutdown_timeout_secs, cloned.shutdown_timeout_secs);
}

// Test thread safety
#[test]
fn test_server_types_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<ServerError>();
    assert_sync::<ServerError>();
    assert_send::<ServerConfig>();
    assert_sync::<ServerConfig>();
    assert_send::<HealthStatus>();
    assert_sync::<HealthStatus>();
}

#[test]
fn test_server_error_debug() {
    let err = ServerError::Backend("test".to_string());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("Backend"));
    assert!(debug_str.contains("test"));
}
