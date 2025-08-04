# PulseEngine MCP Macros

Procedural macros for the PulseEngine MCP Framework that dramatically simplify server and tool development.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
# Core macros
pulseengine-mcp-macros = "0.7.1"

# Required dependencies for generated code
pulseengine-mcp-protocol = "0.7.1"
pulseengine-mcp-server = "0.7.1"  
pulseengine-mcp-transport = "0.7.1"
async-trait = "0.1"
thiserror = "1.0"
tokio = { version = "1.0", features = ["full"] }

# For STDIO transport (recommended)
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Basic Server Example

```rust
use pulseengine_mcp_macros::{mcp_server, mcp_tools};

#[mcp_server(name = "Hello Server")]
#[derive(Default, Clone)]
pub struct HelloServer;

#[mcp_tools]
impl HelloServer {
    /// Say hello to someone
    pub async fn hello(&self, name: Option<String>) -> anyhow::Result<String> {
        Ok(format!("Hello, {}!", name.unwrap_or_else(|| "World".to_string())))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CRITICAL for STDIO: Configure logging to stderr
    HelloServer::configure_stdio_logging();
    
    let server = HelloServer::with_defaults().serve_stdio().await?;
    server.run().await?;
    Ok(())
}
```

## Required Dependencies

### Always Required

These dependencies are **always required** when using `#[mcp_server]`:

```toml
pulseengine-mcp-protocol = "0.7.1"
pulseengine-mcp-server = "0.7.1"
pulseengine-mcp-transport = "0.7.1"
async-trait = "0.1"
thiserror = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Feature-Based Dependencies

#### STDIO Logging (`stdio-logging` feature)

For STDIO transport compatibility:

```toml
pulseengine-mcp-macros = { version = "0.7.1", features = ["stdio-logging"] }
tracing-subscriber = "0.3"
```

#### Authentication (`auth` feature + auth parameter)

**Important**: Authentication is **opt-in**. By default, servers have no authentication.

When using `auth = "memory"`, `auth = "file"`, etc.:

```toml
pulseengine-mcp-macros = { version = "0.7.1", features = ["auth"] }
pulseengine-mcp-auth = "0.7.1"
```

## Macros

### `#[mcp_server]`

Generates a complete MCP server from a struct:

```rust
#[mcp_server(
    name = "My Server",
    version = "1.0.0",
    description = "Server description",
    auth = "memory"  // Optional: "memory", "file", "disabled"
)]
#[derive(Default, Clone)]
pub struct MyServer;
```

**Generated methods:**
- `MyServer::with_defaults()` - Create instance
- `MyServer::serve_stdio()` - Start STDIO transport
- `MyServer::serve_http(port)` - Start HTTP transport  
- `MyServer::configure_stdio_logging()` - Fix STDIO logging

### `#[mcp_tools]`

Automatically discovers public methods as MCP tools:

```rust
#[mcp_tools]
impl MyServer {
    /// Tool description from doc comment
    pub async fn my_tool(&self, param: String) -> anyhow::Result<String> {
        Ok(format!("Result: {}", param))
    }
}
```

**Note**: Currently passthrough - full implementation coming soon.

## Transport Types

### STDIO (Default)

For MCP clients like Claude Desktop:

```rust
// CRITICAL: Configure logging first!
MyServer::configure_stdio_logging();
let server = MyServer::with_defaults().serve_stdio().await?;
```

### HTTP

For web-based clients:

```rust
let server = MyServer::with_defaults().serve_http(8080).await?;
```

### WebSocket

For real-time clients:

```rust
let server = MyServer::with_defaults().serve_websocket(8080).await?;
```

## Authentication

**By default, servers have no authentication**. To enable authentication, add the `auth` parameter:

```rust
// Memory-based (development)
#[mcp_server(name = "Dev Server", auth = "memory")]

// File-based (production)  
#[mcp_server(name = "Prod Server", auth = "file", app_name = "my-app")]

// Explicitly disabled (same as default)
#[mcp_server(name = "Test Server", auth = "disabled")]

// No auth parameter = no authentication (default)
#[mcp_server(name = "Simple Server")]
```

**Requires**: `pulseengine-mcp-auth` dependency and `auth` feature only when `auth` parameter is specified.

## STDIO Transport Critical Note

⚠️ **CRITICAL**: For STDIO transport, you **MUST** configure logging to go to stderr, not stdout. Stdout is reserved for JSON-RPC messages only.

```rust
// Always call this before serve_stdio()
MyServer::configure_stdio_logging();
```

Failure to do this will break MCP client compatibility with errors like:
```
Error from MCP server: SyntaxError: Unexpected token '2025-0"... is not valid JSON
```

## Complete Example

See [`examples/hello-world-stdio-fixed/`](../examples/hello-world-stdio-fixed/) for a working example with proper STDIO logging configuration.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
