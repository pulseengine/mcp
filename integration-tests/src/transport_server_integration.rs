//! Integration tests for transport and server interaction

use crate::test_utils::*;
use async_trait::async_trait;
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::{
    backend::{BackendError, McpBackend},
    server::{McpServer, ServerConfig},
};
use pulseengine_mcp_transport::TransportConfig;
use std::error::Error as StdError;
use std::fmt;
use std::time::Duration;
use tokio::net::TcpListener;

// Simple test backend for transport testing
#[derive(Clone)]
struct TransportTestBackend {
    server_name: String,
}

#[derive(Debug)]
struct TransportTestError(String);

impl fmt::Display for TransportTestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transport test error: {}", self.0)
    }
}

impl StdError for TransportTestError {}

impl From<BackendError> for TransportTestError {
    fn from(err: BackendError) -> Self {
        TransportTestError(err.to_string())
    }
}

impl From<TransportTestError> for Error {
    fn from(err: TransportTestError) -> Self {
        Error::internal_error(err.to_string())
    }
}

#[async_trait]
impl McpBackend for TransportTestBackend {
    type Error = TransportTestError;
    type Config = String;

    async fn initialize(server_name: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(Self { server_name })
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(true),
                }),
                prompts: Some(PromptsCapability {
                    list_changed: Some(true),
                }),
                logging: Some(LoggingCapability {
                    level: Some("info".to_string()),
                }),
                sampling: None,
                ..Default::default()
            },
            server_info: Implementation {
                name: self.server_name.clone(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Backend for transport integration testing".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "echo_tool".to_string(),
                    description: "Echoes back the input message".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": {"type": "string"}
                        },
                        "required": ["message"]
                    }),
                    output_schema: None,
                },
                Tool {
                    name: "transport_info".to_string(),
                    description: "Returns information about the transport layer".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    }),
                    output_schema: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        match request.name.as_str() {
            "echo_tool" => {
                let args = request.arguments.unwrap_or_default();
                let message = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No message");

                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: format!("Echo: {message}"),
                    }],
                    is_error: Some(false),
                    structured_content: None,
                })
            }
            "transport_info" => Ok(CallToolResult {
                content: vec![Content::Text {
                    text: format!("Transport test backend: {}", self.server_name),
                }],
                is_error: Some(false),
                structured_content: None,
            }),
            _ => {
                Err(BackendError::not_supported(format!("Tool not found: {}", request.name)).into())
            }
        }
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult {
            resources: vec![Resource {
                uri: "test://transport_resource".to_string(),
                name: "Transport Test Resource".to_string(),
                description: Some("A resource for testing transport layer".to_string()),
                mime_type: Some("text/plain".to_string()),
                annotations: None,
                raw: None,
            }],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        if request.uri == "test://transport_resource" {
            Ok(ReadResourceResult {
                contents: vec![ResourceContents {
                    uri: request.uri.clone(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some(format!(
                        "Transport resource content from {}",
                        self.server_name
                    )),
                    blob: None,
                }],
            })
        } else {
            Err(BackendError::not_supported(format!("Resource not found: {}", request.uri)).into())
        }
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

// Helper function to find a free port
#[allow(dead_code)]
async fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[tokio::test]
async fn test_server_with_stdio_transport() {
    let backend = TransportTestBackend::initialize("Stdio Backend".to_string())
        .await
        .unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Verify server configuration
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "MCP Server"); // Uses config name, not backend name

    // Health check should include transport
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("transport"));

    // Server should not be running initially
    assert!(!server.is_running().await);
}

#[tokio::test]
async fn test_server_with_http_transport() {
    let backend = TransportTestBackend::initialize("HTTP Backend".to_string())
        .await
        .unwrap();
    let port = find_free_port().await;

    let config = ServerConfig {
        transport_config: TransportConfig::Http {
            host: Some("127.0.0.1".to_string()),
            port,
        },
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Verify server configuration
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "MCP Server");

    // Health check should include transport
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("transport"));

    // Server should not be running initially
    assert!(!server.is_running().await);
}

#[tokio::test]
async fn test_server_with_websocket_transport() {
    let backend = TransportTestBackend::initialize("WebSocket Backend".to_string())
        .await
        .unwrap();
    let port = find_free_port().await;

    let config = ServerConfig {
        transport_config: TransportConfig::WebSocket {
            host: Some("127.0.0.1".to_string()),
            port,
        },
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Verify server configuration
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "MCP Server");

    // Health check should include transport
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("transport"));

    // Server should not be running initially
    assert!(!server.is_running().await);
}

#[tokio::test]
async fn test_server_startup_and_shutdown() {
    let backend = TransportTestBackend::initialize("Lifecycle Backend".to_string())
        .await
        .unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        graceful_shutdown: false, // Disable signal handling for tests
        ..Default::default()
    };

    let mut server = McpServer::new(backend, config).await.unwrap();

    // Initially not running
    assert!(!server.is_running().await);

    // Start the server
    let start_result = server.start().await;
    assert!(start_result.is_ok());
    assert!(server.is_running().await);

    // Health check while running
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("transport"));
    assert!(health.components.contains_key("backend"));

    // Stop the server
    let stop_result = server.stop().await;
    assert!(stop_result.is_ok());
    assert!(!server.is_running().await);
}

#[tokio::test]
async fn test_server_run_with_timeout() {
    let backend = TransportTestBackend::initialize("Timeout Backend".to_string())
        .await
        .unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        graceful_shutdown: false,
        ..Default::default()
    };

    let mut server = McpServer::new(backend, config).await.unwrap();

    // Run the server with a very short timeout
    let run_result = tokio::time::timeout(Duration::from_millis(50), server.run()).await;

    // Should timeout since the server runs indefinitely
    assert!(run_result.is_err());
}

#[tokio::test]
async fn test_multiple_transport_configs() {
    let backends = vec![
        ("Stdio", TransportConfig::Stdio),
        (
            "HTTP",
            TransportConfig::Http {
                host: Some("127.0.0.1".to_string()),
                port: find_free_port().await,
            },
        ),
        (
            "WebSocket",
            TransportConfig::WebSocket {
                host: Some("127.0.0.1".to_string()),
                port: find_free_port().await,
            },
        ),
    ];

    for (name, transport_config) in backends {
        let backend = TransportTestBackend::initialize(format!("{} Backend", name))
            .await
            .unwrap();

        let config = ServerConfig {
            transport_config,
            auth_config: {
                let mut auth_config = test_auth_config();
                auth_config.enabled = false;
                auth_config
            },
            ..Default::default()
        };

        let server = McpServer::new(backend, config).await;
        assert!(
            server.is_ok(),
            "Failed to create server with {} transport",
            name
        );

        let server = server.unwrap();
        let health = server.health_check().await.unwrap();
        assert!(health.components.contains_key("transport"));
        assert!(health.components.contains_key("backend"));
    }
}

#[tokio::test]
async fn test_transport_error_handling() {
    let backend = TransportTestBackend::initialize("Error Backend".to_string())
        .await
        .unwrap();

    // Try to create a server with an invalid port (should work, but may fail on start)
    let config = ServerConfig {
        transport_config: TransportConfig::Http {
            host: Some("127.0.0.1".to_string()),
            port: 65000, // High port number
        },
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        ..Default::default()
    };

    // Server creation should succeed
    let server = McpServer::new(backend, config).await;
    assert!(server.is_ok());

    // Health check should still work
    let server = server.unwrap();
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("transport"));
}

#[tokio::test]
async fn test_server_metrics_with_transport() {
    let backend = TransportTestBackend::initialize("Metrics Backend".to_string())
        .await
        .unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        monitoring_config: test_monitoring_config(),
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Get initial metrics
    let metrics = server.get_metrics().await;
    // requests_total is a u64, so it's always >= 0
    assert!(metrics.requests_total < u64::MAX);
    assert!(metrics.error_rate >= 0.0);

    // Health check should include monitoring component
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("transport"));
    assert!(health.components.contains_key("backend"));
}
