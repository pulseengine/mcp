# PulseEngine MCP Macros: API Reference

Complete reference documentation for all PulseEngine MCP macro attributes, generated code, and APIs.

## Server Macro: `#[mcp_server]`

The `#[mcp_server]` macro transforms a Rust struct into a fully-featured MCP server.

### Syntax

```rust
#[mcp_server(
    name = "Server Name",           // Required: Display name
    version = "1.0.0",              // Optional: Version (defaults to Cargo.toml)
    description = "Description",     // Optional: Description (defaults to doc comments)
    app_name = "app-id"             // Optional: Application-specific storage isolation
)]
struct MyServer {
    // Your server state
}
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | `String` | ✅ | Human-readable server name displayed to clients |
| `version` | `String` | ❌ | Server version (defaults to crate version) |
| `description` | `String` | ❌ | Server description (defaults to struct doc comments) |
| `app_name` | `String` | ❌ | Application identifier for storage isolation |

### Generated Code

The macro generates:

- **Server Implementation**: Full MCP protocol compliance
- **Configuration Struct**: `{ServerName}Config` with server settings
- **Factory Methods**: `with_defaults()`, `with_config()`, `new()`
- **Transport Methods**: `serve_stdio()`, `serve_http()`, `serve_ws()`
- **Health Check**: Built-in health monitoring
- **Capability Detection**: Automatic feature discovery

### Example

```rust
use pulseengine_mcp_macros::mcp_server;

#[mcp_server(
    name = "File Manager Server",
    version = "2.1.0",
    description = "Advanced file management with security",
    app_name = "filemanager"
)]
#[derive(Default, Clone)]
struct FileManagerServer {
    root_path: std::path::PathBuf,
    permissions: std::collections::HashMap<String, Vec<String>>,
}
```

## Tool Macro: `#[mcp_tool]`

The `#[mcp_tool]` macro exposes struct methods as MCP tools.

### Syntax

```rust
#[mcp_tool]
impl MyServer {
    /// Tool description from doc comments
    async fn tool_name(&self, param: Type) -> Result<ReturnType, ErrorType> {
        // Implementation
    }
}
```

### Method Requirements

- **Self Parameter**: Must take `&self` as first parameter
- **Async/Sync**: Both `async` and synchronous methods supported
- **Parameters**: All parameter types must implement `serde::Deserialize`
- **Return Types**: Must implement `serde::Serialize` or be `Result<T, E>` where `T: Serialize`
- **Documentation**: Doc comments become tool descriptions

### Supported Parameter Types

| Type Category | Examples | Notes |
|---------------|----------|-------|
| **Primitives** | `i32`, `u64`, `f64`, `bool`, `String` | Direct JSON mapping |
| **Options** | `Option<T>` | Optional parameters |
| **Collections** | `Vec<T>`, `HashMap<K, V>` | JSON arrays/objects |
| **Custom Types** | Structs with `#[derive(Deserialize)]` | Complex nested data |
| **Enums** | `#[derive(Deserialize)]` enums | Tagged or untagged variants |

### Supported Return Types

| Type Category | Examples | Notes |
|---------------|----------|-------|
| **Direct** | `String`, `i32`, `CustomStruct` | Serialized directly |
| **Results** | `Result<T, E>` | Errors converted to MCP errors |
| **Options** | `Option<T>` | `null` for `None` |
| **Collections** | `Vec<T>`, `HashMap<K, V>` | JSON arrays/objects |

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error("Not found: {id}")]
    NotFound { id: u64 },
    #[error("Validation failed: {reason}")]
    Validation { reason: String },
}

#[mcp_tool]
impl MyServer {
    async fn risky_operation(&self, id: u64) -> Result<String, MyError> {
        // Errors automatically converted to MCP protocol errors
    }
}
```

## Resource Macro: `#[mcp_resource]`

The `#[mcp_resource]` macro creates MCP resources with URI template matching.

### Syntax

```rust
#[mcp_resource(
    uri_template = "scheme://{param1}/{param2}",  // Required: URI pattern
    name = "resource_name",                       // Optional: Resource name
    description = "Resource description",         // Optional: Description
    mime_type = "application/json"                // Optional: Content type
)]
impl MyServer {
    async fn resource_handler(&self, param1: String, param2: String) -> Result<T, E> {
        // Implementation
    }
}
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `uri_template` | `String` | ✅ | URI pattern with `{param}` placeholders |
| `name` | `String` | ❌ | Resource identifier (defaults to method name) |
| `description` | `String` | ❌ | Resource description (defaults to doc comments) |
| `mime_type` | `String` | ❌ | Content MIME type (defaults to auto-detection) |

### URI Template Syntax

- **Parameters**: `{param_name}` extracts path segments
- **Schemes**: Any scheme supported (`file://`, `http://`, `custom://`)
- **Paths**: Static and dynamic path segments
- **Validation**: Automatic parameter extraction and validation

### Examples

```rust
#[mcp_resource(uri_template = "file://{path}")]
impl MyServer {
    /// Read file contents
    async fn read_file(&self, path: String) -> Result<String, std::io::Error> {
        tokio::fs::read_to_string(&path).await
    }
}

#[mcp_resource(
    uri_template = "api://{version}/{endpoint}/{id}",
    mime_type = "application/json",
    description = "REST API resource access"
)]
impl MyServer {
    async fn api_resource(&self, version: String, endpoint: String, id: String) -> Result<serde_json::Value, std::io::Error> {
        // API call implementation
    }
}
```

## Prompt Macro: `#[mcp_prompt]`

The `#[mcp_prompt]` macro creates prompt templates for AI interactions.

### Syntax

```rust
#[mcp_prompt(
    name = "prompt_name",                          // Required: Prompt identifier
    description = "Prompt description",            // Optional: Description
    arguments = ["arg1", "arg2"]                   // Optional: Argument names
)]
impl MyServer {
    async fn prompt_handler(&self, arg1: String, arg2: String) -> Result<PromptMessage, E> {
        // Implementation
    }
}
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | `String` | ✅ | Prompt identifier for client requests |
| `description` | `String` | ❌ | Prompt description (defaults to doc comments) |
| `arguments` | `[String]` | ❌ | Expected argument names for validation |

### Return Type

Must return `Result<PromptMessage, E>` where:

```rust
pub struct PromptMessage {
    pub role: Role,
    pub content: PromptContent,
}

pub enum Role {
    User,
    Assistant,
    System,
}

pub enum PromptContent {
    Text { text: String },
    Image { data: String, mime_type: String },
}
```

### Examples

```rust
use pulseengine_mcp_protocol::{PromptMessage, Role, PromptContent};

#[mcp_prompt(name = "code_review")]
impl MyServer {
    /// Generate code review prompts
    async fn code_review_prompt(&self, code: String, language: String) -> Result<PromptMessage, std::io::Error> {
        Ok(PromptMessage {
            role: Role::User,
            content: PromptContent::Text {
                text: format!("Please review this {} code:\n\n```{}\n{}\n```", language, language, code),
            },
        })
    }
}

#[mcp_prompt(
    name = "data_analysis",
    description = "Generate data analysis prompts",
    arguments = ["data", "analysis_type", "focus_areas"]
)]
impl MyServer {
    async fn analysis_prompt(&self, data: serde_json::Value, analysis_type: String, focus_areas: Vec<String>) -> Result<PromptMessage, std::io::Error> {
        let focus_text = focus_areas.join(", ");
        let prompt_text = format!(
            "Analyze this data with focus on {}:\n\nAnalysis type: {}\nData: {}\n\nProvide insights and recommendations.",
            focus_text, analysis_type, serde_json::to_string_pretty(&data)?
        );
        
        Ok(PromptMessage {
            role: Role::User,
            content: PromptContent::Text { text: prompt_text },
        })
    }
}
```

## Generated Server API

Every `#[mcp_server]` struct generates a comprehensive API:

### Core Methods

```rust
impl MyServer {
    // Factory methods
    fn with_defaults() -> Self;
    fn with_config(config: MyServerConfig) -> Self;
    fn new() -> Self;
    
    // Server information
    fn get_server_info(&self) -> ServerInfo;
    
    // Transport methods
    async fn serve_stdio(&self) -> Result<impl McpService, Error>;
    async fn serve_http(&self, port: u16) -> Result<impl McpService, Error>;
    async fn serve_ws(&self, addr: impl ToSocketAddrs) -> Result<impl McpService, Error>;
    
    // Health check
    async fn health_check(&self) -> Result<(), Error>;
}
```

### MCP Backend Implementation

```rust
impl McpBackend for MyServer {
    // Tool operations
    async fn list_tools(&self, params: ListToolsParams) -> Result<ListToolsResult, Error>;
    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, Error>;
    
    // Resource operations  
    async fn list_resources(&self, params: ListResourcesParams) -> Result<ListResourcesResult, Error>;
    async fn read_resource(&self, params: ReadResourceParams) -> Result<ReadResourceResult, Error>;
    
    // Prompt operations
    async fn list_prompts(&self, params: ListPromptsParams) -> Result<ListPromptsResult, Error>;
    async fn get_prompt(&self, params: GetPromptParams) -> Result<GetPromptResult, Error>;
    
    // Logging operations
    async fn set_logging_level(&self, params: SetLoggingLevelParams) -> Result<(), Error>;
}
```

### Configuration Struct

```rust
#[derive(Debug, Clone)]
pub struct MyServerConfig {
    pub server_name: String,
    pub server_version: String,
    pub server_description: Option<String>,
    pub app_name: Option<String>,
    pub log_level: LogLevel,
    pub max_request_size: usize,
    pub timeout: Duration,
    // Additional fields based on your server
}

impl Default for MyServerConfig {
    fn default() -> Self {
        // Sensible defaults
    }
}

impl MyServerConfig {
    pub fn builder() -> MyServerConfigBuilder;
    
    #[cfg(feature = "auth")]
    pub fn get_auth_config() -> AuthConfig;
}
```

## Capability Detection

The macro system automatically detects and enables MCP capabilities:

### Automatic Detection

- **Tools**: Enabled when `#[mcp_tool]` implementations found
- **Resources**: Enabled when `#[mcp_resource]` implementations found  
- **Prompts**: Enabled when `#[mcp_prompt]` implementations found
- **Logging**: Always enabled with configurable levels

### Manual Override

```rust
impl MyServer {
    fn override_capabilities(&self) -> Capabilities {
        Capabilities {
            tools: Some(ToolsCapability { list_changed: true }),
            resources: Some(ResourcesCapability { subscribe: false, list_changed: true }),
            prompts: Some(PromptsCapability { list_changed: true }),
            logging: Some(LoggingCapability {}),
        }
    }
}
```

## Error Handling

### Automatic Error Conversion

All tool, resource, and prompt methods can return `Result<T, E>` where `E` implements `std::error::Error`. Errors are automatically converted to appropriate MCP protocol errors.

### Custom Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyServerError {
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },
    
    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },
    
    #[error("Invalid input: {field}")]
    InvalidInput { field: String },
    
    #[error("Internal error: {source}")]
    Internal { #[from] source: Box<dyn std::error::Error + Send + Sync> },
}
```

### Error Mapping

| Rust Error Kind | MCP Error Code | Description |
|------------------|----------------|-------------|
| `InvalidInput` | `-32602` | Invalid parameters |
| `NotFound` | `-32001` | Resource/method not found |
| `PermissionDenied` | `-32003` | Access denied |
| `Other` | `-32000` | Internal error |

## Type System Integration

### Serialization Requirements

- **Parameters**: Must implement `serde::Deserialize`
- **Return Values**: Must implement `serde::Serialize`
- **Error Types**: Must implement `std::error::Error + Send + Sync`

### Complex Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub has_more: bool,
}
```

## Application-Specific Configuration

When using `app_name`, the server creates isolated storage and configuration:

### Storage Isolation

- **Config Path**: `~/.pulseengine/{app_name}/config/`
- **Data Path**: `~/.pulseengine/{app_name}/data/`
- **Cache Path**: `~/.pulseengine/{app_name}/cache/`
- **Logs Path**: `~/.pulseengine/{app_name}/logs/`

### Authentication Integration

```rust
#[cfg(feature = "auth")]
impl MyServer {
    fn get_auth_manager(&self) -> &AuthManager {
        // App-specific auth manager
    }
    
    fn verify_api_key(&self, key: &str) -> Result<bool, AuthError> {
        // App-specific key validation
    }
}
```

## Threading and Concurrency

All generated servers are:

- **Clone**: Can be safely cloned and shared
- **Send + Sync**: Can be used across thread boundaries
- **Thread-Safe**: Internal state properly synchronized

### Concurrent Access Patterns

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[mcp_server(name = "Concurrent Server")]
#[derive(Clone)]
struct ConcurrentServer {
    shared_state: Arc<RwLock<HashMap<String, String>>>,
}

#[mcp_tool]
impl ConcurrentServer {
    async fn concurrent_operation(&self, key: String) -> Result<String, std::io::Error> {
        let state = self.shared_state.read().await;
        Ok(state.get(&key).cloned().unwrap_or_default())
    }
}
```

## Performance Considerations

### Memory Usage

- **Zero-Copy**: URI template parsing avoids unnecessary allocations
- **Efficient Serialization**: Direct serde integration
- **Lazy Initialization**: Resources loaded on-demand

### Async Performance

- **Tokio Integration**: Full async/await support
- **Connection Pooling**: Automatic for HTTP/WebSocket transports
- **Backpressure**: Built-in flow control

### Benchmarking

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn benchmark_tool_call(c: &mut Criterion) {
        let server = MyServer::with_defaults();
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        c.bench_function("tool_call", |b| {
            b.iter(|| {
                rt.block_on(async {
                    black_box(server.my_tool("test".to_string()).await)
                })
            })
        });
    }

    criterion_group!(benches, benchmark_tool_call);
    criterion_main!(benches);
}
```

---

This API reference provides complete documentation for all macro features and generated code. For practical examples and patterns, see the [Macro Guide](./MACRO_GUIDE.md) and [Advanced Patterns](./ADVANCED_PATTERNS.md).