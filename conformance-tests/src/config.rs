use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub description: String,
    pub binary: String,

    #[serde(default = "default_transport")]
    pub transport: String,

    pub port: Option<u16>,

    #[serde(default)]
    pub oauth: bool,

    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[serde(default)]
    pub scenarios: ScenarioConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioConfig {
    #[serde(default)]
    pub include: Vec<String>,

    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_transport() -> String {
    "stdio".to_string()
}

fn default_timeout() -> u64 {
    30000
}

impl ServerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path.display()))?;

        let config: ServerConfig =
            serde_json::from_str(&content).context("Failed to parse server config JSON")?;

        Ok(config)
    }

    pub fn get_url(&self) -> Result<String> {
        match self.transport.as_str() {
            "stdio" => {
                // For stdio, convert the binary path to stdio:// URL format
                // The conformance tool expects: stdio://executable_path with optional args
                Ok(format!("stdio://{}", self.binary))
            }
            "http" | "sse" => {
                let port = self.port.context("HTTP/SSE transport requires port")?;
                Ok(format!("http://localhost:{port}/mcp"))
            }
            "websocket" => {
                let port = self.port.context("WebSocket transport requires port")?;
                Ok(format!("ws://localhost:{port}/mcp"))
            }
            other => anyhow::bail!("Unknown transport type: {}", other),
        }
    }

    pub fn needs_network(&self) -> bool {
        !matches!(self.transport.as_str(), "stdio")
    }
}
