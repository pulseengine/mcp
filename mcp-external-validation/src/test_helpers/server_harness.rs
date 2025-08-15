//! Server test harness for managing MCP server lifecycle during testing
//!
//! This module provides utilities for starting, monitoring, and stopping
//! MCP servers for integration testing, particularly for stdio transport testing.

use crate::{ValidationConfig, ValidationError, ValidationResult};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::{Instant, sleep, timeout};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for test server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Path to the server binary
    pub binary_path: PathBuf,

    /// Command line arguments for the server
    pub args: Vec<String>,

    /// Environment variables to set
    pub env_vars: Vec<(String, String)>,

    /// Working directory for the server
    pub working_dir: Option<PathBuf>,

    /// Timeout for server startup
    pub startup_timeout: Duration,

    /// Timeout for server shutdown
    pub shutdown_timeout: Duration,

    /// Maximum time to wait for server to be ready
    pub ready_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("./target/release/timedate-mcp-server"),
            args: vec![],
            env_vars: vec![
                ("RUST_LOG".to_string(), "info".to_string()),
                ("MCP_SERVER_NAME".to_string(), "Test Server".to_string()),
            ],
            working_dir: None,
            startup_timeout: Duration::from_secs(10),
            shutdown_timeout: Duration::from_secs(5),
            ready_timeout: Duration::from_secs(15),
        }
    }
}

impl ServerConfig {
    /// Create config for timedate-mcp-server
    pub fn timedate_server() -> Self {
        Self {
            binary_path: PathBuf::from("./target/release/timedate-mcp-server"),
            ..Default::default()
        }
    }

    /// Create config for a custom server binary
    pub fn custom_server<P: AsRef<Path>>(binary_path: P) -> Self {
        Self {
            binary_path: binary_path.as_ref().to_path_buf(),
            ..Default::default()
        }
    }

    /// Add command line argument
    pub fn with_arg<S: Into<String>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple command line arguments
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Add environment variable
    pub fn with_env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    /// Set working directory
    pub fn with_working_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Set startup timeout
    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }
}

/// Test harness for managing MCP server lifecycle
pub struct ServerTestHarness {
    config: ServerConfig,
    process: Option<Child>,
    test_id: String,
    start_time: Option<Instant>,
}

impl ServerTestHarness {
    /// Create a new server test harness
    pub fn new(config: ServerConfig) -> Self {
        let test_id = format!(
            "test_{}",
            &Uuid::new_v4().to_string().replace('-', "")[..8]
        );

        Self {
            config,
            process: None,
            test_id,
            start_time: None,
        }
    }

    /// Create harness for timedate-mcp-server
    pub fn timedate_server() -> Self {
        Self::new(ServerConfig::timedate_server())
    }

    /// Get the test ID for this harness
    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    /// Check if server binary exists and is executable
    pub async fn check_binary_exists(&self) -> ValidationResult<()> {
        let path = &self.config.binary_path;

        if !path.exists() {
            return Err(ValidationError::ConfigurationError {
                message: format!(
                    "Server binary not found at: {}. Please build the server first.",
                    path.display()
                ),
            });
        }

        // Try to run --help to verify it's executable
        let output = timeout(
            Duration::from_secs(5),
            Command::new(path)
                .arg("--help")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output(),
        )
        .await;

        match output {
            Ok(Ok(result)) => {
                if result.status.success() {
                    info!("✅ Server binary verified: {}", path.display());
                    Ok(())
                } else {
                    Err(ValidationError::ConfigurationError {
                        message: format!("Server binary failed to run --help: {}", path.display()),
                    })
                }
            }
            Ok(Err(e)) => Err(ValidationError::ConfigurationError {
                message: format!("Failed to execute server binary: {}", e),
            }),
            Err(_) => Err(ValidationError::ConfigurationError {
                message: format!("Server binary --help timed out: {}", path.display()),
            }),
        }
    }

    /// Start the server process
    pub async fn start_server(&mut self) -> ValidationResult<()> {
        info!(
            "🚀 Starting server: {} (test: {})",
            self.config.binary_path.display(),
            self.test_id
        );

        // Verify binary exists first
        self.check_binary_exists().await?;

        let mut cmd = Command::new(&self.config.binary_path);

        // Add arguments
        for arg in &self.config.args {
            cmd.arg(arg);
        }

        // Add environment variables
        for (key, value) in &self.config.env_vars {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        }

        // Configure stdio - we need to capture for stdio transport testing
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Command: {:?}", cmd);

        // Start the process
        let start_time = Instant::now();
        let child = cmd.spawn().map_err(|e| ValidationError::InspectorError {
            message: format!("Failed to start server process: {}", e),
        })?;

        self.process = Some(child);
        self.start_time = Some(start_time);

        info!(
            "✅ Server process started (PID: {:?})",
            self.get_process_id()
        );

        // Wait for server to be ready
        self.wait_for_ready().await?;

        Ok(())
    }

    /// Wait for server to be ready to accept connections
    pub async fn wait_for_ready(&mut self) -> ValidationResult<()> {
        info!("⏳ Waiting for server to be ready...");

        let start = Instant::now();
        let timeout_duration = self.config.ready_timeout;

        while start.elapsed() < timeout_duration {
            // Check if process is still running
            if !self.is_running() {
                return Err(ValidationError::InspectorError {
                    message: "Server process exited during startup".to_string(),
                });
            }

            // For stdio servers, we consider them ready immediately after startup
            // since they don't bind to ports. In a real test, we'd send an MCP
            // initialize message to verify readiness.
            sleep(Duration::from_millis(100)).await;

            // Simple readiness check - if process has been running for 1 second, consider it ready
            if start.elapsed() >= Duration::from_secs(1) {
                info!("✅ Server ready after {:?}", start.elapsed());
                return Ok(());
            }
        }

        Err(ValidationError::Timeout {
            seconds: timeout_duration.as_secs(),
        })
    }

    /// Check if the server process is running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            // Check if process has exited
            match process.try_wait() {
                Ok(Some(_)) => false, // Process has exited
                Ok(None) => true,     // Process is still running
                Err(_) => false,      // Error checking - assume not running
            }
        } else {
            false
        }
    }

    /// Get the process ID if running
    pub fn get_process_id(&self) -> Option<u32> {
        self.process.as_ref().and_then(|p| p.id())
    }

    /// Get the command string for use with MCP Inspector
    pub fn get_server_command(&self) -> String {
        let mut cmd_parts = vec![self.config.binary_path.to_string_lossy().to_string()];
        cmd_parts.extend(self.config.args.clone());
        cmd_parts.join(" ")
    }

    /// Get server uptime
    pub fn uptime(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Stop the server process gracefully
    pub async fn stop_server(&mut self) -> ValidationResult<()> {
        if let Some(mut process) = self.process.take() {
            info!("🛑 Stopping server (test: {})", self.test_id);

            // Try graceful termination first
            #[cfg(unix)]
            {
                if let Some(pid) = process.id() {
                    // Send SIGTERM
                    debug!("Sending SIGTERM to process {}", pid);
                    let _ = Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .output()
                        .await;
                }
            }

            // Wait for graceful shutdown
            let shutdown_result = timeout(self.config.shutdown_timeout, process.wait()).await;

            match shutdown_result {
                Ok(Ok(status)) => {
                    info!("✅ Server stopped gracefully with status: {}", status);
                    Ok(())
                }
                Ok(Err(e)) => {
                    warn!("Error waiting for server shutdown: {}", e);
                    Err(ValidationError::InspectorError {
                        message: format!("Server shutdown error: {}", e),
                    })
                }
                Err(_) => {
                    warn!("Server shutdown timed out, killing process");
                    let _ = process.kill().await;
                    Err(ValidationError::Timeout {
                        seconds: self.config.shutdown_timeout.as_secs(),
                    })
                }
            }
        } else {
            debug!("No server process to stop");
            Ok(())
        }
    }

    /// Force kill the server process
    pub async fn kill_server(&mut self) -> ValidationResult<()> {
        if let Some(mut process) = self.process.take() {
            warn!("🔪 Force killing server process (test: {})", self.test_id);

            match process.kill().await {
                Ok(_) => {
                    info!("✅ Server process killed");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to kill server process: {}", e);
                    Err(ValidationError::InspectorError {
                        message: format!("Failed to kill server: {}", e),
                    })
                }
            }
        } else {
            debug!("No server process to kill");
            Ok(())
        }
    }
}

impl Drop for ServerTestHarness {
    fn drop(&mut self) {
        if self.process.is_some() {
            warn!("ServerTestHarness dropped with running process - force killing");
            // We can't await in Drop, so we spawn a task
            if let Some(mut process) = self.process.take() {
                tokio::spawn(async move {
                    let _ = process.kill().await;
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_server_config_creation() {
        let config = ServerConfig::timedate_server();
        assert!(
            config
                .binary_path
                .to_string_lossy()
                .contains("timedate-mcp-server")
        );
        assert_eq!(config.args.len(), 0);
        assert!(config.env_vars.len() > 0);
    }

    #[test]
    fn test_server_config_builder() {
        let config = ServerConfig::custom_server("/path/to/server")
            .with_arg("--verbose")
            .with_args(vec!["--port", "3000"])
            .with_env("DEBUG", "1")
            .with_startup_timeout(Duration::from_secs(20));

        assert_eq!(config.binary_path, PathBuf::from("/path/to/server"));
        assert_eq!(config.args, vec!["--verbose", "--port", "3000"]);
        assert!(
            config
                .env_vars
                .contains(&("DEBUG".to_string(), "1".to_string()))
        );
        assert_eq!(config.startup_timeout, Duration::from_secs(20));
    }

    #[test]
    fn test_server_harness_creation() {
        let config = ServerConfig::timedate_server();
        let mut harness = ServerTestHarness::new(config);

        assert!(!harness.test_id().is_empty());
        assert!(harness.test_id().starts_with("test_"));
        assert!(!harness.is_running());
        assert!(harness.get_process_id().is_none());
        assert!(harness.uptime().is_none());
    }

    #[test]
    fn test_get_server_command() {
        let config = ServerConfig::custom_server("./my-server")
            .with_args(vec!["--arg1", "value1", "--arg2"]);
        let harness = ServerTestHarness::new(config);

        let cmd = harness.get_server_command();
        assert_eq!(cmd, "./my-server --arg1 value1 --arg2");
    }

    // Note: Actual server start/stop tests would require a real binary
    // and are better suited for integration tests
}
