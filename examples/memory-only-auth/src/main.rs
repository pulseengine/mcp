//! Memory-Only Authentication Example
//!
//! This example demonstrates how to run a PulseEngine MCP server with
//! memory-only authentication, eliminating all filesystem dependencies.
//!
//! All API keys are stored in memory and are lost when the server restarts.
//! This is ideal for development, testing, or containerized deployments.

use pulseengine_mcp_auth::{config::AuthConfig, models::Role, AuthenticationManager};
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::{BackendError, McpBackend, McpServer, ServerConfig};
use pulseengine_mcp_transport::TransportConfig;

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),
}

impl From<ServerError> for pulseengine_mcp_protocol::Error {
    fn from(err: ServerError) -> Self {
        match err {
            ServerError::InvalidParameter(msg) => Error::invalid_params(msg),
            ServerError::Backend(backend_err) => backend_err.into(),
        }
    }
}

#[derive(Clone)]
pub struct MemoryAuthBackend {
    auth_manager: Arc<AuthenticationManager>,
}

#[derive(Debug, Clone)]
pub struct MemoryAuthConfig {
    pub initial_api_keys: Vec<(String, String, Role)>,
}

impl Default for MemoryAuthConfig {
    fn default() -> Self {
        Self {
            initial_api_keys: vec![
                (
                    "admin_key_1".to_string(),
                    "admin-secret-key-12345".to_string(),
                    Role::Admin,
                ),
                (
                    "operator_key_1".to_string(),
                    "operator-secret-key-67890".to_string(),
                    Role::Operator,
                ),
                (
                    "monitor_key_1".to_string(),
                    "monitor-secret-key-abcdef".to_string(),
                    Role::Monitor,
                ),
            ],
        }
    }
}

#[async_trait]
impl McpBackend for MemoryAuthBackend {
    type Error = ServerError;
    type Config = MemoryAuthConfig;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::Error> {
        info!("Initializing Memory-Only Authentication backend");

        // Create memory-only auth configuration
        let auth_config = AuthConfig::memory();

        // Initialize authentication manager
        let auth_manager = AuthenticationManager::new(auth_config)
            .await
            .map_err(|e| ServerError::InvalidParameter(format!("Auth init failed: {e}")))?;

        // Add initial API keys to memory storage
        for (name, _api_key, role) in config.initial_api_keys {
            let _api_key_obj = auth_manager
                .create_api_key(name.clone(), role.clone(), None, None)
                .await
                .map_err(|e| {
                    ServerError::InvalidParameter(format!("Failed to create key {name}: {e}"))
                })?;

            info!("Added {} API key: {}", role, name);
        }

        Ok(Self {
            auth_manager: Arc::new(auth_manager),
        })
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                logging: None,
                sampling: None,
                ..Default::default()
            },
            server_info: Implementation {
                name: "Memory-Only Auth MCP Server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some(
                "MCP server with in-memory authentication - keys are lost on restart".to_string(),
            ),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        let keys = self.auth_manager.list_keys().await;
        let key_count = keys.len();

        info!("Health check passed - {} API keys in memory", key_count);
        Ok(())
    }

    async fn list_tools(
        &self,
        _: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "list_auth_keys".to_string(),
                    description: "List all API keys currently in memory".to_string(),
                    input_schema: json!({"type": "object", "properties": {}}),
                    output_schema: None,
                    title: None,
                    annotations: None,
                    icons: None,
                    _meta: None,
                },
                Tool {
                    name: "add_temp_key".to_string(),
                    description: "Add a temporary API key to memory".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "name": {"type": "string", "description": "Human readable name"},
                            "role": {"type": "string", "enum": ["Admin", "Operator", "Monitor", "Device"]}
                        },
                        "required": ["name", "role"]
                    }),
                    output_schema: None,
                    title: None,
                    annotations: None,
                    icons: None,
                    _meta: None,
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
            "list_auth_keys" => {
                let keys = self.auth_manager.list_keys().await;

                let key_info: Vec<_> = keys
                    .into_iter()
                    .map(|key| {
                        format!(
                            "ID: {}, Name: {}, Role: {}, Active: {}, Created: {}",
                            key.id,
                            key.name,
                            key.role,
                            key.active,
                            key.created_at.format("%Y-%m-%d %H:%M:%S")
                        )
                    })
                    .collect();

                Ok(CallToolResult {
                    content: vec![Content::text(format!(
                        "API Keys in Memory:\n{}",
                        key_info.join("\n")
                    ))],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }
            "add_temp_key" => {
                let args = request.arguments.unwrap_or_default();

                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidParameter("name required".to_string()))?;
                let role_str = args
                    .get("role")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidParameter("role required".to_string()))?;

                let role = match role_str {
                    "Admin" => Role::Admin,
                    "Operator" => Role::Operator,
                    "Monitor" => Role::Monitor,
                    "Device" => Role::Device {
                        allowed_devices: vec![],
                    },
                    _ => return Err(ServerError::InvalidParameter("Invalid role".to_string())),
                };

                let api_key_obj = self
                    .auth_manager
                    .create_api_key(name.to_string(), role.clone(), None, None)
                    .await
                    .map_err(|e| {
                        ServerError::InvalidParameter(format!("Failed to create key: {e}"))
                    })?;

                Ok(CallToolResult {
                    content: vec![Content::text(format!(
                        "Added temporary {} API key: {} (ID: {})",
                        role, name, api_key_obj.id
                    ))],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }
            _ => Err(ServerError::InvalidParameter(format!(
                "Unknown tool: {}",
                request.name
            ))),
        }
    }

    async fn list_resources(
        &self,
        _: PaginatedRequestParam,
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
        Err(ServerError::InvalidParameter(format!(
            "Resource not found: {}",
            request.uri
        )))
    }

    async fn list_prompts(
        &self,
        _: PaginatedRequestParam,
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
        Err(ServerError::InvalidParameter(format!(
            "Prompt not found: {}",
            request.name
        )))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("🚀 Starting Memory-Only Authentication MCP Server");

    let backend = MemoryAuthBackend::initialize(MemoryAuthConfig::default())
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let server_config = ServerConfig {
        server_info: backend.get_server_info(),
        transport_config: TransportConfig::Stdio,
        ..Default::default()
    };

    let mut server = McpServer::new(backend, server_config)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    info!("✅ Memory-Only Authentication MCP Server started");
    info!("🔒 Authentication keys are stored in memory only");
    info!("⚠️  All keys will be lost when the server restarts");

    server
        .run()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(())
}
