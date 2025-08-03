# STDIO Transport Logging Fix Example

This example demonstrates the **correct** way to configure logging for MCP servers using stdio transport to ensure compatibility with MCP clients like the inspector and Claude Desktop.

## The Problem

When using stdio transport, **only JSON-RPC messages should go to stdout**. If logging messages are written to stdout, they will be interpreted as malformed JSON-RPC messages, causing errors like:

```
Error from MCP server: SyntaxError: Unexpected token '2025-0"... is not valid JSON
```

## The Solution

Use `HelloWorldServer::configure_stdio_logging()` before starting the server to automatically configure tracing to output to stderr instead of stdout.

## Required Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
pulseengine-mcp-macros = { version = "0.7.1", features = ["stdio-logging"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Correct Usage

```rust
use pulseengine_mcp_macros::{mcp_server, mcp_tools};

#[mcp_server(name = "My Server")]
#[derive(Default, Clone)]
pub struct MyServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CRITICAL: Configure logging to stderr before starting stdio transport
    MyServer::configure_stdio_logging();
    
    let server = MyServer::with_defaults().serve_stdio().await?;
    server.run().await?;
    Ok(())
}
```

## What This Fixes

- ✅ MCP inspector compatibility
- ✅ Claude Desktop compatibility  
- ✅ All MCP client compatibility
- ✅ Proper separation of logs (stderr) and protocol (stdout)

## Alternative Manual Configuration

If you prefer manual control:

```rust
// Configure tracing to stderr manually
tracing_subscriber::fmt()
    .with_writer(std::io::stderr)
    .with_env_filter("info")
    .init();

let server = MyServer::with_defaults().serve_stdio().await?;
```

## Testing

Test with MCP inspector:

```bash
# Build your server
cargo build --release

# Test with MCP inspector  
npx @modelcontextprotocol/inspector@latest
# Then enter your server binary path when prompted
```

Your server should now work correctly with the inspector without JSON parsing errors!