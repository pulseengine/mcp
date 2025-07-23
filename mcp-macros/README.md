# PulseEngine MCP Macros

Procedural macros for the PulseEngine MCP Framework that dramatically simplify server and tool development.

## Overview

This crate provides macros that reduce boilerplate code and enable a more developer-friendly experience while maintaining the enterprise-grade capabilities of PulseEngine MCP.

## Macros

### `#[mcp_tool]`

Automatically generates MCP tool definitions from Rust functions:

```rust
use pulseengine_mcp_macros::mcp_tool;

#[mcp_tool(description = "Say hello to someone")]
async fn say_hello(name: String, greeting: Option<String>) -> String {
    format!("{}, {}!", greeting.unwrap_or("Hello"), name)
}
```

### `#[mcp_backend]`

Auto-implements the `McpBackend` trait:

```rust
use pulseengine_mcp_macros::mcp_backend;

#[mcp_backend(name = "Hello World Server")]
struct HelloWorldBackend;
```

### `#[mcp_server]`

Complete server generation from a simple struct:

```rust
use pulseengine_mcp_macros::mcp_server;

#[mcp_server(name = "My Server")]
struct MyServer;
```

## Features

- **Zero Boilerplate**: Focus on business logic, not protocol details
- **Type Safety**: Compile-time validation of tool definitions
- **Auto Schema Generation**: JSON schemas derived from Rust types
- **Doc Comments**: Function documentation becomes tool descriptions
- **Progressive Complexity**: Start simple, add enterprise features as needed

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
pulseengine-mcp-macros = "0.5"
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.