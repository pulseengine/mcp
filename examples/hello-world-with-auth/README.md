# Hello World with Authentication

This example demonstrates how to add zero-configuration authentication to an MCP server using the security middleware.

## What This Example Shows

- **Progressive Complexity**: Builds on the basic hello-world example
- **Development Security Profile**: Permissive settings perfect for local development
- **Auto-Generated Credentials**: No manual key management required
- **Simple Integration**: Single line of code to add security

## Key Features

### 🔐 Development Security Profile

- Authentication is **optional** but **logged** when present
- Auto-generates API keys for testing
- Permissive CORS settings
- Detailed logging for debugging

### 🎯 Zero Configuration

- No environment variables required
- No CLI tools to run
- No configuration files needed
- Works out of the box

### 📊 Security Logging

- All authentication attempts logged
- Request/response audit trail
- Security events tracking
- Performance metrics

## Running the Example

```bash
# From the workspace root
cargo run --bin hello-world-with-auth
```

The server will display:

- Generated API key for testing
- Security configuration summary
- Available endpoints
- Testing instructions

## Testing Authentication

The example runs in development mode, so authentication is optional:

```bash
# Without authentication (works in development mode)
curl http://localhost:8080/mcp/tools/list

# With API key (use the key shown in logs)
curl -H "Authorization: ApiKey mcp_generated_key_here" \\
     -H "Content-Type: application/json" \\
     -d '{"method": "tools/call", "params": {"name": "say_hello", "arguments": {"name": "Developer"}}}' \\
     http://localhost:8080/mcp/tools/call

# Check authentication status
curl -H "Authorization: ApiKey mcp_generated_key_here" \\
     http://localhost:8080/mcp/tools/call/auth_status
```

## Code Walkthrough

### 1. Security Configuration (3 lines)

```rust
let security_config = SecurityConfig::development();
let security_middleware = security_config.create_middleware().await?;
```

### 2. MCP Server Integration (1 line)

```rust
// Integrate with MCP server HTTP transport
.layer(from_fn(security_middleware))
```

### 3. Tool Implementation (unchanged)

```rust
#[mcp_tools]
impl HelloWorldAuth {
    pub async fn say_hello(&self, name: Option<String>) -> anyhow::Result<String> {
        // Your business logic here - security is handled transparently
    }
}
```

## Security Features Demonstrated

| Feature            | Development Mode | Notes                          |
| ------------------ | ---------------- | ------------------------------ |
| Authentication     | Optional         | Logged when present            |
| Rate Limiting      | Disabled         | For development convenience    |
| HTTPS              | Optional         | Localhost connections accepted |
| CORS               | Permissive       | Wildcard origins allowed       |
| Audit Logging      | Enabled          | All requests logged            |
| API Key Generation | Automatic        | New key per restart            |

## Environment Variables (Optional)

While zero configuration, you can customize behavior:

```bash
# Override default settings
MCP_API_KEY=my-custom-dev-key
MCP_SECURITY_PROFILE=development
MCP_ENABLE_AUDIT_LOG=true

# Run with custom settings
cargo run --bin hello-world-with-auth
```

## Next Steps

1. **hello-world-production**: See how to deploy with production security
2. **multi-tool-server**: Learn about tool-level permissions
3. **enterprise-server**: Advanced security configurations

## Comparison with Basic Hello World

| Aspect            | hello-world | hello-world-with-auth |
| ----------------- | ----------- | --------------------- |
| Lines of code     | 25          | 35 (+authentication)  |
| Setup complexity  | None        | None (zero-config)    |
| Security features | None        | Full middleware stack |
| Production ready  | No          | Development ready     |
| Authentication    | None        | API key + JWT support |

This demonstrates how the security middleware maintains simplicity while adding comprehensive security features.
