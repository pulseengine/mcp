# PulseEngine MCP

Rust framework for building [Model Context Protocol](https://modelcontextprotocol.io/) servers and clients.

[![Crates.io](https://img.shields.io/crates/v/pulseengine-mcp-protocol.svg)](https://crates.io/crates/pulseengine-mcp-protocol)
[![Documentation](https://docs.rs/pulseengine-mcp-protocol/badge.svg)](https://docs.rs/pulseengine-mcp-protocol)
[![CI](https://github.com/pulseengine/mcp/actions/workflows/pr-validation.yml/badge.svg)](https://github.com/pulseengine/mcp/actions/workflows/pr-validation.yml)
[![codecov](https://codecov.io/gh/pulseengine/mcp/graph/badge.svg?token=ZGAL6V3SQR)](https://codecov.io/gh/pulseengine/mcp)

## Example

```rust
use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GreetParams {
    pub name: Option<String>,
}

#[mcp_server(name = "My Server")]
#[derive(Default, Clone)]
pub struct MyServer;

#[mcp_tools]
impl MyServer {
    /// Greet someone by name
    pub async fn greet(&self, params: GreetParams) -> anyhow::Result<String> {
        let name = params.name.unwrap_or_else(|| "World".to_string());
        Ok(format!("Hello, {name}!"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    MyServer::configure_stdio_logging();
    MyServer::with_defaults().serve_stdio().await?.run().await
}
```

The `#[mcp_server]` and `#[mcp_tools]` macros generate the protocol implementation. Tool schemas are derived from your Rust types via `JsonSchema`.

## Crates

| Crate                           | Description                                        |
| ------------------------------- | -------------------------------------------------- |
| [mcp-protocol](mcp-protocol/)   | MCP types, JSON-RPC, schema validation             |
| [mcp-server](mcp-server/)       | Server infrastructure with `McpBackend` trait      |
| [mcp-client](mcp-client/)       | Client for connecting to MCP servers               |
| [mcp-transport](mcp-transport/) | stdio, HTTP, WebSocket transports                  |
| [mcp-auth](mcp-auth/)           | Authentication, API keys, OAuth 2.1                |
| [mcp-security](mcp-security/)   | Input validation, rate limiting                    |
| [mcp-logging](mcp-logging/)     | Structured logging with credential sanitization    |
| [mcp-macros](mcp-macros/)       | `#[mcp_server]`, `#[mcp_tools]`, `#[mcp_resource]` |

## Examples

- [hello-world](examples/hello-world/) - Minimal server
- [hello-world-with-auth](examples/hello-world-with-auth/) - With authentication
- [resources-demo](examples/resources-demo/) - Resource templates with `#[mcp_resource]`
- [ui-enabled-server](examples/ui-enabled-server/) - MCP Apps extension (SEP-1865)

## MCP Spec

Implements MCP 2025-11-25: tools, resources, prompts, completions, sampling, roots, logging, progress, cancellation, tasks, and elicitation.

## License

MIT OR Apache-2.0
