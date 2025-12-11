# PulseEngine MCP Macros: Complete Guide

A comprehensive guide to building Model Context Protocol servers using PulseEngine's powerful macro system.

## Overview

PulseEngine MCP Macros dramatically simplify building MCP servers by automatically generating protocol-compliant code from simple Rust function annotations. This guide follows the patterns from the [official MCP tutorial](https://modelcontextprotocol.io/tutorials/building-mcp-with-llms) while leveraging the power of Rust macros.

## Quick Start

### 1. Preparing Your Project

Add PulseEngine MCP Macros to your `Cargo.toml`:

```toml
[dependencies]
pulseengine-mcp-macros = "0.15"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### 2. Building Your First Server

Create a simple MCP server with just a few lines:

```rust
use pulseengine_mcp_macros::mcp_server;

#[mcp_server(name = "My First Server")]
#[derive(Default, Clone)]
struct MyServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = MyServer::with_defaults();
    server.serve_stdio().await?.run().await?;
    Ok(())
}
```

That's it! You now have a working MCP server that Claude can connect to.

## Core Concepts

### Server Declaration

The `#[mcp_server]` macro transforms a simple struct into a fully-featured MCP server:

```rust
#[mcp_server(
    name = "Advanced Server",
    version = "1.0.0",
    description = "A sophisticated MCP server",
    app_name = "my-app"  // For isolated storage
)]
#[derive(Default, Clone)]
struct AdvancedServer {
    // Your server state here
}
```

**Key Parameters:**

- `name` - Display name for your server (required)
- `version` - Server version (defaults to Cargo.toml version)
- `description` - Server description (defaults to doc comments)
- `app_name` - Application name for storage isolation (optional)

### Adding Tools

Tools are the core functionality of your MCP server. Use `#[mcp_tool]` to expose functions:

```rust
use pulseengine_mcp_macros::mcp_tool;

#[mcp_tool]
impl AdvancedServer {
    /// Calculate the sum of two numbers
    async fn add(&self, a: f64, b: f64) -> f64 {
        a + b
    }

    /// Process text with various operations
    async fn process_text(&self, text: String, operation: String) -> Result<String, std::io::Error> {
        match operation.as_str() {
            "uppercase" => Ok(text.to_uppercase()),
            "reverse" => Ok(text.chars().rev().collect()),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Unknown operation"
            ))
        }
    }
}
```

**Tool Features:**

- **Automatic Schema Generation** - Parameter types become JSON schemas
- **Error Handling** - Rust errors are converted to MCP protocol errors
- **Documentation** - Doc comments become tool descriptions
- **Type Safety** - Compile-time validation of parameters

### Adding Resources

Resources provide access to external data. Use `#[mcp_resource]` with URI templates:

```rust
use pulseengine_mcp_macros::mcp_resource;

#[mcp_resource(uri_template = "file://{path}")]
impl AdvancedServer {
    /// Read a file from the filesystem
    async fn read_file(&self, path: String) -> Result<String, std::io::Error> {
        tokio::fs::read_to_string(&path).await
    }
}

#[mcp_resource(
    uri_template = "api://{endpoint}/{id}",
    mime_type = "application/json"
)]
impl AdvancedServer {
    /// Fetch data from an API endpoint
    async fn api_data(&self, endpoint: String, id: String) -> Result<serde_json::Value, std::io::Error> {
        // Your API call logic here
        Ok(serde_json::json!({
            "endpoint": endpoint,
            "id": id,
            "data": "example"
        }))
    }
}
```

**Resource Features:**

- **URI Templates** - Flexible parameter extraction from URIs
- **MIME Type Support** - Specify content types for proper handling
- **Path Parameters** - Automatic extraction and validation
- **Content Negotiation** - Support for various content types

### Adding Prompts

Prompts help Claude generate better responses. Use `#[mcp_prompt]`:

````rust
use pulseengine_mcp_macros::mcp_prompt;
use pulseengine_mcp_protocol::{PromptMessage, Role, PromptContent};

#[mcp_prompt(name = "code_review")]
impl AdvancedServer {
    /// Generate a code review prompt
    async fn code_review_prompt(&self, code: String, language: String) -> Result<PromptMessage, std::io::Error> {
        Ok(PromptMessage {
            role: Role::User,
            content: PromptContent::Text {
                text: format!(
                    "Please review this {} code and provide feedback:\n\n```{}\n{}\n```",
                    language, language, code
                ),
            },
        })
    }
}
````

## Advanced Patterns

### Application-Specific Configuration

Use `app_name` to isolate storage and configuration:

```rust
#[mcp_server(
    name = "MyApp Server",
    app_name = "myapp"  // Creates isolated ~/.pulseengine/myapp/ directory
)]
#[derive(Default, Clone)]
struct MyAppServer;
```

This prevents conflicts when running multiple MCP servers on the same system.

### Complex Data Types

The macro system supports sophisticated data structures:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[mcp_tool]
impl AdvancedServer {
    /// Create a new user
    async fn create_user(&self, user_data: User) -> Result<User, std::io::Error> {
        // Validation and processing
        Ok(user_data)
    }

    /// Search users with complex parameters
    async fn search_users(&self,
        query: Option<String>,
        limit: Option<u32>,
        filters: std::collections::HashMap<String, String>
    ) -> Vec<User> {
        // Your search logic here
        vec![]
    }
}
```

### Error Handling Best Practices

Design robust error handling for production use:

```rust
#[derive(Debug, thiserror::Error)]
enum MyServerError {
    #[error("User not found: {id}")]
    UserNotFound { id: u64 },
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

#[mcp_tool]
impl AdvancedServer {
    /// Example with proper error handling
    async fn get_user(&self, id: u64) -> Result<User, MyServerError> {
        // Your logic here
        Err(MyServerError::UserNotFound { id })
    }
}
```

### Performance and Concurrency

Design for high-performance concurrent access:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[mcp_server(name = "High Performance Server")]
#[derive(Clone)]
struct PerformanceServer {
    data: Arc<RwLock<std::collections::HashMap<String, String>>>,
}

impl Default for PerformanceServer {
    fn default() -> Self {
        Self {
            data: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[mcp_tool]
impl PerformanceServer {
    /// Concurrent-safe data access
    async fn get_data(&self, key: String) -> Option<String> {
        let data = self.data.read().await;
        data.get(&key).cloned()
    }

    /// Batch processing for efficiency
    async fn process_batch(&self, items: Vec<String>) -> Vec<String> {
        // Process items concurrently
        let tasks: Vec<_> = items.into_iter()
            .map(|item| async move { format!("processed: {}", item) })
            .collect();

        futures::future::join_all(tasks).await
    }
}
```

## Working with Claude

### Optimal Tool Design

When designing tools for Claude, follow these principles:

- **Clear Naming** - Use descriptive function names that explain the purpose
- **Rich Documentation** - Write comprehensive doc comments
- **Logical Parameters** - Group related parameters together
- **Consistent Returns** - Use consistent return types across similar tools

```rust
#[mcp_tool]
impl AdvancedServer {
    /// Analyze text sentiment and extract key insights
    ///
    /// This tool processes natural language text to determine emotional tone
    /// and extract meaningful insights for content analysis.
    ///
    /// # Parameters
    /// - `text`: The text content to analyze
    /// - `detailed`: Whether to include detailed breakdown
    ///
    /// # Returns
    /// A structured analysis with sentiment scores and key insights
    async fn analyze_sentiment(&self,
        text: String,
        detailed: Option<bool>
    ) -> Result<SentimentAnalysis, std::io::Error> {
        // Your analysis logic
        todo!()
    }
}
```

### Resource Organization

Structure resources to match Claude's mental model:

```rust
// Hierarchical data access
#[mcp_resource(uri_template = "docs://{category}/{document}")]
impl AdvancedServer {
    async fn documentation(&self, category: String, document: String) -> Result<String, std::io::Error> {
        // Return documentation content
    }
}

// Dynamic content generation
#[mcp_resource(uri_template = "reports://{type}/{date_range}")]
impl AdvancedServer {
    async fn generate_report(&self, report_type: String, date_range: String) -> Result<String, std::io::Error> {
        // Generate and return report
    }
}
```

### Prompt Engineering

Create prompts that help Claude understand your domain:

```rust
#[mcp_prompt(name = "database_query")]
impl AdvancedServer {
    /// Generate optimized database queries
    async fn database_query_prompt(&self,
        table_schema: String,
        requirements: String
    ) -> Result<PromptMessage, std::io::Error> {
        let prompt_text = format!(
            "Given this database schema:\n\n{}\n\nGenerate an optimized SQL query that: {}\n\nConsider:\n- Performance implications\n- Index usage\n- Security (prevent SQL injection)\n- Readability and maintainability",
            table_schema,
            requirements
        );

        Ok(PromptMessage {
            role: Role::User,
            content: PromptContent::Text { text: prompt_text },
        })
    }
}
```

## Best Practices

### Security Considerations

Always validate and sanitize inputs:

```rust
#[mcp_tool]
impl AdvancedServer {
    /// Secure file access with validation
    async fn read_secure_file(&self, path: String) -> Result<String, std::io::Error> {
        // Prevent directory traversal
        if path.contains("..") || path.starts_with("/") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Invalid path"
            ));
        }

        // Restrict to safe directory
        let safe_path = format!("./data/{}", path);
        tokio::fs::read_to_string(safe_path).await
    }
}
```

### Testing Your Server

Write comprehensive tests for your MCP server:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_functionality() {
        let server = AdvancedServer::with_defaults();

        // Test tools
        let result = server.add(2.0, 3.0).await;
        assert_eq!(result, 5.0);

        // Test error handling
        let error_result = server.process_text("test".to_string(), "invalid".to_string()).await;
        assert!(error_result.is_err());
    }
}
```

### Deployment Patterns

Structure your main function for robust deployment:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create server instance
    let server = AdvancedServer::with_defaults();

    // Choose transport based on environment
    let service = match std::env::var("MCP_TRANSPORT") {
        Ok(transport) if transport == "http" => {
            let port = std::env::var("MCP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080);
            server.serve_http(port).await?
        }
        _ => server.serve_stdio().await?
    };

    // Handle graceful shutdown
    let shutdown = async {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        tracing::info!("Shutdown signal received");
    };

    service.run_with_shutdown(shutdown).await?;
    Ok(())
}
```

## Next Steps

Now that you understand the PulseEngine MCP macro system:

1. **Explore Examples** - Check out the [examples directory](../examples/) for real-world implementations
2. **Read the Protocol** - Understand the [MCP specification](https://modelcontextprotocol.io/specification/)
3. **Join the Community** - Connect with other MCP developers
4. **Contribute** - Help improve the macro system with feedback and contributions

### Additional Resources

- [Macro API Reference](./API_REFERENCE.md) - Complete macro documentation
- [Advanced Patterns](./ADVANCED_PATTERNS.md) - Complex implementation patterns
- [Deployment Guide](./DEPLOYMENT.md) - Production deployment strategies
- [Troubleshooting](./TROUBLESHOOTING.md) - Common issues and solutions

---

Happy building! The PulseEngine MCP macro system makes it easier than ever to create powerful, protocol-compliant MCP servers that work seamlessly with Claude and other AI assistants.
