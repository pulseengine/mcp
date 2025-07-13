# Memory-Only Authentication Example

This example demonstrates how to run a PulseEngine MCP server with memory-only authentication, completely eliminating filesystem dependencies.

## Features

- **Zero Filesystem Dependencies**: All authentication data is stored in memory
- **Runtime Key Management**: Add/remove API keys while the server is running
- **Temporary by Design**: All keys are lost when the server restarts
- **Full Authentication**: Supports all authentication features (roles, permissions, rate limiting)

## Usage

```bash
# Run the server
cargo run --example memory-only-auth

# Or build and run
cargo build --example memory-only-auth
./target/debug/examples/memory-only-auth
```

## Default API Keys

The server starts with these pre-configured keys:

- **Admin Key**: `admin-secret-key-12345` (ID: `admin_key_1`)
- **Operator Key**: `operator-secret-key-67890` (ID: `operator_key_1`)
- **Monitor Key**: `monitor-secret-key-abcdef` (ID: `monitor_key_1`)

## Available Tools

- `list_auth_keys`: List all API keys currently in memory
- `add_temp_key`: Add a temporary API key to memory (lost on restart)

## Configuration

To customize the initial API keys, modify the `MemoryAuthConfig::default()` implementation:

```rust
impl Default for MemoryAuthConfig {
    fn default() -> Self {
        Self {
            initial_api_keys: vec![
                ("my_admin".to_string(), "my-admin-key".to_string(), Role::Admin),
                ("my_operator".to_string(), "my-operator-key".to_string(), Role::Operator),
            ],
        }
    }
}
```

## Use Cases

- **Development**: No filesystem setup required
- **Testing**: Clean state on each restart
- **Containerized Deployments**: No volume mounts needed
- **Temporary Services**: Short-lived servers that don't need persistent auth