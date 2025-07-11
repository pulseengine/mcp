# PulseEngine: AI + WebAssembly Infrastructure (Work in Progress)

**Building the intersection of AI and WebAssembly - from safety-critical runtimes to AI-native applications**

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

## About PulseEngine

PulseEngine is developing infrastructure at the intersection of AI and WebAssembly. Our projects range from experimental proposals to battle-tested implementations, all focused on enabling AI systems to safely interact with real-world systems through WebAssembly's security model.

## Our Projects

### 🔧 [wrt](https://github.com/pulseengine/wrt) - WebAssembly Runtime (95% complete)
A safety-critical WebAssembly runtime supporting both Core WebAssembly and Component Model. Designed for ASIL-B compliance with features like Control Flow Integrity, fuel-based execution limiting, and support for no_std environments. Nearly production-ready for embedded and safety-critical applications.

### 🤖 [mcp](https://github.com/pulseengine/mcp) - Rust MCP Framework (Production-tested)
This framework provides everything you need to build Model Context Protocol servers in Rust. Extracted from a real-world home automation server with 30+ tools that successfully integrates with MCP Inspector, Claude Desktop, and HTTP clients. The most mature of our projects.

### 🎨 [glsp-mcp](https://github.com/pulseengine/glsp-mcp) - AI-Native Graphical Modeling (MVP)
World's first AI-native implementation of the Graphical Language Server Protocol using MCP. Enables AI agents to create and manipulate diagrams through natural language. Includes a WebAssembly-based ADAS demo showing how WASM components can simulate automotive systems. Functional MVP demonstrating the concept.

### 🏗️ [rules_wasm_component](https://github.com/pulseengine/rules_wasm_component) - Bazel Build Rules (Active Development)
Modern Bazel rules for building WebAssembly components with multi-profile support. Features 73% faster builds and 99% less disk usage through symlink optimization. Supports Rust, C/C++, and provides seamless integration with the Component Model toolchain.

### 📐 [wasi-mcp](https://github.com/pulseengine/wasi-mcp) - WASI API Proposal (Early Stage)
A proposed WebAssembly System Interface API for the Model Context Protocol, targeting WASI Preview3. This early-stage proposal aims to standardize how WebAssembly components can act as MCP servers and clients, leveraging WASI's security model for safe AI integration.

## Our Vision

We're exploring what happens when AI systems can safely interact with real-world systems through WebAssembly's security sandbox. Our work spans:

- **Safety-Critical Systems**: Building infrastructure suitable for automotive, medical, and industrial applications
- **AI-Native Development**: Creating tools that assume AI agents are first-class participants
- **Secure Integration**: Using WebAssembly's capability-based security for safe AI-to-system interaction
- **Universal Portability**: Write once, run anywhere - from embedded devices to cloud servers

## Project Status

- ✅ **Production-Ready**: MCP Rust framework (extracted from working system)
- 🔄 **Nearly Complete**: WRT WebAssembly runtime (95% done, fixing final issues)
- 🚧 **Active Development**: Bazel build rules, GLSP-MCP platform
- 📝 **Early Proposal**: WASI-MCP standardization effort

## The Rest of This README: MCP Framework Details

Below you'll find the original documentation for the MCP framework specifically. For information about our other projects, visit their respective repositories.

---

## What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io/) enables AI assistants to securely connect to and interact with external systems through tools, resources, and prompts. Instead of AI models having static knowledge, they can dynamically access live data and perform actions through MCP servers.

## Why This Framework?

**🏗️ Production-Proven:** This framework powers a working Loxone home automation server that handles real-world complexity - device control, sensor monitoring, authentication, and concurrent operations.

**🔧 Complete Infrastructure:** You focus on your domain logic (databases, APIs, file systems) while the framework handles protocol compliance, transport layers, security, and monitoring.

**📡 Multiple Transport Support:** Works with Claude Desktop (stdio), web applications (HTTP), real-time apps (WebSocket), and tools like MCP Inspector.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
pulseengine-mcp-server = "0.4.1"
pulseengine-mcp-protocol = "0.4.1"
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
```

Create your first MCP server:

```rust
use pulseengine_mcp_server::{McpServer, McpBackend, ServerConfig};
use pulseengine_mcp_protocol::*;
use async_trait::async_trait;

#[derive(Clone)]
struct MyBackend;

#[async_trait]
impl McpBackend for MyBackend {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Config = ();
    
    async fn initialize(_: Self::Config) -> Result<Self, Self::Error> {
        Ok(MyBackend)
    }
    
    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(false) }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "My MCP Server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("A simple example server".to_string()),
        }
    }
    
    async fn list_tools(&self, _: PaginatedRequestParam) -> Result<ListToolsResult, Self::Error> {
        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "hello".to_string(),
                    description: "Say hello to someone".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {"type": "string", "description": "Name to greet"}
                        },
                        "required": ["name"]
                    }),
                }
            ],
            next_cursor: String::new(),
        })
    }
    
    async fn call_tool(&self, request: CallToolRequestParam) -> Result<CallToolResult, Self::Error> {
        match request.name.as_str() {
            "hello" => {
                let name = request.arguments
                    .and_then(|args| args.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");
                
                Ok(CallToolResult {
                    content: vec![Content::text(format!("Hello, {}!", name))],
                    is_error: Some(false),
                })
            }
            _ => Err("Unknown tool".into()),
        }
    }
    
    // Simple implementations for unused features
    async fn list_resources(&self, _: PaginatedRequestParam) -> Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult { resources: vec![], next_cursor: String::new() })
    }
    async fn read_resource(&self, _: ReadResourceRequestParam) -> Result<ReadResourceResult, Self::Error> {
        Err("No resources".into())
    }
    async fn list_prompts(&self, _: PaginatedRequestParam) -> Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult { prompts: vec![], next_cursor: String::new() })
    }
    async fn get_prompt(&self, _: GetPromptRequestParam) -> Result<GetPromptResult, Self::Error> {
        Err("No prompts".into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MyBackend::initialize(()).await?;
    let config = ServerConfig::default();
    let mut server = McpServer::new(backend, config).await?;
    server.run().await?;
    Ok(())
}
```

## Framework Components

### 🔧 [mcp-protocol](mcp-protocol/) - Core Protocol Types
- MCP request/response types with validation
- JSON-RPC 2.0 support and error handling
- Schema validation for tool parameters

### 🏗️ [mcp-server](mcp-server/) - Server Infrastructure  
- Pluggable backend system via `McpBackend` trait
- Request routing and protocol compliance
- Middleware integration for auth, security, monitoring

### 📡 [mcp-transport](mcp-transport/) - Multiple Transports
- stdio (Claude Desktop), HTTP (web apps), WebSocket (real-time)
- MCP Inspector compatibility with content negotiation
- Session management and CORS support

### 🔑 [mcp-auth](mcp-auth/) - Authentication Framework
- API key management with role-based access control
- Rate limiting and IP whitelisting
- Audit logging and security features

### 🛡️ [mcp-security](mcp-security/) - Security Middleware
- Input validation and XSS/injection prevention
- Request size limits and parameter validation
- CORS policies and security headers

### 📊 [mcp-monitoring](mcp-monitoring/) - Observability
- Health checks and metrics collection
- Performance tracking and request tracing
- Integration with monitoring systems

### 📝 [mcp-logging](mcp-logging/) - Structured Logging
- JSON logging with correlation IDs
- Automatic credential sanitization
- Security audit trails

### 🖥️ [mcp-cli](mcp-cli/) & [mcp-cli-derive](mcp-cli-derive/) - CLI Integration
- Command-line interface generation
- Configuration management
- Derive macros for backends

## Examples

### 🌍 [Hello World](examples/hello-world/)
Complete minimal MCP server demonstrating basic concepts.

### 🏗️ [Backend Example](examples/backend-example/)
Shows advanced backend implementation patterns.

### 🖥️ [CLI Example](examples/cli-example/)
Demonstrates CLI integration and configuration.

### 🏠 Real-World Reference: Loxone MCP Server
The framework was extracted from a production Loxone home automation server that provides:

- **30+ Tools** - Complete home automation control (lighting, climate, security, energy)
- **Multiple Transports** - Works with Claude Desktop, MCP Inspector, n8n workflows  
- **Production Security** - API keys, rate limiting, input validation, audit logging
- **Real-Time Integration** - WebSocket support for live device status updates
- **Proven Reliability** - Handles concurrent operations and error conditions

## Development Workflow

### Building the Framework
```bash
# Build all framework crates
cargo build --workspace

# Test all crates
cargo test --workspace

# Run examples
cargo run --bin hello-world-server
```

### Creating Your MCP Server

1. **Choose Your Domain** - What system do you want to make accessible via MCP?
2. **Implement McpBackend** - Define your tools, resources, and prompts
3. **Configure Transport** - stdio for Claude Desktop, HTTP for web clients
4. **Add Security** - Authentication, validation, and monitoring as needed
5. **Deploy** - Native binary, Docker container, or WebAssembly

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Clients   │    │  Your Backend   │    │ External Systems│
│                 │    │                 │    │                 │
│ • Claude Desktop│    │ • Tools         │    │ • Databases     │
│ • MCP Inspector │◄──►│ • Resources     │◄──►│ • APIs          │
│ • Web Apps      │    │ • Prompts       │    │ • File Systems  │
│ • Custom Clients│    │                 │    │ • Hardware      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │
         │              ┌─────────────────┐
         │              │ MCP Framework   │
         │              │                 │
         └──────────────►│ • Protocol      │
                        │ • Transport     │
                        │ • Security      │
                        │ • Monitoring    │
                        └─────────────────┘
```

## Contributing

This framework grows from real-world usage. The most valuable contributions come from:

1. **New Backend Examples** - Show how to integrate different types of systems
2. **Production Patterns** - Share patterns from your own MCP server deployments  
3. **Client Compatibility** - Test with different MCP clients and report issues
4. **Performance Improvements** - Optimizations based on real usage patterns
5. **Security Enhancements** - Better validation, authentication, or audit capabilities

## Community

- **Documentation** - [docs.rs/pulseengine-mcp-protocol](https://docs.rs/pulseengine-mcp-protocol)
- **Issues** - [GitHub Issues](https://github.com/pulseengine/mcp/issues)
- **Discussions** - [GitHub Discussions](https://github.com/pulseengine/mcp/discussions)

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

**Built by developers who needed a robust MCP framework for real production use.** Start building your MCP server today with confidence that the foundation has been proven in demanding real-world scenarios.