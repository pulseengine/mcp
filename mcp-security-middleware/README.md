# PulseEngine MCP Security Middleware

Zero-configuration security middleware for MCP servers with Axum integration.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

## Overview

This crate provides a simple, secure-by-default authentication and authorization middleware system for MCP servers. It transforms security from a complex, multi-crate barrier into a competitive advantage with minimal configuration.

## Key Features

- **🚀 Zero Configuration**: Works out of the box with sensible secure defaults
- **📊 Security Profiles**: Development, staging, and production profiles with appropriate security levels
- **🔧 Environment-Based Config**: Configure via environment variables without CLI tools
- **🎲 Auto-Generation**: Automatically generates API keys and JWT secrets securely
- **⚡ Axum Integration**: Built on `middleware::from_fn` for seamless integration
- **✅ MCP Compliance**: Follows 2025 MCP security best practices

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
pulseengine-mcp-security-middleware = "0.10.0"
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1.0", features = ["full"] }
```

### Development Setup (3 lines of code)

```rust
use pulseengine_mcp_security_middleware::*;
use axum::{Router, routing::get, middleware::from_fn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Zero-config development setup
    let security_config = SecurityConfig::development();
    let middleware = security_config.create_middleware().await?;
    
    let app = Router::new()
        .route("/", get(|| async { "Hello, secure world!" }))
        .layer(from_fn(move |req, next| {
            let middleware = middleware.clone();
            async move { middleware.process(req, next).await }
        }));
        
    // Server setup...
    Ok(())
}
```

### Production Setup (5 lines of code)

```rust
use pulseengine_mcp_security_middleware::*;

#[tokio::main]  
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Production-ready security
    let security_config = SecurityConfig::production()
        .with_api_key(std::env::var("MCP_API_KEY")?)
        .with_jwt_secret(std::env::var("MCP_JWT_SECRET")?);
        
    let middleware = security_config.create_middleware().await?;
    
    // Use with your MCP server...
    Ok(())
}
```

## Security Profiles

### Development Profile
- **Authentication**: Optional (logged when present)
- **HTTPS**: Optional (localhost connections accepted)
- **Rate Limiting**: Disabled for development convenience
- **CORS**: Permissive (wildcard origins allowed)
- **Key Generation**: Automatic with secure random
- **Perfect for**: Local development, testing, prototyping

```rust
let config = SecurityConfig::development();
// API key auto-generated and logged for testing
```

### Staging Profile
- **Authentication**: Required with JWT validation
- **HTTPS**: Enforced for all connections
- **Rate Limiting**: Moderate (1000 requests/minute)
- **CORS**: Localhost origins only
- **Key Generation**: Automatic with secure random
- **Perfect for**: Testing environments, CI/CD pipelines

```rust
let config = SecurityConfig::staging();
// Balanced security for testing environments
```

### Production Profile
- **Authentication**: Strict JWT with audience validation
- **HTTPS**: Mandatory with security headers
- **Rate Limiting**: Conservative (100 requests/minute)
- **CORS**: Explicit origins only
- **Key Management**: Manual configuration required
- **Perfect for**: Production deployments, enterprise environments

```rust
let config = SecurityConfig::production()
    .with_api_key(env::var("MCP_API_KEY")?)
    .with_jwt_secret(env::var("MCP_JWT_SECRET")?);
```

## Environment Configuration

Zero CLI tools required - configure everything via environment variables:

```bash
# Security profile (development, staging, production)
MCP_SECURITY_PROFILE=production

# Auto-generated if not provided (development/staging only)
MCP_API_KEY=your-api-key-here
MCP_JWT_SECRET=your-jwt-secret-here

# CORS configuration
MCP_CORS_ORIGIN=https://yourdomain.com
# or for multiple origins:
MCP_CORS_ORIGIN=https://app1.com,https://app2.com

# Rate limiting
MCP_RATE_LIMIT=100/min

# Security features
MCP_REQUIRE_HTTPS=true
MCP_ENABLE_AUDIT_LOG=true
```

## Features Overview

| Feature | Development | Staging | Production |
|---------|-------------|---------|------------|
| Authentication | Optional | Required | Strict |
| Auto-Generate Keys | ✅ | ✅ | ❌ |
| HTTPS Required | ❌ | ✅ | ✅ |
| Rate Limiting | Disabled | 1000/min | 100/min |
| CORS | Permissive | Localhost | Explicit |
| Audit Logging | ✅ | ✅ | ✅ |
| JWT Expiry | 24 hours | 1 hour | 15 minutes |

## Authentication Methods

### API Key Authentication

```bash
# In Authorization header
curl -H "Authorization: ApiKey mcp_your_key_here" https://api.example.com/

# In X-API-Key header
curl -H "X-API-Key: mcp_your_key_here" https://api.example.com/
```

### JWT Bearer Token

```bash
curl -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." https://api.example.com/
```

## Security Features

### Rate Limiting
- Per-client IP address tracking
- Configurable time windows and request limits
- Automatic cleanup of old entries
- Burst allowance for legitimate usage spikes

### Request Validation
- API key format validation
- JWT signature and audience verification
- HTTPS enforcement for production
- Request size limits

### Audit Logging
- All authentication attempts logged
- Request/response correlation IDs
- Security events tracking
- Structured logging format

### Security Headers
Automatically adds security headers to all responses:
- Content Security Policy
- X-Frame-Options: DENY
- X-Content-Type-Options: nosniff
- Strict-Transport-Security (HTTPS only)
- Referrer-Policy

## Migration from Multi-Crate System

If you're currently using the complex multi-crate system:

### Before (5+ crates, 318+ lines of config)
```rust
// Complex setup with multiple CLI tools and crates
use pulseengine_mcp_auth::*;
use pulseengine_mcp_security::*;
use pulseengine_mcp_monitoring::*;
// ... extensive configuration ...
```

### After (1 crate, 3 lines of code)
```rust
use pulseengine_mcp_security_middleware::*;

let config = SecurityConfig::development();
let middleware = config.create_middleware().await?;
```

### Migration Steps

1. **Replace Dependencies**: Remove old security crates, add middleware crate
2. **Update Code**: Replace complex configuration with security profiles
3. **Set Environment**: Move settings to environment variables
4. **Test**: Verify authentication works with new middleware

## Examples

### Hello World with Authentication
See `examples/hello-world-with-auth/` for a complete working example showing:
- Zero-config development setup
- Auto-generated API keys
- Request logging and audit trails
- Progressive security complexity

### Integration with MCP Server
```rust
use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_security_middleware::*;

#[mcp_server(name = "Secure MCP Server")]
#[derive(Default, Clone)]
pub struct SecureServer;

#[mcp_tools]
impl SecureServer {
    pub async fn secure_operation(&self) -> anyhow::Result<String> {
        // Your business logic here
        // Security is handled transparently by middleware
        Ok("Operation completed securely".to_string())
    }
}
```

## Error Handling

The middleware provides clear error responses:

- `401 Unauthorized`: Missing or invalid authentication
- `403 Forbidden`: HTTPS required but not provided
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Configuration or validation errors

## Performance

- **Authentication Overhead**: <1ms per request
- **Memory Usage**: Minimal (rate limiter cleanup automatic)
- **Throughput**: No significant impact on request throughput
- **Concurrency**: Thread-safe with Arc<Mutex> for rate limiting

## Security Best Practices

1. **Use Production Profile**: For production deployments
2. **Environment Variables**: Never commit secrets to version control
3. **HTTPS Only**: Enforce HTTPS for all production traffic
4. **Regular Key Rotation**: Rotate JWT secrets periodically
5. **Monitor Audit Logs**: Watch for unusual authentication patterns
6. **Rate Limiting**: Tune rate limits based on usage patterns

## Troubleshooting

### Authentication Failures
- Check API key format (must start with `mcp_`)
- Verify JWT secret is at least 32 characters
- Ensure token audience matches configuration

### Rate Limiting Issues
- Check client IP detection (proxy headers)
- Adjust rate limits for your usage patterns
- Monitor rate limiter memory usage

### CORS Problems
- Verify allowed origins configuration
- Check that credentials flag matches wildcard usage
- Test preflight OPTIONS requests

## Contributing

Contributions welcome! This middleware was designed based on real-world production needs and feedback.

Priority areas:
- Additional authentication methods (OAuth 2.1, SAML)
- More sophisticated rate limiting algorithms
- Integration examples with different MCP server frameworks
- Performance optimizations

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.

---

**Transform security from complexity to competitive advantage with zero-configuration MCP security middleware.**