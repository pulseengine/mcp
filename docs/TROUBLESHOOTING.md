# PulseEngine MCP Macros: Troubleshooting Guide

This guide helps diagnose and resolve common issues when building and deploying MCP servers with PulseEngine macros.

## Compilation Issues

### Macro Expansion Errors

**Problem**: Macro expansion fails with cryptic error messages.

```rust
error: expected identifier, found `async`
  --> src/lib.rs:12:5
   |
12 |     async fn my_tool(&self) -> String { ... }
   |     ^^^^^
```

**Common Causes**:

1. Missing `#[mcp_tool]` attribute on impl block
2. Incorrect macro syntax
3. Unsupported method signatures

**Solutions**:

```rust
// ❌ Wrong - missing #[mcp_tool] attribute
impl MyServer {
    async fn my_tool(&self) -> String {
        "result".to_string()
    }
}

// ✅ Correct - with attribute
#[mcp_tool]
impl MyServer {
    async fn my_tool(&self) -> String {
        "result".to_string()
    }
}

// ❌ Wrong - invalid parameter types
#[mcp_tool]
impl MyServer {
    async fn invalid_tool(&self, param: Box<dyn Any>) -> String {
        // Box<dyn Any> doesn't implement Deserialize
        "result".to_string()
    }
}

// ✅ Correct - serializable parameters
#[mcp_tool]
impl MyServer {
    async fn valid_tool(&self, param: String) -> String {
        format!("processed: {}", param)
    }
}
```

### Type System Errors

**Problem**: Complex types fail to serialize/deserialize.

```rust
error[E0277]: the trait bound `CustomType: serde::Deserialize<'_>` is not satisfied
```

**Solutions**:

```rust
use serde::{Deserialize, Serialize};

// ❌ Wrong - missing Serialize/Deserialize
struct CustomType {
    field: String,
}

// ✅ Correct - with derives
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomType {
    field: String,
}

// For external types, use wrapper types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WrappedExternalType {
    #[serde(flatten)]
    inner: ExternalType,
}
```

### Lifetime Issues

**Problem**: Lifetime errors in generated code.

```rust
error[E0621]: explicit lifetime required in the type of `self`
```

**Solutions**:

```rust
// ❌ Wrong - returning references to local data
#[mcp_tool]
impl MyServer {
    async fn bad_tool(&self) -> &str {
        let local_string = "temp".to_string();
        &local_string // This won't work
    }
}

// ✅ Correct - return owned data
#[mcp_tool]
impl MyServer {
    async fn good_tool(&self) -> String {
        "result".to_string()
    }
}

// ✅ Correct - return references to self
#[mcp_tool]
impl MyServer {
    async fn reference_tool(&self) -> &str {
        &self.static_data // OK if static_data is part of self
    }
}
```

## Runtime Issues

### Connection Problems

**Problem**: Client cannot connect to MCP server.

**Diagnostic Steps**:

1. **Check Transport Type**:

```rust
// Verify transport matches client expectations
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = MyServer::with_defaults();

    // For Claude Desktop - use STDIO
    let service = server.serve_stdio().await?;

    // For HTTP clients
    // let service = server.serve_http(8080).await?;

    service.run().await?;
    Ok(())
}
```

2. **Enable Debug Logging**:

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable debug logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("debug"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let server = MyServer::with_defaults();
    let service = server.serve_stdio().await?;
    service.run().await?;
    Ok(())
}
```

3. **Test with MCP Inspector**:

```bash
# Install MCP Inspector
npm install -g @modelcontextprotocol/inspector

# Test your server
mcp-inspector path/to/your/server/binary
```

### Tool Execution Errors

**Problem**: Tools fail at runtime with serialization errors.

```json
{
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": "missing field `required_param`"
  }
}
```

**Solutions**:

1. **Add Parameter Validation**:

```rust
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
struct ToolParams {
    #[validate(length(min = 1, max = 100))]
    name: String,

    #[validate(range(min = 0, max = 1000))]
    count: Option<u32>,

    #[validate(email)]
    email: Option<String>,
}

#[mcp_tool]
impl MyServer {
    /// Tool with validation
    async fn validated_tool(&self, params: ToolParams) -> Result<String, ValidationError> {
        // Validate input
        params.validate()?;

        // Process validated data
        Ok(format!("Processed: {}", params.name))
    }
}
```

2. **Improve Error Messages**:

```rust
#[derive(Debug, thiserror::Error)]
enum ToolError {
    #[error("Invalid input parameter '{field}': {reason}")]
    InvalidParameter { field: String, reason: String },

    #[error("Resource not found: {resource_id}")]
    ResourceNotFound { resource_id: String },

    #[error("Operation failed: {details}")]
    OperationFailed { details: String },
}

#[mcp_tool]
impl MyServer {
    async fn error_handling_tool(&self, id: String) -> Result<String, ToolError> {
        if id.is_empty() {
            return Err(ToolError::InvalidParameter {
                field: "id".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        // Simulate resource lookup
        if id == "missing" {
            return Err(ToolError::ResourceNotFound { resource_id: id });
        }

        Ok(format!("Found resource: {}", id))
    }
}
```

### Resource Access Issues

**Problem**: Resource URIs fail to match or parse incorrectly.

**Diagnostic Steps**:

1. **Test URI Templates**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_template_parsing() {
        // Test URI template matching
        let uri = "file:///home/user/document.txt";
        let template = "file://{path}";

        // Manual verification of template parsing
        assert!(uri.starts_with("file://"));

        let path = uri.strip_prefix("file://").unwrap();
        assert_eq!(path, "/home/user/document.txt");
    }

    #[tokio::test]
    async fn test_resource_access() {
        let server = MyServer::with_defaults();

        // Test with valid URI
        let result = server.my_resource("valid_path".to_string()).await;
        assert!(result.is_ok());

        // Test with invalid URI
        let result = server.my_resource("".to_string()).await;
        assert!(result.is_err());
    }
}
```

2. **Debug URI Template Parsing**:

```rust
#[mcp_resource(uri_template = "file://{path}")]
impl MyServer {
    /// Resource with debug logging
    async fn debug_resource(&self, path: String) -> Result<String, std::io::Error> {
        tracing::debug!("Resource accessed with path: {}", path);

        // Validate path
        if path.is_empty() {
            tracing::error!("Empty path provided");
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path cannot be empty"
            ));
        }

        // Check file existence
        if !std::path::Path::new(&path).exists() {
            tracing::warn!("File does not exist: {}", path);
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path)
            ));
        }

        tokio::fs::read_to_string(&path).await
    }
}
```

## Performance Issues

### Memory Usage Problems

**Problem**: Server consumes excessive memory or has memory leaks.

**Diagnostic Tools**:

1. **Add Memory Monitoring**:

```rust
use sysinfo::{System, SystemExt};

#[derive(Clone)]
struct MemoryMonitor {
    system: Arc<Mutex<System>>,
}

impl MemoryMonitor {
    fn new() -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }

    async fn get_memory_usage(&self) -> (u64, u64) {
        let mut system = self.system.lock().await;
        system.refresh_memory();
        (system.used_memory(), system.total_memory())
    }
}

#[mcp_tool]
impl MyServer {
    /// Memory usage diagnostic tool
    async fn memory_usage(&self) -> Result<serde_json::Value, std::io::Error> {
        let (used, total) = self.memory_monitor.get_memory_usage().await;
        let usage_percent = (used as f64 / total as f64) * 100.0;

        Ok(serde_json::json!({
            "used_bytes": used,
            "total_bytes": total,
            "usage_percent": usage_percent,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}
```

2. **Use Memory Profiling**:

```toml
# Cargo.toml
[dependencies]
jemalloc = { version = "0.5", features = ["profiling"], optional = true }

[features]
jemalloc = ["dep:jemalloc"]
```

```rust
#[cfg(feature = "jemalloc")]
use jemalloc_ctl::{stats, epoch};

#[mcp_tool]
impl MyServer {
    /// Memory profiling tool (requires jemalloc feature)
    #[cfg(feature = "jemalloc")]
    async fn memory_profile(&self) -> Result<serde_json::Value, std::io::Error> {
        epoch::advance().unwrap();

        let allocated = stats::allocated::read().unwrap();
        let resident = stats::resident::read().unwrap();
        let retained = stats::retained::read().unwrap();

        Ok(serde_json::json!({
            "allocated": allocated,
            "resident": resident,
            "retained": retained,
            "fragmentation_ratio": resident as f64 / allocated as f64
        }))
    }
}
```

### Connection Pool Issues

**Problem**: Database connection pool exhaustion or timeouts.

**Solutions**:

1. **Configure Pool Properly**:

```rust
use deadpool_postgres::{Config, Pool};

async fn create_optimized_pool() -> Result<Pool, poolError> {
    let mut config = Config::new();
    config.host = Some("localhost".to_string());
    config.user = Some("postgres".to_string());
    config.dbname = Some("mydb".to_string());

    // Pool configuration
    config.manager = Some(deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    });

    config.pool = Some(deadpool::managed::PoolConfig {
        max_size: 20,           // Adjust based on your needs
        timeouts: deadpool::managed::Timeouts {
            wait: Some(std::time::Duration::from_secs(30)),
            create: Some(std::time::Duration::from_secs(30)),
            recycle: Some(std::time::Duration::from_secs(30)),
        },
    });

    config.create_pool(Some(deadpool_postgres::Runtime::Tokio1), tokio_postgres::NoTls)
}
```

2. **Add Connection Monitoring**:

```rust
#[mcp_tool]
impl MyServer {
    /// Database pool status
    async fn pool_status(&self) -> Result<serde_json::Value, std::io::Error> {
        let status = self.db_pool.status();

        Ok(serde_json::json!({
            "size": status.size,
            "available": status.available,
            "waiting": status.waiting,
            "max_size": status.max_size
        }))
    }
}
```

3. **Implement Connection Health Checks**:

```rust
use tokio::time::{interval, Duration};

async fn connection_health_monitor(pool: Pool) {
    let mut interval = interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        match pool.get().await {
            Ok(conn) => {
                match conn.simple_query("SELECT 1").await {
                    Ok(_) => tracing::debug!("Database connection healthy"),
                    Err(e) => tracing::error!("Database health check failed: {}", e),
                }
            }
            Err(e) => tracing::error!("Failed to get database connection: {}", e),
        }
    }
}
```

## Configuration Issues

### Environment Variable Problems

**Problem**: Configuration values not loading correctly from environment.

**Solutions**:

1. **Add Configuration Validation**:

```rust
use config::{Config, ConfigError, Environment, File};

#[derive(Debug, serde::Deserialize)]
struct ServerConfig {
    database_url: String,
    redis_url: String,
    api_key: Option<String>,
    log_level: String,
}

impl ServerConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(Environment::with_prefix("MYAPP").separator("_"))
            .build()?;

        let config: Self = config.try_deserialize()?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.database_url.is_empty() {
            return Err(ConfigError::Message("DATABASE_URL is required".into()));
        }

        if !self.database_url.starts_with("postgresql://") {
            return Err(ConfigError::Message("DATABASE_URL must be a PostgreSQL URL".into()));
        }

        match self.log_level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => return Err(ConfigError::Message("Invalid log level".into())),
        }

        Ok(())
    }
}
```

2. **Add Configuration Debug Tool**:

```rust
#[mcp_tool]
impl MyServer {
    /// Show current configuration (sanitized)
    async fn show_config(&self) -> Result<serde_json::Value, std::io::Error> {
        let config = &self.config;

        Ok(serde_json::json!({
            "database_url": mask_sensitive_info(&config.database_url),
            "redis_url": mask_sensitive_info(&config.redis_url),
            "log_level": config.log_level,
            "api_key_configured": config.api_key.is_some(),
        }))
    }
}

fn mask_sensitive_info(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        let mut masked = parsed.clone();
        if masked.password().is_some() {
            let _ = masked.set_password(Some("***"));
        }
        masked.to_string()
    } else {
        "invalid_url".to_string()
    }
}
```

### Authentication Issues

**Problem**: API key authentication failing.

**Solutions**:

1. **Add Authentication Debugging**:

```rust
use pulseengine_mcp_auth::{AuthManager, AuthError};

#[derive(Clone)]
struct DebuggingAuthManager {
    inner: AuthManager,
}

impl DebuggingAuthManager {
    async fn verify_api_key(&self, key: &str) -> Result<bool, AuthError> {
        tracing::debug!("Verifying API key: {}***", &key[..4.min(key.len())]);

        let result = self.inner.verify_api_key(key).await;

        match &result {
            Ok(valid) => tracing::debug!("API key validation result: {}", valid),
            Err(e) => tracing::error!("API key validation error: {}", e),
        }

        result
    }
}

#[mcp_tool]
impl MyServer {
    /// Test API key validation
    async fn test_auth(&self, api_key: String) -> Result<serde_json::Value, AuthError> {
        let is_valid = self.auth_manager.verify_api_key(&api_key).await?;

        Ok(serde_json::json!({
            "valid": is_valid,
            "key_prefix": &api_key[..4.min(api_key.len())],
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}
```

## Deployment Issues

### Docker Container Problems

**Problem**: Server fails to start in Docker container.

**Common Issues and Solutions**:

1. **Port Binding Issues**:

```dockerfile
# ❌ Wrong - binding to localhost only
EXPOSE 8080
CMD ["./server", "--bind", "127.0.0.1:8080"]

# ✅ Correct - binding to all interfaces
EXPOSE 8080
CMD ["./server", "--bind", "0.0.0.0:8080"]
```

2. **File Permission Issues**:

```dockerfile
# Add proper user setup
RUN useradd -r -s /bin/false -u 1001 appuser
RUN mkdir -p /app/data && chown -R appuser:appuser /app
USER appuser
```

3. **Resource Limits**:

```yaml
# docker-compose.yml
services:
  mcp-server:
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: "0.5"
        reservations:
          memory: 256M
          cpus: "0.25"
```

### Kubernetes Deployment Issues

**Problem**: Pods failing health checks or crashing.

**Solutions**:

1. **Add Comprehensive Health Checks**:

```rust
#[mcp_tool]
impl MyServer {
    /// Kubernetes readiness probe
    async fn ready(&self) -> Result<serde_json::Value, std::io::Error> {
        // Check database connectivity
        if let Err(e) = self.db_pool.get().await {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Database not ready: {}", e)
            ));
        }

        // Check Redis connectivity
        if let Err(e) = self.redis_pool.get().await {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Redis not ready: {}", e)
            ));
        }

        Ok(serde_json::json!({"status": "ready"}))
    }

    /// Kubernetes liveness probe
    async fn alive(&self) -> Result<serde_json::Value, std::io::Error> {
        // Simple alive check
        Ok(serde_json::json!({
            "status": "alive",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}
```

2. **Add Resource Monitoring**:

```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
    - name: mcp-server
      resources:
        requests:
          memory: "128Mi"
          cpu: "100m"
        limits:
          memory: "512Mi"
          cpu: "500m"
      livenessProbe:
        httpGet:
          path: /alive
          port: 8080
        initialDelaySeconds: 30
        periodSeconds: 10
        timeoutSeconds: 5
        failureThreshold: 3
      readinessProbe:
        httpGet:
          path: /ready
          port: 8080
        initialDelaySeconds: 5
        periodSeconds: 5
        timeoutSeconds: 3
        failureThreshold: 3
```

## Debugging Tools

### Built-in Diagnostic Tools

Add these diagnostic tools to any server for troubleshooting:

```rust
#[mcp_tool]
impl MyServer {
    /// System information
    async fn system_info(&self) -> Result<serde_json::Value, std::io::Error> {
        use sysinfo::{System, SystemExt};

        let mut system = System::new_all();
        system.refresh_all();

        Ok(serde_json::json!({
            "hostname": system.host_name(),
            "os": system.long_os_version(),
            "kernel": system.kernel_version(),
            "cpu_count": system.processors().len(),
            "total_memory": system.total_memory(),
            "used_memory": system.used_memory(),
            "total_swap": system.total_swap(),
            "used_swap": system.used_swap(),
            "uptime": system.uptime(),
        }))
    }

    /// Process information
    async fn process_info(&self) -> Result<serde_json::Value, std::io::Error> {
        use sysinfo::{Pid, ProcessExt, System, SystemExt};

        let mut system = System::new_all();
        system.refresh_all();

        let pid = Pid::from(std::process::id() as usize);
        if let Some(process) = system.process(pid) {
            Ok(serde_json::json!({
                "pid": process.pid().as_u32(),
                "name": process.name(),
                "memory": process.memory(),
                "virtual_memory": process.virtual_memory(),
                "cpu_usage": process.cpu_usage(),
                "start_time": process.start_time(),
                "run_time": process.run_time(),
            }))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Process not found"
            ))
        }
    }

    /// Connection status
    async fn connection_status(&self) -> Result<serde_json::Value, std::io::Error> {
        let db_status = self.db_pool.status();
        let redis_status = "connected"; // Implement actual Redis status check

        Ok(serde_json::json!({
            "database": {
                "size": db_status.size,
                "available": db_status.available,
                "waiting": db_status.waiting,
            },
            "redis": {
                "status": redis_status,
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}
```

### Log Analysis

Configure structured logging for better debugging:

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| {
                "myapp=debug,pulseengine_mcp=debug,tower_http=debug".into()
            })
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json() // Use JSON for structured logging
        )
        .try_init()?;

    Ok(())
}
```

This troubleshooting guide covers the most common issues encountered when building and deploying MCP servers with PulseEngine macros. Keep this guide handy during development and deployment phases.
