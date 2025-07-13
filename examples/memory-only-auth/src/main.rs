//! Memory-Only Authentication Example
//!
//! This example demonstrates how to run a PulseEngine MCP server with
//! memory-only authentication, eliminating all filesystem dependencies.
//!
//! All API keys are stored in memory and are lost when the server restarts.
//! This is ideal for development, testing, or containerized deployments.

use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::{BackendError, McpBackend, McpServer, ServerConfig};
use pulseengine_mcp_transport::TransportConfig;
use pulseengine_mcp_auth::{
    config::AuthConfig,
    types::{ApiKey, Role},
    AuthenticationManager,
};

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn};
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
    auth_manager: AuthenticationManager,
}

#[derive(Debug, Clone)]
pub struct MemoryAuthConfig {
    pub initial_api_keys: Vec<(String, String, Role)>,
}

impl Default for MemoryAuthConfig {
    fn default() -> Self {
        Self {
            initial_api_keys: vec![
                ("admin_key_1".to_string(), "admin-secret-key-12345".to_string(), Role::Admin),
                ("operator_key_1".to_string(), "operator-secret-key-67890".to_string(), Role::Operator),
                ("monitor_key_1".to_string(), "monitor-secret-key-abcdef".to_string(), Role::Monitor),
            ],
        }
    }
}

#[async_trait]
impl McpBackend for MemoryAuthBackend {
    type Error = ServerError;
    type Config = MemoryAuthConfig;

    async fn initialize(config: Self::Config) -> Result<Self, Self::Error> {
        info!("Initializing Memory-Only Authentication backend");
        
        // Create memory-only auth configuration
        let auth_config = AuthConfig::memory();
        
        // Initialize authentication manager
        let auth_manager = AuthenticationManager::new(auth_config)
            .await
            .map_err(|e| ServerError::InvalidParameter(format!("Auth init failed: {}", e)))?;

        // Add initial API keys to memory storage
        for (key_id, api_key, role) in config.initial_api_keys {
            let api_key_obj = ApiKey {
                id: key_id.clone(),
                key: api_key,
                role,
                created_at: chrono::Utc::now(),
                last_used: None,
                permissions: vec![],
                rate_limit: None,
                ip_whitelist: None,
                expires_at: None,
                metadata: HashMap::new(),
            };
            
            auth_manager.save_api_key(&api_key_obj)
                .await
                .map_err(|e| ServerError::InvalidParameter(format!("Failed to save key {}: {}", key_id, e)))?;
            
            info!("Added {} API key: {}", role, key_id);
        }

        Ok(Self { auth_manager })
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

    async fn health_check(&self) -> Result<(), Self::Error> {
        let key_count = self.auth_manager.list_api_keys().await
            .map_err(|e| ServerError::InvalidParameter(format!("Health check failed: {}", e)))?
            .len();
        
        info!("Health check passed - {} API keys in memory", key_count);
        Ok(())
    }

    async fn list_tools(&self, _: PaginatedRequestParam) -> Result<ListToolsResult, Self::Error> {
        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "list_auth_keys".to_string(),
                    description: "List all API keys currently in memory".to_string(),
                    input_schema: json!({"type": "object", "properties": {}}),
                },
                Tool {
                    name: "add_temp_key".to_string(),
                    description: "Add a temporary API key to memory".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "key_id": {"type": "string", "description": "Unique identifier"},
                            "api_key": {"type": "string", "description": "The API key value"},
                            "role": {"type": "string", "enum": ["Admin", "Operator", "Monitor", "Device"]}
                        },
                        "required": ["key_id", "api_key", "role"]
                    }),
                },
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(&self, request: CallToolRequestParam) -> Result<CallToolResult, Self::Error> {
        match request.name.as_str() {
            "list_auth_keys" => {
                let keys = self.auth_manager.list_api_keys().await
                    .map_err(|e| ServerError::InvalidParameter(format!("Failed to list keys: {}", e)))?;
                
                let key_info: Vec<_> = keys.into_iter()
                    .map(|key| format!("ID: {}, Role: {:?}, Created: {}", 
                        key.id, key.role, key.created_at.format("%Y-%m-%d %H:%M:%S")))
                    .collect();
                
                Ok(CallToolResult {
                    content: vec![Content::text(format!(
                        "API Keys in Memory:\n{}", 
                        key_info.join("\n")
                    ))],
                    is_error: Some(false),
                })
            }
            "add_temp_key" => {
                let args = request.arguments.unwrap_or_default();
                
                let key_id = args.get("key_id").and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidParameter("key_id required".to_string()))?;
                let api_key = args.get("api_key").and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidParameter("api_key required".to_string()))?;
                let role_str = args.get("role").and_then(|v| v.as_str())
                    .ok_or_else(|| ServerError::InvalidParameter("role required".to_string()))?;
                
                let role = match role_str {
                    "Admin" => Role::Admin,
                    "Operator" => Role::Operator,
                    "Monitor" => Role::Monitor,
                    "Device" => Role::Device,
                    _ => return Err(ServerError::InvalidParameter("Invalid role".to_string())),
                };
                
                let api_key_obj = ApiKey {
                    id: key_id.to_string(),
                    key: api_key.to_string(),
                    role,
                    created_at: chrono::Utc::now(),
                    last_used: None,
                    permissions: vec![],
                    rate_limit: None,
                    ip_whitelist: None,
                    expires_at: None,
                    metadata: HashMap::new(),
                };
                
                self.auth_manager.save_api_key(&api_key_obj).await
                    .map_err(|e| ServerError::InvalidParameter(format!("Failed to save key: {}", e)))?;
                
                Ok(CallToolResult {
                    content: vec![Content::text(format!(
                        "Added temporary {} API key: {}", role, key_id
                    ))],
                    is_error: Some(false),
                })
            }
            _ => Err(ServerError::InvalidParameter(format!("Unknown tool: {}", request.name))),
        }
    }

    async fn list_resources(&self, _: PaginatedRequestParam) -> Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult { resources: vec![], next_cursor: None })
    }

    async fn read_resource(&self, request: ReadResourceRequestParam) -> Result<ReadResourceResult, Self::Error> {
        Err(ServerError::InvalidParameter(format!("Resource not found: {}", request.uri)))
    }

    async fn list_prompts(&self, _: PaginatedRequestParam) -> Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult { prompts: vec![], next_cursor: None })
    }

    async fn get_prompt(&self, request: GetPromptRequestParam) -> Result<GetPromptResult, Self::Error> {
        Err(ServerError::InvalidParameter(format!("Prompt not found: {}", request.name)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    info!("🚀 Starting Memory-Only Authentication MCP Server");

    let backend = MemoryAuthBackend::initialize(MemoryAuthConfig::default()).await?;
    let server_config = ServerConfig {
        server_info: backend.get_server_info(),
        transport_config: TransportConfig::Stdio,
        ..Default::default()
    };

    let mut server = McpServer::new(backend, server_config).await?;

    info!("✅ Memory-Only Authentication MCP Server started");
    info!("🔒 Authentication keys are stored in memory only");
    info!("⚠️  All keys will be lost when the server restarts");

    server.run().await?;
    Ok(())
}