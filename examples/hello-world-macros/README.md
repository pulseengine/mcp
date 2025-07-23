# Hello World MCP Server with Macros

This example demonstrates the new macro-driven development experience for PulseEngine MCP, inspired by the simplicity of the official RMCP SDK.

## Features Showcased

- **`#[mcp_server]`**: Complete server generation from a simple struct
- **`#[mcp_tool]`**: Automatic tool definition generation from functions
- **Fluent Builder API**: One-line server creation with `.serve_stdio()`
- **Zero Boilerplate**: Focus on business logic, not protocol details

## Comparison

### Before (Original PulseEngine MCP)
```rust
// 280+ lines of manual implementation
pub struct HelloWorldBackend { /* ... */ }

#[async_trait]
impl McpBackend for HelloWorldBackend {
    // 50+ lines of manual trait implementation
    async fn list_tools(&self, request: PaginatedRequestParam) -> Result<ListToolsResult, Self::Error> {
        let tools = vec![
            Tool {
                name: "say_hello".to_string(),
                description: "Say hello to someone".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "The name to greet"},
                        "greeting": {"type": "string", "description": "Custom greeting", "default": "Hello"}
                    },
                    "required": ["name"]
                }),
                output_schema: None,
            },
            // More manual tool definitions...
        ];
        // More manual implementation...
    }
    // More methods...
}
```

### After (With Macros)
```rust
// 10 lines of actual business logic
#[mcp_server(name = "Hello World Macros")]
#[derive(Default)]
struct HelloWorldMacros {
    greeting_count: AtomicU64,
}

impl HelloWorldMacros {
    #[mcp_tool(description = "Say hello to someone")]
    async fn say_hello(&self, name: String, greeting: Option<String>) -> String {
        format!("{}, {}!", greeting.unwrap_or("Hello".to_string()), name)
    }
}

// Usage: HelloWorldMacros::default().serve_stdio().await?
```

## Running the Example

```bash
cargo run --bin hello-world-macros
```

## Benefits

- **90% less code**: From 280+ lines to ~30 lines
- **Type-safe**: Automatic JSON schema generation from Rust types
- **Self-documenting**: Function docs become tool descriptions
- **Progressive complexity**: Start simple, add enterprise features as needed
- **Maintainable**: Less code to debug and maintain

## Architecture

The macro system provides multiple layers of abstraction:

1. **`#[mcp_tool]`**: Converts functions to MCP tools
2. **`#[mcp_server]`**: Generates complete server infrastructure
3. **Fluent API**: Provides simple `.serve_*()` methods
4. **Auto-detection**: Smart defaults based on function signatures

This maintains all PulseEngine enterprise capabilities while matching the developer experience of the official RMCP SDK.