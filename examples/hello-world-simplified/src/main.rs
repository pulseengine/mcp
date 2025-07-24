//! Simplified Hello World MCP Server
//!
//! This demonstrates the improved developer experience patterns
//! that we're implementing, showing the progression from complex
//! manual implementation to simple, fluent APIs.

use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::{BackendError, McpBackend, McpServer, ServerConfig};
use pulseengine_mcp_transport::TransportConfig;

use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tracing::{info, warn};

/// Simplified error type
#[derive(Debug, Error)]
pub enum SimpleError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),
}

impl From<SimpleError> for pulseengine_mcp_protocol::Error {
    fn from(err: SimpleError) -> Self {
        match err {
            SimpleError::InvalidParameter(msg) => Error::invalid_params(msg),
            SimpleError::Backend(backend_err) => backend_err.into(),
        }
    }
}

/// Simplified backend with helper functions
#[derive(Clone)]
pub struct SimpleHelloWorld {
    greeting_count: std::sync::Arc<AtomicU64>,
}

impl Default for SimpleHelloWorld {
    fn default() -> Self {
        Self {
            greeting_count: std::sync::Arc::new(AtomicU64::new(0)),
        }
    }
}

impl SimpleHelloWorld {
    /// Create a new instance - this is our simplified constructor
    pub fn new() -> Self {
        Self::default()
    }

    /// Helper function to create a tool definition - reduces boilerplate
    fn create_tool(name: &str, description: &str, schema: serde_json::Value) -> Tool {
        Tool {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: schema,
            output_schema: None,
        }
    }

    /// Tool implementation: say hello
    async fn tool_say_hello(
        &self,
        name: String,
        greeting: Option<String>,
    ) -> std::result::Result<CallToolResult, SimpleError> {
        let greeting = greeting.unwrap_or_else(|| "Hello".to_string());
        let count = self.greeting_count.fetch_add(1, Ordering::Relaxed) + 1;

        let message = format!("{greeting}, {name}! 👋 (Greeting #{count})");

        info!(tool = "say_hello", name = %name, greeting = %greeting, count = count);

        Ok(CallToolResult {
            content: vec![Content::text(message)],
            is_error: Some(false),
            structured_content: None,
        })
    }

    /// Tool implementation: count greetings
    async fn tool_count_greetings(&self) -> std::result::Result<CallToolResult, SimpleError> {
        let count = self.greeting_count.load(Ordering::Relaxed);

        info!(tool = "count_greetings", count = count);

        Ok(CallToolResult {
            content: vec![Content::text(format!("Total greetings: {count}"))],
            is_error: Some(false),
            structured_content: None,
        })
    }
}

#[async_trait]
impl McpBackend for SimpleHelloWorld {
    type Error = SimpleError;
    type Config = ();

    async fn initialize(_config: Self::Config) -> std::result::Result<Self, Self::Error> {
        info!("Initializing Simple Hello World backend");
        Ok(Self::new())
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
                logging: Some(LoggingCapability {
                    level: Some("info".to_string()),
                }),
                sampling: None,
                ..Default::default()
            },
            server_info: Implementation {
                name: "Simple Hello World MCP Server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some(
                "A simplified demonstration server with streamlined development experience"
                    .to_string(),
            ),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        // Simplified tool definition using helper
        let tools = vec![
            Self::create_tool(
                "say_hello",
                "Say hello to someone with an optional custom greeting",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Name to greet"},
                        "greeting": {"type": "string", "description": "Custom greeting (optional)"}
                    },
                    "required": ["name"]
                }),
            ),
            Self::create_tool(
                "count_greetings",
                "Get the total number of greetings sent",
                json!({"type": "object", "properties": {}}),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        match request.name.as_str() {
            "say_hello" => {
                let args = request.arguments.unwrap_or_default();
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SimpleError::InvalidParameter("name is required".to_string()))?
                    .to_string();
                let greeting = args
                    .get("greeting")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                self.tool_say_hello(name, greeting).await
            }
            "count_greetings" => self.tool_count_greetings().await,
            _ => {
                warn!(tool = request.name, "Unknown tool requested");
                Err(SimpleError::InvalidParameter(format!(
                    "Unknown tool: {}",
                    request.name
                )))
            }
        }
    }

    // Simplified default implementations
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
        Err(SimpleError::InvalidParameter(format!(
            "Resource not found: {}",
            request.uri
        )))
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
        Err(SimpleError::InvalidParameter(format!(
            "Prompt not found: {}",
            request.name
        )))
    }
}

/// Builder pattern for easier server creation - this shows the direction we're heading
impl SimpleHelloWorld {
    /// Fluent API: serve using stdio (like RMCP's simple API)
    pub async fn serve_stdio(
        self,
    ) -> std::result::Result<McpServer<Self>, Box<dyn std::error::Error>> {
        let server_config = ServerConfig {
            server_info: self.get_server_info(),
            transport_config: TransportConfig::Stdio,
            ..Default::default()
        };

        McpServer::new(self, server_config)
            .await
            .map_err(Into::into)
    }

    /// Fluent API: serve using HTTP on specified port
    pub async fn serve_http(
        self,
        port: u16,
    ) -> std::result::Result<McpServer<Self>, Box<dyn std::error::Error>> {
        let server_config = ServerConfig {
            server_info: self.get_server_info(),
            transport_config: TransportConfig::Http {
                host: Some("127.0.0.1".to_string()),
                port,
            },
            ..Default::default()
        };

        McpServer::new(self, server_config)
            .await
            .map_err(Into::into)
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("🚀 Starting Simple Hello World MCP Server");

    // This demonstrates the simplified API we're building towards
    // Compare this single line to the complex setup in the original example
    let mut server = SimpleHelloWorld::new().serve_stdio().await?;

    info!("✅ Simple Hello World MCP Server started successfully");
    info!("💡 Available tools: say_hello, count_greetings");
    info!("🔗 Connect using any MCP client via stdio transport");
    info!("📊 This example shows ~50% less code than the original");

    // Run server until shutdown
    server
        .run()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    info!("👋 Simple Hello World MCP Server stopped");
    Ok(())
}
