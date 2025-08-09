# Hello World Simple - Easy as Pi! 🥧

The simplest possible MCP server that actually works.

## What This Shows

- **20 lines of code** - that's it!
- **6 dependencies** - minimal and clean
- **Works out of the box** - no complex configuration
- **One tool** - `say_hello` that greets people

## How to Run

```bash
cargo run
```

That's it! The server will start and be ready for MCP clients.

## How to Test

Use the MCP inspector:

```bash
npx @modelcontextprotocol/inspector@latest
# Enter the path to: target/debug/hello-world-simple
```

## Code Explained

```rust
#[mcp_server(name = "Hello World")]     // Creates the server
#[derive(Default, Clone)]
pub struct HelloWorld;                  // Your server struct

#[mcp_tools]                           // Auto-discovers tools
impl HelloWorld {
    pub fn say_hello(&self, name: Option<String>) -> anyhow::Result<String> {
        let name = name.unwrap_or_else(|| "World".to_string());
        Ok(format!("Hello, {}!", name))
    }
}
```

## Next Steps

- Add more tools to the `impl HelloWorld` block
- See `hello-world-with-auth` for authentication
- See `hello-world-advanced` for complex features

## Dependencies

Only 6 total dependencies:

- 4 core MCP framework packages
- `tokio` for async runtime
- `anyhow` for error handling

That's it - **easy as pi!** 🥧
