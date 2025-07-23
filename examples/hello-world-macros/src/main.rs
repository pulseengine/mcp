//! Hello World MCP Server Example Using Macros
//!
//! This demonstrates how the macro system simplifies MCP server development
//! while maintaining enterprise capabilities.
//!
//! This example shows the macro-generated server infrastructure without
//! conflicting manual implementations.

use pulseengine_mcp_macros::mcp_server;
use pulseengine_mcp_server::McpBackend;
use pulseengine_mcp_protocol::{Tool, CallToolRequestParam, CallToolResult, Content};
use serde_json::json;
use std::sync::{atomic::{AtomicU64, Ordering}, Arc};

/// A simple greeting server that showcases the macro-driven API
/// 
/// This server demonstrates:
/// - Automatic backend trait implementation via #[mcp_server]
/// - Type-safe error handling
/// - Fluent builder API for server creation
/// - Smart defaults with enterprise capabilities
/// - Manual tool integration (until automatic tool discovery is implemented)
#[mcp_server(name = "Hello World Macros", description = "Demonstrates the new macro system")]
#[derive(Clone)]
struct HelloWorldMacros {
    #[allow(dead_code)]
    greeting_count: Arc<AtomicU64>,
}

impl Default for HelloWorldMacros {
    fn default() -> Self {
        Self {
            greeting_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

// Business logic methods - these would be exposed as tools in a complete implementation
impl HelloWorldMacros {
    /// Say hello to someone with a customizable greeting
    #[allow(dead_code)]
    pub async fn say_hello(&self, name: String, greeting: Option<String>) -> String {
        let greeting = greeting.unwrap_or_else(|| "Hello".to_string());
        let count = self.greeting_count.fetch_add(1, Ordering::Relaxed) + 1;
        
        tracing::info!(
            tool = "say_hello",
            name = %name,
            greeting = %greeting,
            count = count,
            "Generated greeting"
        );
        
        format!("{greeting}, {name}! 👋 (Greeting #{count})")
    }

    /// Get the total number of greetings sent
    #[allow(dead_code)]
    pub async fn count_greetings(&self) -> u64 {
        let count = self.greeting_count.load(Ordering::Relaxed);
        
        tracing::info!(
            tool = "count_greetings", 
            count = count,
            "Retrieved greeting count"
        );
        
        count
    }

    /// Generate a random greeting in different languages
    #[allow(dead_code)]
    pub async fn random_greeting(&self) -> String {
        let greetings = ["Hello", "Hola", "Bonjour", "Guten Tag", 
            "Ciao", "こんにちは", "안녕하세요", "Привет"];
        
        let random_index = self.greeting_count.load(Ordering::Relaxed) as usize % greetings.len();
        let greeting = greetings[random_index];
        
        tracing::info!(
            tool = "random_greeting",
            greeting = %greeting,
            "Generated random greeting"
        );
        
        greeting.to_string()
    }
}

// Override the tool registry methods to wire up our custom tools using the trait
impl McpToolProvider for HelloWorldMacros {
    /// Register all tools - manually wired until automatic discovery is implemented
    fn register_tools(&self, tools: &mut Vec<Tool>) {
        tools.push(Tool {
            name: "say_hello".to_string(),
            description: "Say hello to someone with a customizable greeting".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Name to greet"},
                    "greeting": {"type": "string", "description": "Custom greeting (optional)"}
                },
                "required": ["name"]
            }),
            output_schema: None,
        });
        
        tools.push(Tool {
            name: "count_greetings".to_string(),
            description: "Get the total number of greetings sent".to_string(),
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: None,
        });
        
        tools.push(Tool {
            name: "random_greeting".to_string(),
            description: "Generate a random greeting in different languages".to_string(),
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: None,
        });
    }
    
    /// Dispatch tool calls to appropriate handlers
    fn dispatch_tool_call(
        &self,
        request: CallToolRequestParam,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<CallToolResult, pulseengine_mcp_protocol::Error>> + Send + '_>> {
        Box::pin(async move {
        match request.name.as_str() {
            "say_hello" => {
                let args = request.arguments.unwrap_or_default();
                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| pulseengine_mcp_protocol::Error::invalid_params("name is required"))?
                    .to_string();
                let greeting = args.get("greeting").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                let result = self.say_hello(name, greeting).await;
                
                Ok(CallToolResult {
                    content: vec![Content::text(result)],
                    is_error: Some(false),
                    structured_content: None,
                })
            }
            "count_greetings" => {
                let result = self.count_greetings().await;
                
                Ok(CallToolResult {
                    content: vec![Content::text(format!("Total greetings: {result}"))],
                    is_error: Some(false),
                    structured_content: None,
                })
            }
            "random_greeting" => {
                let result = self.random_greeting().await;
                
                Ok(CallToolResult {
                    content: vec![Content::text(result)],
                    is_error: Some(false),
                    structured_content: None,
                })
            }
            _ => Err(pulseengine_mcp_protocol::Error::invalid_params(
                format!("Unknown tool: {}", request.name)
            ))
        }
        })
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

    tracing::info!("🚀 Starting Hello World Macros MCP Server");

    // This demonstrates the macro-generated fluent API
    // The #[mcp_server] macro generates:
    // - Complete McpBackend implementation
    // - Error types and conversions
    // - Configuration management
    // - Fluent builder methods like .serve_stdio()
    let server = HelloWorldMacros::with_defaults()
        .serve_stdio()
        .await?;

    tracing::info!("✅ Hello World Macros MCP Server started successfully");
    tracing::info!("💡 Server demonstrates macro-generated infrastructure");
    tracing::info!("🔗 Connect using any MCP client via stdio transport");
    tracing::info!("📝 Note: Tool implementations would use #[mcp_tool] in practice");

    // Run the server - this uses the macro-generated service wrapper
    server.run().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    tracing::info!("👋 Hello World Macros MCP Server stopped");
    Ok(())
}