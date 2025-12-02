//! # MCP Resources Demo
//!
//! This example demonstrates how to use the #[mcp_resource] attribute inside
//! #[mcp_tools] impl blocks to create dynamic, parameterized resources.
//!
//! ## Key Concepts
//!
//! 1. **Tools vs Resources**:
//!    - Methods WITHOUT #[mcp_resource] → become tools
//!    - Methods WITH #[mcp_resource] → become resources
//!
//! 2. **URI Templates**:
//!    - Resources use URI templates with parameters: "scheme://{param1}/{param2}"
//!    - Parameters are automatically extracted and passed to methods
//!    - Uses matchit library for efficient URI routing
//!
//! 3. **Automatic Integration**:
//!    - #[mcp_tools] scans all methods
//!    - Implements McpToolsProvider AND McpResourcesProvider
//!    - #[mcp_server] generates complete McpBackend implementation
//!    - No manual registration needed!

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A server demonstrating resources with URI templates
#[mcp_server(name = "Resources Demo", auth = "disabled")]
#[derive(Default, Clone)]
struct ResourcesDemo {
    // In-memory data store for demonstration
    data: HashMap<String, String>,
}

impl ResourcesDemo {
    fn new() -> Self {
        let mut data = HashMap::new();

        // Pre-populate with some demo data
        data.insert(
            "1".to_string(),
            r#"{"id": "1", "name": "Alice", "role": "admin"}"#.to_string(),
        );
        data.insert(
            "2".to_string(),
            r#"{"id": "2", "name": "Bob", "role": "user"}"#.to_string(),
        );
        data.insert(
            "app".to_string(),
            r#"{"theme": "dark", "language": "en"}"#.to_string(),
        );

        Self { data }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    theme: String,
    language: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataInfo {
    key: String,
    exists: bool,
    value_preview: Option<String>,
}

// 🎯 KEY PATTERN: Use #[mcp_tools] for BOTH tools AND resources
#[mcp_tools]
impl ResourcesDemo {
    // ==================== TOOLS ====================
    // Methods WITHOUT #[mcp_resource] attribute become tools

    /// List all available data keys
    pub fn list_keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// Get information about how many items are stored
    pub fn info(&self) -> String {
        format!("Storing {} items", self.data.len())
    }

    // ==================== RESOURCES ====================
    // Methods WITH #[mcp_resource] attribute become resources with URI routing

    /// Get user data by ID
    #[mcp_resource(uri_template = "user://{user_id}")]
    pub fn get_user(&self, user_id: String) -> Result<User, String> {
        let data = self
            .data
            .get(&user_id)
            .ok_or_else(|| format!("User not found: {user_id}"))?;

        serde_json::from_str(data).map_err(|e| format!("Failed to parse user data: {e}"))
    }

    /// Get configuration settings
    #[mcp_resource(uri_template = "config://{config_name}")]
    pub fn get_config(&self, config_name: String) -> Result<Config, String> {
        let data = self
            .data
            .get(&config_name)
            .ok_or_else(|| format!("Config not found: {config_name}"))?;

        serde_json::from_str(data).map_err(|e| format!("Failed to parse config: {e}"))
    }

    /// Get any data by key
    #[mcp_resource(uri_template = "data://{key}")]
    pub fn get_data(&self, key: String) -> Result<DataInfo, String> {
        let exists = self.data.contains_key(&key);
        let value_preview = self.data.get(&key).map(|v| {
            if v.len() > 50 {
                format!("{}...", &v[..50])
            } else {
                v.clone()
            }
        });

        Ok(DataInfo {
            key,
            exists,
            value_preview,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ResourcesDemo::new();

    // Use HTTP transport for conformance testing
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let mut mcp_server = server.serve_http(port).await?;
    mcp_server.run().await?;

    Ok(())
}
