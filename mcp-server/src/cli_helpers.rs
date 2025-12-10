//! CLI helpers and configuration utilities
//!
//! This module provides utilities for CLI-based MCP servers, including:
//! - Server info creation from Cargo.toml metadata
//! - Logging configuration
//! - Environment variable utilities

use pulseengine_mcp_protocol::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

/// CLI-related errors
#[derive(Debug, Error)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("CLI parsing error: {0}")]
    Parsing(String),

    #[error("Server setup error: {0}")]
    ServerSetup(String),

    #[error("Logging setup error: {0}")]
    Logging(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(#[from] pulseengine_mcp_protocol::Error),
}

impl CliError {
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    pub fn parsing(msg: impl Into<String>) -> Self {
        Self::Parsing(msg.into())
    }

    pub fn server_setup(msg: impl Into<String>) -> Self {
        Self::ServerSetup(msg.into())
    }

    pub fn logging(msg: impl Into<String>) -> Self {
        Self::Logging(msg.into())
    }
}

/// Default logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultLoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub output: LogOutput,
    pub structured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "pretty")]
    Pretty,
    #[serde(rename = "compact")]
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
    #[serde(rename = "file")]
    File(String),
}

impl Default for DefaultLoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            output: LogOutput::Stdout,
            structured: true,
        }
    }
}

impl DefaultLoggingConfig {
    pub fn initialize(&self) -> Result<(), CliError> {
        use tracing_subscriber::{EnvFilter, fmt, prelude::*};

        let level = env::var("RUST_LOG").unwrap_or_else(|_| self.level.clone());
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(&level))
            .map_err(|e| CliError::logging(format!("Invalid log level: {e}")))?;

        match self.format {
            LogFormat::Json => {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::layer().json())
                    .init();
            }
            LogFormat::Pretty => {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::layer().pretty())
                    .init();
            }
            LogFormat::Compact => {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::layer().compact())
                    .init();
            }
        }

        Ok(())
    }
}

/// Create default server info from Cargo.toml metadata
///
/// # Arguments
/// * `name` - Optional server name (defaults to CARGO_PKG_NAME)
/// * `version` - Optional version (defaults to CARGO_PKG_VERSION)
///
/// # Example
/// ```rust,ignore
/// use pulseengine_mcp_server::cli_helpers::create_server_info;
///
/// let info = create_server_info(Some("My Server".to_string()), None);
/// ```
pub fn create_server_info(name: Option<String>, version: Option<String>) -> ServerInfo {
    ServerInfo {
        protocol_version: ProtocolVersion::default(),
        capabilities: ServerCapabilities::default(),
        server_info: Implementation::new(
            name.unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string()),
            version.unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        ),
        instructions: None,
    }
}

/// Environment variable utilities
pub mod env_utils {
    use std::env;
    use std::str::FromStr;

    /// Get environment variable with default value
    pub fn get_env_or_default<T>(key: &str, default: T) -> T
    where
        T: FromStr + Clone,
    {
        env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    /// Get required environment variable
    pub fn get_required_env<T>(key: &str) -> Result<T, super::CliError>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
    {
        env::var(key)
            .map_err(|_| {
                super::CliError::configuration(format!(
                    "Missing required environment variable: {key}"
                ))
            })?
            .parse()
            .map_err(|e| super::CliError::configuration(format!("Invalid value for {key}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_error_constructors() {
        let config_err = CliError::configuration("config issue");
        assert!(matches!(config_err, CliError::Configuration(_)));
        assert!(config_err.to_string().contains("config issue"));

        let parse_err = CliError::parsing("parse issue");
        assert!(matches!(parse_err, CliError::Parsing(_)));
        assert!(parse_err.to_string().contains("parse issue"));

        let setup_err = CliError::server_setup("setup issue");
        assert!(matches!(setup_err, CliError::ServerSetup(_)));
        assert!(setup_err.to_string().contains("setup issue"));

        let log_err = CliError::logging("log issue");
        assert!(matches!(log_err, CliError::Logging(_)));
        assert!(log_err.to_string().contains("log issue"));
    }

    #[test]
    fn test_default_logging_config() {
        let config = DefaultLoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.structured);
        assert!(matches!(config.format, LogFormat::Pretty));
        assert!(matches!(config.output, LogOutput::Stdout));
    }

    #[test]
    fn test_log_format_serialization() {
        let json_format = serde_json::to_string(&LogFormat::Json).unwrap();
        assert!(json_format.contains("json"));

        let pretty_format = serde_json::to_string(&LogFormat::Pretty).unwrap();
        assert!(pretty_format.contains("pretty"));

        let compact_format = serde_json::to_string(&LogFormat::Compact).unwrap();
        assert!(compact_format.contains("compact"));
    }

    #[test]
    fn test_log_output_serialization() {
        let stdout = serde_json::to_string(&LogOutput::Stdout).unwrap();
        assert!(stdout.contains("stdout"));

        let stderr = serde_json::to_string(&LogOutput::Stderr).unwrap();
        assert!(stderr.contains("stderr"));

        let file = serde_json::to_string(&LogOutput::File("/tmp/log.txt".to_string())).unwrap();
        assert!(file.contains("/tmp/log.txt"));
    }

    #[test]
    fn test_create_server_info_with_custom_values() {
        let info = create_server_info(Some("TestServer".to_string()), Some("1.0.0".to_string()));
        assert_eq!(info.server_info.name, "TestServer");
        assert_eq!(info.server_info.version, "1.0.0");
    }

    #[test]
    fn test_create_server_info_with_defaults() {
        let info = create_server_info(None, None);
        // Should use CARGO_PKG_NAME and CARGO_PKG_VERSION
        assert!(!info.server_info.name.is_empty());
        assert!(!info.server_info.version.is_empty());
    }

    #[test]
    fn test_env_utils_get_env_or_default() {
        // Test with non-existent env var
        let result: i32 = env_utils::get_env_or_default("NON_EXISTENT_VAR_12345", 42);
        assert_eq!(result, 42);

        // Test with string
        let result: String =
            env_utils::get_env_or_default("NON_EXISTENT_VAR_12345", "default".to_string());
        assert_eq!(result, "default");
    }

    #[test]
    fn test_env_utils_get_required_env_missing() {
        let result: Result<String, _> = env_utils::get_required_env("NON_EXISTENT_VAR_12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing required"));
    }
}
