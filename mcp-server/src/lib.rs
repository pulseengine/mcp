//! Generic MCP server infrastructure with pluggable backends
//!
//! This crate provides a complete MCP server implementation that can be extended
//! with custom backends for different domains (home automation, databases, APIs, etc.).
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use pulseengine_mcp_server::{McpServer, McpBackend, ServerConfig};
//! use pulseengine_mcp_protocol::*;
//! use async_trait::async_trait;
//!
//! #[derive(Clone)]
//! struct MyBackend;
//!
//! #[async_trait]
//! impl McpBackend for MyBackend {
//!     type Error = Box<dyn std::error::Error + Send + Sync>;
//!     type Config = ();
//!
//!     async fn initialize(_: ()) -> std::result::Result<Self, Self::Error> {
//!         Ok(MyBackend)
//!     }
//!
//!     fn get_server_info(&self) -> ServerInfo {
//!         ServerInfo {
//!             protocol_version: ProtocolVersion::default(),
//!             capabilities: ServerCapabilities::default(),
//!             server_info: Implementation {
//!                 name: "My Server".to_string(),
//!                 version: "1.0.0".to_string(),
//!             },
//!             instructions: Some("Example server".to_string()),
//!         }
//!     }
//!
//!     async fn list_tools(&self, _: PaginatedRequestParam) -> std::result::Result<ListToolsResult, Self::Error> {
//!         Ok(ListToolsResult { tools: vec![], next_cursor: None })
//!     }
//!
//!     async fn call_tool(&self, _: CallToolRequestParam) -> Result<CallToolResult, Self::Error> {
//!         Ok(CallToolResult { content: vec![], is_error: Some(false) })
//!     }
//!
//!     // Implement other required methods (simplified for example)
//! #   async fn list_resources(&self, _: PaginatedRequestParam) -> Result<ListResourcesResult, Self::Error> {
//! #       Ok(ListResourcesResult { resources: vec![], next_cursor: String::new() })
//! #   }
//! #   async fn read_resource(&self, _: ReadResourceRequestParam) -> Result<ReadResourceResult, Self::Error> {
//! #       Err("No resources".into())
//! #   }
//! #   async fn list_prompts(&self, _: PaginatedRequestParam) -> Result<ListPromptsResult, Self::Error> {
//! #       Ok(ListPromptsResult { prompts: vec![], next_cursor: String::new() })
//! #   }
//! #   async fn get_prompt(&self, _: GetPromptRequestParam) -> Result<GetPromptResult, Self::Error> {
//! #       Err("No prompts".into())
//! #   }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let backend = MyBackend::initialize(()).await?;
//!     let config = ServerConfig::default();
//!     let mut server = McpServer::new(backend, config).await?;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Authentication Options
//!
//! The server supports multiple authentication backends for different deployment scenarios:
//!
//! ## File-based Authentication (Default)
//!
//! ```rust,ignore
//! use pulseengine_mcp_server::{McpServer, ServerConfig, AuthConfig};
//! use pulseengine_mcp_auth::config::StorageConfig;
//! use std::path::PathBuf;
//!
//! let auth_config = AuthConfig {
//!     storage: StorageConfig::File {
//!         path: PathBuf::from("~/.pulseengine/mcp-auth/keys.enc"),
//!         file_permissions: 0o600,
//!         dir_permissions: 0o700,
//!         require_secure_filesystem: true,
//!         enable_filesystem_monitoring: false,
//!     },
//!     enabled: true,
//!     // ... other config
//! };
//!
//! let server_config = ServerConfig {
//!     auth_config: Some(auth_config),
//!     // ... other config
//! };
//! ```
//!
//! ## Environment Variable Authentication
//!
//! For containerized deployments without filesystem access:
//!
//! ```rust,ignore
//! use pulseengine_mcp_server::{McpServer, ServerConfig, AuthConfig};
//! use pulseengine_mcp_auth::config::StorageConfig;
//!
//! let auth_config = AuthConfig {
//!     storage: StorageConfig::Environment {
//!         prefix: "MCP_AUTH".to_string(),
//!     },
//!     enabled: true,
//!     // ... other config
//! };
//!
//! // Set environment variables:
//! // MCP_AUTH_API_KEY_ADMIN_1=admin-key-12345
//! // MCP_AUTH_API_KEY_OPERATOR_1=operator-key-67890
//! ```
//!
//! ## Memory-Only Authentication
//!
//! For temporary deployments where keys don't need persistence:
//!
//! ```rust,ignore
//! use pulseengine_mcp_server::{McpServer, ServerConfig, AuthConfig};
//! use pulseengine_mcp_auth::{config::StorageConfig, types::{ApiKey, Role}};
//! use std::collections::HashMap;
//!
//! // Create memory-only auth config
//! let auth_config = AuthConfig::memory();
//!
//! let server_config = ServerConfig {
//!     auth_config: Some(auth_config),
//!     // ... other config
//! };
//!
//! // Add API keys programmatically during runtime
//! let api_key = ApiKey {
//!     id: "temp_key_1".to_string(),
//!     key: "temporary-secret-key".to_string(),
//!     role: Role::Admin,
//!     created_at: chrono::Utc::now(),
//!     last_used: None,
//!     permissions: vec![],
//!     rate_limit: None,
//!     ip_whitelist: None,
//!     expires_at: None,
//!     metadata: HashMap::new(),
//! };
//!
//! // Add to server's auth manager after initialization
//! server.auth_manager().save_api_key(&api_key).await?;
//! ```
//!
//! ## Disabled Authentication
//!
//! For development or trusted environments:
//!
//! ```rust,ignore
//! let auth_config = AuthConfig::disabled();
//! let server_config = ServerConfig {
//!     auth_config: Some(auth_config),
//!     // ... other config
//! };
//! ```

pub mod backend;
pub mod context;
pub mod handler;
pub mod middleware;
pub mod server;

// Endpoint modules
pub mod alerting_endpoint;
pub mod dashboard_endpoint;
pub mod health_endpoint;
pub mod metrics_endpoint;

// Test modules
#[cfg(test)]
mod backend_tests;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod handler_tests;
#[cfg(test)]
mod lib_tests;
#[cfg(test)]
mod middleware_tests;
#[cfg(test)]
mod server_tests;

// Re-export core types
pub use backend::{BackendError, McpBackend};
pub use context::RequestContext;
pub use handler::{GenericServerHandler, HandlerError};
pub use middleware::{Middleware, MiddlewareStack};
pub use server::{McpServer, ServerConfig, ServerError};

// Re-export from dependencies for convenience
pub use pulseengine_mcp_auth::{self as auth, AuthConfig, AuthenticationManager};
pub use pulseengine_mcp_monitoring::{self as monitoring, MetricsCollector, MonitoringConfig};
pub use pulseengine_mcp_protocol::{self as protocol, *};
pub use pulseengine_mcp_security::{self as security, SecurityConfig, SecurityMiddleware};
pub use pulseengine_mcp_transport::{self as transport, Transport, TransportConfig};
