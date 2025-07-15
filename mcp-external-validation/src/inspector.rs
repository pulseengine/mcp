//! MCP Inspector integration for automated testing
//!
//! This module provides integration with the official MCP Inspector tool
//! (@modelcontextprotocol/inspector) for automated testing and validation of MCP servers.

use crate::{
    report::{InspectorResult, IssueSeverity, ValidationIssue},
    ValidationConfig, ValidationError, ValidationResult,
};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// MCP Inspector client for automated testing  
pub struct InspectorClient {
    config: ValidationConfig,
    inspector_executable: PathBuf,
    session_token: Option<String>,
}

/// Real MCP Inspector output format
#[derive(Debug, Deserialize)]
struct RealInspectorOutput {
    /// Inspector session information
    session: Option<InspectorSessionInfo>,

    /// Server validation results
    server: Option<ServerValidationResults>,

    /// Tools discovered and tested
    tools: Option<Vec<ToolValidationResult>>,

    /// Resources discovered and tested
    resources: Option<Vec<ResourceValidationResult>>,

    /// Prompts discovered and tested
    prompts: Option<Vec<PromptValidationResult>>,

    /// Transport test results
    transport: Option<TransportValidationResult>,

    /// Any errors encountered
    errors: Option<Vec<InspectorError>>,
}

/// Inspector session information from real inspector
#[derive(Debug, Deserialize)]
struct InspectorSessionInfo {
    /// Unique session identifier
    id: String,

    /// Inspector version
    inspector_version: String,

    /// Authentication token for this session
    token: Option<String>,

    /// Server URL being tested
    server_url: String,

    /// Transport method used
    transport_method: String,
}

/// Server-level validation results
#[derive(Debug, Deserialize)]
struct ServerValidationResults {
    /// Server connection successful
    connected: bool,

    /// Server initialization successful  
    initialized: bool,

    /// Server capabilities properly declared
    capabilities_valid: bool,

    /// Server responds to ping
    ping_successful: bool,

    /// Authentication status
    authentication: AuthenticationResult,
}

/// Authentication test result
#[derive(Debug, Deserialize)]
struct AuthenticationResult {
    /// Whether authentication was attempted
    attempted: bool,

    /// Whether authentication succeeded
    successful: bool,

    /// Authentication method used
    method: Option<String>,

    /// Error if authentication failed
    error: Option<String>,
}

/// Tool validation result
#[derive(Debug, Deserialize)]
struct ToolValidationResult {
    /// Tool name
    name: String,

    /// Tool description
    description: Option<String>,

    /// Whether tool was callable
    callable: bool,

    /// Test call result
    test_result: Option<String>,

    /// Any errors during testing
    errors: Option<Vec<String>>,
}

/// Resource validation result
#[derive(Debug, Deserialize)]
struct ResourceValidationResult {
    /// Resource URI
    uri: String,

    /// Resource name
    name: Option<String>,

    /// Whether resource was accessible
    accessible: bool,

    /// Resource content type
    mime_type: Option<String>,

    /// Any errors during testing
    errors: Option<Vec<String>>,
}

/// Prompt validation result
#[derive(Debug, Deserialize)]
struct PromptValidationResult {
    /// Prompt name
    name: String,

    /// Prompt description
    description: Option<String>,

    /// Whether prompt was usable
    usable: bool,

    /// Arguments the prompt accepts
    arguments: Option<Vec<String>>,

    /// Any errors during testing
    errors: Option<Vec<String>>,
}

/// Transport validation result
#[derive(Debug, Deserialize)]
struct TransportValidationResult {
    /// Transport method (stdio, sse, websocket)
    method: String,

    /// Whether transport worked
    working: bool,

    /// Connection latency if measured
    latency_ms: Option<u64>,

    /// Any transport-specific errors
    errors: Option<Vec<String>>,
}

/// Inspector error
#[derive(Debug, Deserialize)]
struct InspectorError {
    /// Error code or type
    code: Option<String>,

    /// Human-readable error message
    message: String,

    /// Additional error context
    context: Option<serde_json::Value>,
}

impl InspectorClient {
    /// Create a new inspector client
    pub fn new(config: ValidationConfig) -> ValidationResult<Self> {
        // Check if npx is available
        let npx_check = Command::new("npx")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if npx_check.is_err() || !npx_check.unwrap().success() {
            return Err(ValidationError::ConfigurationError {
                message: "npx is not available. Please install Node.js and npm.".to_string(),
            });
        }

        Ok(Self {
            config,
            inspector_executable: PathBuf::from("npx"),
            session_token: None,
        })
    }

    /// Check if MCP Inspector package is available
    pub async fn check_inspector_availability(&self) -> ValidationResult<bool> {
        info!("Checking MCP Inspector availability...");

        let output = TokioCommand::new("npx")
            .arg("@modelcontextprotocol/inspector")
            .arg("--help")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match output {
            Ok(result) => {
                if result.status.success() {
                    info!("MCP Inspector is available");
                    Ok(true)
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    warn!("MCP Inspector check failed: {}", stderr);
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("Failed to check MCP Inspector: {}", e);
                Ok(false)
            }
        }
    }

    /// Get inspector version information
    pub async fn get_inspector_version(&self) -> ValidationResult<String> {
        info!("Getting MCP Inspector version...");

        let output = TokioCommand::new("npx")
            .arg("@modelcontextprotocol/inspector")
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ValidationError::InspectorError {
                message: format!("Failed to get inspector version: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ValidationError::InspectorError {
                message: format!("Inspector version check failed: {}", stderr),
            });
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }

    /// Test an MCP server using the real inspector
    pub async fn test_server(&self, server_command: &str) -> ValidationResult<InspectorResult> {
        info!("Testing MCP server with real Inspector: {}", server_command);

        // Check if inspector is available first
        if !self.check_inspector_availability().await? {
            return Err(ValidationError::InspectorError {
                message: "MCP Inspector is not available. Please ensure Node.js is installed."
                    .to_string(),
            });
        }

        // Parse server command into parts
        let server_parts: Vec<&str> = server_command.split_whitespace().collect();
        if server_parts.is_empty() {
            return Err(ValidationError::ConfigurationError {
                message: "Server command cannot be empty".to_string(),
            });
        }

        // Run the real MCP Inspector
        let mut cmd = TokioCommand::new("npx");
        cmd.arg("@modelcontextprotocol/inspector");

        // Add server command parts
        for part in server_parts {
            cmd.arg(part);
        }

        // Set environment variables for automation
        cmd.env("CLIENT_PORT", self.config.inspector.port.to_string());
        cmd.env("HEADLESS", "true");
        cmd.env("JSON_OUTPUT", "true");

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        info!(
            "Running: npx @modelcontextprotocol/inspector {}",
            server_command
        );

        let output = timeout(self.config.inspector_timeout_duration(), cmd.output())
            .await
            .map_err(|_| ValidationError::Timeout {
                seconds: self.config.inspector.timeout,
            })?
            .map_err(|e| ValidationError::InspectorError {
                message: format!("Failed to run inspector: {}", e),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!("Inspector stdout: {}", stdout);
        debug!("Inspector stderr: {}", stderr);

        if !output.status.success() {
            return Err(ValidationError::InspectorError {
                message: format!(
                    "Inspector failed with exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    stderr
                ),
            });
        }

        // Parse the inspector output
        self.parse_inspector_output(&stdout, &stderr)
    }

    /// Test multiple MCP server commands
    pub async fn test_multiple_servers(
        &self,
        server_commands: &[&str],
    ) -> ValidationResult<Vec<InspectorResult>> {
        let mut results = Vec::new();

        for server_command in server_commands {
            info!("Testing server: {}", server_command);

            match self.test_server(server_command).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Server test failed for '{}': {}", server_command, e);
                    // Continue with other servers
                }
            }
        }

        if results.is_empty() {
            return Err(ValidationError::InspectorError {
                message: "All server tests failed".to_string(),
            });
        }

        Ok(results)
    }

    /// Test a Python MCP server (using uvx)
    pub async fn test_python_server(
        &self,
        package_name: &str,
        args: &[&str],
    ) -> ValidationResult<InspectorResult> {
        let mut server_command = format!("uvx {}", package_name);
        for arg in args {
            server_command.push_str(&format!(" {}", arg));
        }

        info!("Testing Python MCP server: {}", server_command);
        self.test_server(&server_command).await
    }

    /// Test a Node.js MCP server
    pub async fn test_node_server(
        &self,
        command: &str,
        args: &[&str],
    ) -> ValidationResult<InspectorResult> {
        let mut server_command = command.to_string();
        for arg in args {
            server_command.push_str(&format!(" {}", arg));
        }

        info!("Testing Node.js MCP server: {}", server_command);
        self.test_server(&server_command).await
    }

    /// Parse the real inspector output
    fn parse_inspector_output(
        &self,
        stdout: &str,
        stderr: &str,
    ) -> ValidationResult<InspectorResult> {
        debug!("Parsing inspector output...");

        // Look for session token in output (needed for authentication)
        if let Some(token) = self.extract_session_token(stderr) {
            debug!("Found session token: {}", token);
        }

        // Try to parse JSON output if available
        if let Ok(inspector_output) = serde_json::from_str::<RealInspectorOutput>(stdout) {
            return self.convert_real_inspector_output(inspector_output);
        }

        // Fall back to parsing text output
        self.parse_text_inspector_output(stdout, stderr)
    }

    /// Extract session token from inspector stderr
    fn extract_session_token(&self, stderr: &str) -> Option<String> {
        // Look for lines like "🔑 Session token: 3a1c267fad21f7150b7d624c..."
        for line in stderr.lines() {
            if line.contains("Session token:") {
                if let Some(token_part) = line.split("Session token:").nth(1) {
                    return Some(token_part.trim().to_string());
                }
            }
        }
        None
    }

    /// Convert real inspector output to our result format
    fn convert_real_inspector_output(
        &self,
        output: RealInspectorOutput,
    ) -> ValidationResult<InspectorResult> {
        let mut inspector_issues = Vec::new();

        // Extract connection and server info
        let connection_success = output.server.as_ref().map(|s| s.connected).unwrap_or(false);

        let auth_success = output
            .server
            .as_ref()
            .map(|s| s.authentication.successful)
            .unwrap_or(false);

        // Count tools and resources
        let tools_discoverable = output
            .tools
            .as_ref()
            .map(|tools| !tools.is_empty())
            .unwrap_or(false);

        let resources_accessible = output
            .resources
            .as_ref()
            .map(|resources| resources.iter().any(|r| r.accessible))
            .unwrap_or(false);

        // Check transport
        let export_success = output
            .transport
            .as_ref()
            .map(|t| t.working)
            .unwrap_or(false);

        // Collect errors as issues
        if let Some(errors) = output.errors {
            for error in errors {
                inspector_issues.push(format!(
                    "{}: {}",
                    error.code.unwrap_or_else(|| "Error".to_string()),
                    error.message
                ));
            }
        }

        Ok(InspectorResult {
            connection_success,
            auth_success,
            tools_discoverable,
            resources_accessible,
            export_success,
            inspector_issues,
        })
    }

    /// Parse text-based inspector output (fallback)
    fn parse_text_inspector_output(
        &self,
        stdout: &str,
        stderr: &str,
    ) -> ValidationResult<InspectorResult> {
        let mut connection_success = false;
        let mut auth_success = false;
        let mut tools_discoverable = false;
        let mut resources_accessible = false;
        let mut export_success = false;
        let mut inspector_issues = Vec::new();

        // Parse common patterns from inspector output
        let combined_output = format!("{}\n{}", stdout, stderr);

        for line in combined_output.lines() {
            let line_lower = line.to_lowercase();

            if line_lower.contains("connected") && line_lower.contains("successfully") {
                connection_success = true;
            }

            if line_lower.contains("authentication") && line_lower.contains("successful") {
                auth_success = true;
            }

            if line_lower.contains("tools")
                && (line_lower.contains("found") || line_lower.contains("discovered"))
            {
                tools_discoverable = true;
            }

            if line_lower.contains("resources") && line_lower.contains("accessible") {
                resources_accessible = true;
            }

            if line_lower.contains("export") && line_lower.contains("success") {
                export_success = true;
            }

            if line_lower.contains("error") || line_lower.contains("failed") {
                inspector_issues.push(line.to_string());
            }
        }

        Ok(InspectorResult {
            connection_success,
            auth_success,
            tools_discoverable,
            resources_accessible,
            export_success,
            inspector_issues,
        })
    }
}

// InspectorClient no longer needs Drop since we use subprocess calls directly

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_client_creation() {
        let config = ValidationConfig::default();
        let client = InspectorClient::new(config);

        // On systems where npx is available, client should succeed
        // On systems where npx is not available, client should fail with configuration error
        match client {
            Ok(_) => {
                // npx is available, test passes
            }
            Err(ValidationError::ConfigurationError { message }) => {
                // npx is not available, which is expected in some CI environments
                assert!(message.contains("npx is not available"));
            }
            Err(e) => {
                // Unexpected error type
                panic!("Unexpected error type: {:?}", e);
            }
        }
    }

    #[test]
    fn test_server_command_parsing() {
        let server_command = "uvx mcp-server-git --repository /path/to/repo";
        let parts: Vec<&str> = server_command.split_whitespace().collect();

        assert_eq!(parts[0], "uvx");
        assert_eq!(parts[1], "mcp-server-git");
        assert_eq!(parts[2], "--repository");
        assert_eq!(parts[3], "/path/to/repo");
    }

    #[tokio::test]
    async fn test_inspector_availability() {
        // Skip this test if no inspector is configured for testing
        if std::env::var("MCP_INSPECTOR_TEST").is_err() {
            return;
        }

        let config = ValidationConfig::default();
        let client = InspectorClient::new(config).unwrap();

        // Check if inspector is available
        let is_available = client.check_inspector_availability().await.unwrap_or(false);
        // We don't assert here since inspector may not be available in CI
        println!("Inspector available: {}", is_available);
    }

    #[test]
    fn test_real_inspector_output_conversion() {
        let config = ValidationConfig::default();

        // Skip test if npx is not available (e.g., in CI environments without Node.js)
        let client = match InspectorClient::new(config) {
            Ok(client) => client,
            Err(ValidationError::ConfigurationError { .. }) => {
                // npx not available, skip test
                return;
            }
            Err(e) => panic!("Unexpected error creating client: {:?}", e),
        };

        let output = RealInspectorOutput {
            session: Some(InspectorSessionInfo {
                id: "test-session".to_string(),
                inspector_version: "1.0.0".to_string(),
                token: Some("test-token".to_string()),
                server_url: "test://server".to_string(),
                transport_method: "stdio".to_string(),
            }),
            server: Some(ServerValidationResults {
                connected: true,
                initialized: true,
                capabilities_valid: true,
                ping_successful: true,
                authentication: AuthenticationResult {
                    attempted: true,
                    successful: false,
                    method: Some("bearer".to_string()),
                    error: Some("Invalid token".to_string()),
                },
            }),
            tools: Some(vec![ToolValidationResult {
                name: "test-tool".to_string(),
                description: Some("A test tool".to_string()),
                callable: true,
                test_result: Some("success".to_string()),
                errors: None,
            }]),
            resources: Some(vec![ResourceValidationResult {
                uri: "file://test.txt".to_string(),
                name: Some("test.txt".to_string()),
                accessible: true,
                mime_type: Some("text/plain".to_string()),
                errors: None,
            }]),
            prompts: None,
            transport: Some(TransportValidationResult {
                method: "stdio".to_string(),
                working: true,
                latency_ms: Some(50),
                errors: None,
            }),
            errors: Some(vec![InspectorError {
                code: Some("AUTH_FAILED".to_string()),
                message: "Authentication failed".to_string(),
                context: None,
            }]),
        };

        let result = client.convert_real_inspector_output(output).unwrap();
        assert!(result.connection_success);
        assert!(!result.auth_success); // Auth failed
        assert!(result.tools_discoverable);
        assert!(result.resources_accessible);
        assert!(result.export_success); // Transport working
        assert_eq!(result.inspector_issues.len(), 1);
        assert!(result.inspector_issues[0].contains("AUTH_FAILED"));
    }
}
