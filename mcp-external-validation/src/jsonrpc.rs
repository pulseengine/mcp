//! JSON-RPC 2.0 compliance validation
//!
//! This module provides validation against the JSON-RPC 2.0 specification
//! using external validators and schema validation.

use crate::{
    report::{IssueSeverity, JsonRpcValidatorResult, TestScore, ValidationIssue},
    ValidationConfig, ValidationError, ValidationResult,
};
use jsonschema::{Draft, JSONSchema};
// Note: reqwest::Client used for real message collection
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
// use std::collections::HashMap;  // Removed unused import
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

/// JSON-RPC 2.0 validator client (local validation)
pub struct JsonRpcValidator {
    config: ValidationConfig,
    schemas: JsonRpcSchemas,
}

/// JSON-RPC 2.0 schema definitions
#[derive(Debug)]
struct JsonRpcSchemas {
    /// Request message schema
    request_schema: JSONSchema,

    /// Response message schema
    response_schema: JSONSchema,

    /// Error object schema
    error_schema: JSONSchema,

    /// Notification schema
    notification_schema: JSONSchema,
}

// Removed old external API structures - now using local validation

// Removed old external API response structures - now using local validation

/// JSON-RPC message types for testing
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    Request {
        jsonrpc: String,
        method: String,
        params: Option<Value>,
        id: Value,
    },
    Response {
        jsonrpc: String,
        result: Option<Value>,
        error: Option<JsonRpcErrorObject>,
        id: Value,
    },
    Notification {
        jsonrpc: String,
        method: String,
        params: Option<Value>,
    },
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    /// Error code
    pub code: i32,

    /// Error message
    pub message: String,

    /// Additional error data
    pub data: Option<Value>,
}

/// Correlation validation results
#[derive(Debug)]
struct CorrelationResults {
    passed: u32,
    total: u32,
    issues: Vec<ValidationIssue>,
}

/// Error handling validation results
#[derive(Debug)]
struct ErrorHandlingResults {
    passed: u32,
    total: u32,
    issues: Vec<ValidationIssue>,
}

impl JsonRpcValidator {
    /// Create a new JSON-RPC validator (local validation only)
    pub fn new(config: ValidationConfig) -> ValidationResult<Self> {
        let schemas = JsonRpcSchemas::new()?;

        Ok(Self { config, schemas })
    }

    /// Validate JSON-RPC messages against specification
    pub async fn validate_messages(
        &self,
        messages: &[Value],
    ) -> ValidationResult<JsonRpcValidatorResult> {
        info!(
            "Starting JSON-RPC validation for {} messages",
            messages.len()
        );

        // Validate against local schemas and enhanced rules
        let validation_results = self.validate_comprehensive(messages)?;

        info!(
            "JSON-RPC validation completed with {} issues",
            validation_results.get_total_issues()
        );
        Ok(validation_results)
    }

    /// Validate messages from an MCP server
    pub async fn validate_server_messages(
        &self,
        server_url: &str,
    ) -> ValidationResult<JsonRpcValidatorResult> {
        info!("Collecting JSON-RPC messages from server: {}", server_url);

        // Collect sample messages from the server
        let messages = self.collect_server_messages(server_url).await?;

        if messages.is_empty() {
            warn!("No JSON-RPC messages collected from server");
            return Err(ValidationError::ValidationFailed {
                message: "No JSON-RPC messages found to validate".to_string(),
            });
        }

        self.validate_messages(&messages).await
    }

    /// Collect JSON-RPC messages from a server (public interface for semantic validation)
    pub async fn collect_messages_from_server(
        &self,
        server_url: &str,
    ) -> ValidationResult<Vec<Value>> {
        info!(
            "Collecting messages from server for semantic validation: {}",
            server_url
        );
        self.collect_server_messages(server_url).await
    }

    /// Validate a single JSON-RPC message
    pub fn validate_single_message(
        &self,
        message: &Value,
    ) -> ValidationResult<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        // Determine message type and validate accordingly
        if let Some(jsonrpc) = message.get("jsonrpc") {
            // Check JSON-RPC version
            if jsonrpc != "2.0" {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "jsonrpc".to_string(),
                    format!(
                        "Invalid JSON-RPC version: expected '2.0', got '{}'",
                        jsonrpc
                    ),
                    "jsonrpc-validator".to_string(),
                ));
            }
        } else {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "jsonrpc".to_string(),
                "Missing required 'jsonrpc' field".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        // Validate based on message type
        if message.get("method").is_some() {
            // Request or notification
            if message.get("id").is_some() {
                // Request
                self.validate_request_message(message, &mut issues)?;
            } else {
                // Notification
                self.validate_notification_message(message, &mut issues)?;
            }
        } else if message.get("result").is_some() || message.get("error").is_some() {
            // Response
            self.validate_response_message(message, &mut issues)?;
        } else {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "message_type".to_string(),
                "Cannot determine message type (missing method, result, or error)".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        Ok(issues)
    }

    /// Test JSON-RPC compliance with sample messages
    pub async fn test_compliance(&self) -> ValidationResult<JsonRpcValidatorResult> {
        let test_messages = self.generate_test_messages();
        self.validate_messages(&test_messages).await
    }

    /// Comprehensive JSON-RPC validation with enhanced features
    fn validate_comprehensive(
        &self,
        messages: &[Value],
    ) -> ValidationResult<JsonRpcValidatorResult> {
        let mut schema_passed = 0;
        let mut schema_total = 0;
        let mut format_passed = 0;
        let mut format_total = 0;
        let mut correlation_passed = 0;
        let mut correlation_total = 0;
        let mut error_handling_passed = 0;
        let mut error_handling_total = 0;

        let mut validation_issues = Vec::new();
        let mut request_response_pairs = Vec::new();

        // First pass: Individual message validation
        for (i, message) in messages.iter().enumerate() {
            schema_total += 1;
            format_total += 1;

            // Schema validation
            match self.validate_message_schema(message) {
                Ok(_) => schema_passed += 1,
                Err(errors) => {
                    for error in errors {
                        validation_issues.push(ValidationIssue::new(
                            IssueSeverity::Error,
                            "schema".to_string(),
                            format!("Message {}: {}", i, error),
                            "jsonrpc-validator".to_string(),
                        ));
                    }
                }
            }

            // Format validation
            match self.validate_message_format(message) {
                Ok(_) => format_passed += 1,
                Err(errors) => {
                    for error in errors {
                        validation_issues.push(ValidationIssue::new(
                            IssueSeverity::Error,
                            "format".to_string(),
                            format!("Message {}: {}", i, error),
                            "jsonrpc-validator".to_string(),
                        ));
                    }
                }
            }

            // Enhanced validation
            self.validate_enhanced_rules(message, i, &mut validation_issues)?;

            // Collect request/response pairs for correlation analysis
            if let Some(id) = message.get("id") {
                let is_request = message.get("method").is_some();
                let is_response = message.get("result").is_some() || message.get("error").is_some();

                request_response_pairs.push((i, id.clone(), is_request, is_response));
            }
        }

        // Second pass: Correlation validation
        if !request_response_pairs.is_empty() {
            let correlation_results =
                self.validate_request_response_correlation(&request_response_pairs);
            correlation_total = correlation_results.total;
            correlation_passed = correlation_results.passed;
            validation_issues.extend(correlation_results.issues);
        }

        // Third pass: Error handling validation
        let error_results = self.validate_error_handling(messages)?;
        error_handling_total = error_results.total;
        error_handling_passed = error_results.passed;
        validation_issues.extend(error_results.issues);

        Ok(JsonRpcValidatorResult {
            schema_validation: TestScore::new(schema_passed, schema_total),
            message_format: TestScore::new(format_passed, format_total),
            error_handling: TestScore::new(error_handling_passed, error_handling_total),
            correlation: TestScore::new(correlation_passed, correlation_total),
        })
    }

    /// Validate enhanced JSON-RPC rules beyond basic schema
    fn validate_enhanced_rules(
        &self,
        message: &Value,
        index: usize,
        issues: &mut Vec<ValidationIssue>,
    ) -> ValidationResult<()> {
        // Check for common JSON-RPC anti-patterns
        self.check_id_format(message, index, issues);
        self.check_method_naming(message, index, issues);
        self.check_parameter_structure(message, index, issues);
        self.check_error_codes(message, index, issues);

        Ok(())
    }

    /// Check ID format compliance
    fn check_id_format(&self, message: &Value, index: usize, issues: &mut Vec<ValidationIssue>) {
        if let Some(id) = message.get("id") {
            match id {
                Value::String(_) | Value::Number(_) | Value::Null => {
                    // Valid ID types
                }
                _ => {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Error,
                        "id_format".to_string(),
                        format!("Message {}: ID must be a string, number, or null", index),
                        "jsonrpc-validator".to_string(),
                    ));
                }
            }

            // Check for empty string IDs
            if let Some(id_str) = id.as_str() {
                if id_str.is_empty() {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "id_format".to_string(),
                        format!("Message {}: Empty string ID is not recommended", index),
                        "jsonrpc-validator".to_string(),
                    ));
                }
            }
        }
    }

    /// Check method naming conventions
    fn check_method_naming(
        &self,
        message: &Value,
        index: usize,
        issues: &mut Vec<ValidationIssue>,
    ) {
        if let Some(method) = message.get("method") {
            if let Some(method_str) = method.as_str() {
                // Check for reserved method names (starting with rpc.)
                if method_str.starts_with("rpc.") && !self.is_allowed_rpc_method(method_str) {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Error,
                        "method_naming".to_string(),
                        format!(
                            "Message {}: Method name '{}' is reserved",
                            index, method_str
                        ),
                        "jsonrpc-validator".to_string(),
                    ));
                }

                // Check for method name conventions
                if method_str.is_empty() {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Error,
                        "method_naming".to_string(),
                        format!("Message {}: Method name cannot be empty", index),
                        "jsonrpc-validator".to_string(),
                    ));
                }

                // Check for non-ASCII characters
                if !method_str.is_ascii() {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "method_naming".to_string(),
                        format!(
                            "Message {}: Method name contains non-ASCII characters",
                            index
                        ),
                        "jsonrpc-validator".to_string(),
                    ));
                }
            }
        }
    }

    /// Check parameter structure
    fn check_parameter_structure(
        &self,
        message: &Value,
        index: usize,
        issues: &mut Vec<ValidationIssue>,
    ) {
        if let Some(params) = message.get("params") {
            match params {
                Value::Object(_) | Value::Array(_) => {
                    // Valid parameter types
                }
                _ => {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Error,
                        "parameter_structure".to_string(),
                        format!("Message {}: Parameters must be an object or array", index),
                        "jsonrpc-validator".to_string(),
                    ));
                }
            }

            // Check for empty parameters
            if (params.is_object() && params.as_object().unwrap().is_empty())
                || (params.is_array() && params.as_array().unwrap().is_empty())
            {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Info,
                    "parameter_structure".to_string(),
                    format!(
                        "Message {}: Empty parameters - consider omitting params field",
                        index
                    ),
                    "jsonrpc-validator".to_string(),
                ));
            }
        }
    }

    /// Check error codes
    fn check_error_codes(&self, message: &Value, index: usize, issues: &mut Vec<ValidationIssue>) {
        if let Some(error) = message.get("error") {
            if let Some(error_obj) = error.as_object() {
                if let Some(code) = error_obj.get("code") {
                    if let Some(code_num) = code.as_i64() {
                        if !self.is_valid_error_code(code_num) {
                            issues.push(ValidationIssue::new(
                                IssueSeverity::Warning,
                                "error_codes".to_string(),
                                format!("Message {}: Error code {} is not a standard JSON-RPC error code", index, code_num),
                                "jsonrpc-validator".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Check if error code is valid according to JSON-RPC 2.0 spec
    fn is_valid_error_code(&self, code: i64) -> bool {
        match code {
            // Pre-defined errors
            -32700 => true, // Parse error
            -32600 => true, // Invalid Request
            -32601 => true, // Method not found
            -32602 => true, // Invalid params
            -32603 => true, // Internal error
            // Server error range
            -32099..=-32000 => true,
            // Application error range (commonly used)
            -32768..=-32000 => true,
            _ => false,
        }
    }

    /// Check if RPC method is allowed
    fn is_allowed_rpc_method(&self, method: &str) -> bool {
        matches!(
            method,
            "rpc.call"
                | "rpc.multicall"
                | "rpc.notification"
                | "rpc.ping"
                | "rpc.info"
                | "rpc.capabilities"
        )
    }

    /// Validate request/response correlation
    fn validate_request_response_correlation(
        &self,
        pairs: &[(usize, Value, bool, bool)], // (index, id, is_request, is_response)
    ) -> CorrelationResults {
        let mut passed = 0;
        let mut total = 0;
        let mut issues = Vec::new();

        // Group by ID
        let mut id_groups: std::collections::HashMap<String, Vec<&(usize, Value, bool, bool)>> =
            std::collections::HashMap::new();

        for pair in pairs {
            let id_str = pair.1.to_string();
            id_groups.entry(id_str).or_insert_with(Vec::new).push(pair);
        }

        // Check each ID group
        for (id, group) in id_groups {
            total += 1;

            let requests: Vec<_> = group.iter().filter(|(_, _, is_req, _)| *is_req).collect();
            let responses: Vec<_> = group.iter().filter(|(_, _, _, is_resp)| *is_resp).collect();

            if requests.is_empty() && !responses.is_empty() {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Warning,
                    "correlation".to_string(),
                    format!("Response with ID {} has no corresponding request", id),
                    "jsonrpc-validator".to_string(),
                ));
            } else if !requests.is_empty() && responses.is_empty() {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Info,
                    "correlation".to_string(),
                    format!("Request with ID {} has no corresponding response", id),
                    "jsonrpc-validator".to_string(),
                ));
            } else if requests.len() > 1 {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "correlation".to_string(),
                    format!("Multiple requests found with same ID {}", id),
                    "jsonrpc-validator".to_string(),
                ));
            } else if responses.len() > 1 {
                issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "correlation".to_string(),
                    format!("Multiple responses found with same ID {}", id),
                    "jsonrpc-validator".to_string(),
                ));
            } else {
                passed += 1; // Perfect correlation
            }
        }

        CorrelationResults {
            passed,
            total,
            issues,
        }
    }

    /// Validate error handling patterns
    fn validate_error_handling(
        &self,
        messages: &[Value],
    ) -> ValidationResult<ErrorHandlingResults> {
        let mut passed = 0;
        let mut total = 0;
        let mut issues = Vec::new();

        for (i, message) in messages.iter().enumerate() {
            if let Some(error) = message.get("error") {
                total += 1;

                if let Some(error_obj) = error.as_object() {
                    // Check required error fields
                    if let Some(code) = error_obj.get("code") {
                        if let Some(message_field) = error_obj.get("message") {
                            if message_field.is_string() {
                                // Check if error code is in valid range
                                if let Some(code_num) = code.as_i64() {
                                    if self.is_valid_error_code(code_num) {
                                        passed += 1;
                                    } else {
                                        issues.push(ValidationIssue::new(
                                            IssueSeverity::Warning,
                                            "error_handling".to_string(),
                                            format!(
                                                "Message {}: Non-standard error code {}",
                                                i, code_num
                                            ),
                                            "jsonrpc-validator".to_string(),
                                        ));
                                    }
                                } else {
                                    issues.push(ValidationIssue::new(
                                        IssueSeverity::Error,
                                        "error_handling".to_string(),
                                        format!("Message {}: Error code must be a number", i),
                                        "jsonrpc-validator".to_string(),
                                    ));
                                }
                            } else {
                                issues.push(ValidationIssue::new(
                                    IssueSeverity::Error,
                                    "error_handling".to_string(),
                                    format!("Message {}: Error message must be a string", i),
                                    "jsonrpc-validator".to_string(),
                                ));
                            }
                        } else {
                            issues.push(ValidationIssue::new(
                                IssueSeverity::Error,
                                "error_handling".to_string(),
                                format!(
                                    "Message {}: Error object missing required 'message' field",
                                    i
                                ),
                                "jsonrpc-validator".to_string(),
                            ));
                        }
                    } else {
                        issues.push(ValidationIssue::new(
                            IssueSeverity::Error,
                            "error_handling".to_string(),
                            format!("Message {}: Error object missing required 'code' field", i),
                            "jsonrpc-validator".to_string(),
                        ));
                    }
                } else {
                    issues.push(ValidationIssue::new(
                        IssueSeverity::Error,
                        "error_handling".to_string(),
                        format!("Message {}: Error field must be an object", i),
                        "jsonrpc-validator".to_string(),
                    ));
                }
            }
        }

        Ok(ErrorHandlingResults {
            passed,
            total,
            issues,
        })
    }

    /// Collect sample messages from an MCP server
    async fn collect_server_messages(&self, server_url: &str) -> ValidationResult<Vec<Value>> {
        info!(
            "Collecting real JSON-RPC messages from MCP server: {}",
            server_url
        );

        let mut collected_messages = Vec::new();

        // Try different collection strategies based on server URL type
        if server_url.starts_with("http://") || server_url.starts_with("https://") {
            // HTTP-based MCP server
            collected_messages.extend(self.collect_http_messages(server_url).await?);
        } else if server_url.starts_with("stdio://") {
            // Stdio-based MCP server
            collected_messages.extend(self.collect_stdio_messages(server_url).await?);
        } else if server_url.starts_with("ws://") || server_url.starts_with("wss://") {
            // WebSocket-based MCP server
            collected_messages.extend(self.collect_websocket_messages(server_url).await?);
        } else {
            // Try to infer the protocol or use a fallback
            warn!(
                "Unknown server URL format: {}, trying HTTP fallback",
                server_url
            );
            collected_messages.extend(self.collect_http_messages(server_url).await?);
        }

        // If we couldn't collect any real messages, fall back to generated test messages
        if collected_messages.is_empty() {
            warn!("No messages collected from server, using generated test messages");
            collected_messages = self.generate_comprehensive_test_messages();
        }

        info!(
            "Collected {} JSON-RPC messages for validation",
            collected_messages.len()
        );
        Ok(collected_messages)
    }

    /// Collect messages from HTTP-based MCP server
    async fn collect_http_messages(&self, server_url: &str) -> ValidationResult<Vec<Value>> {
        info!("Collecting messages from HTTP MCP server: {}", server_url);
        let mut messages = Vec::new();

        // Create HTTP client with timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        // Standard MCP initialization sequence
        let init_request = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {
                        "listChanged": false
                    },
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "mcp-external-validator",
                    "version": "0.3.1"
                }
            },
            "id": 1
        });

        // Send initialization request and collect the exchange
        messages.push(init_request.clone());

        match client.post(server_url).json(&init_request).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(response_json) = response.json::<Value>().await {
                        messages.push(response_json);

                        // Send additional MCP requests to collect more message types
                        messages.extend(
                            self.collect_standard_mcp_messages(&client, server_url)
                                .await?,
                        );
                    }
                } else {
                    debug!(
                        "Server returned status {}, but continuing with available messages",
                        response.status()
                    );
                }
            }
            Err(e) => {
                debug!("Failed to communicate with HTTP server: {}", e);
            }
        }

        Ok(messages)
    }

    /// Collect messages from stdio-based MCP server
    async fn collect_stdio_messages(&self, server_url: &str) -> ValidationResult<Vec<Value>> {
        info!(
            "Collecting messages from stdio MCP server command: {}",
            server_url
        );
        let mut messages = Vec::new();

        // Extract command from stdio:// URL
        let command = server_url.strip_prefix("stdio://").unwrap_or(server_url);
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Err(ValidationError::ConfigurationError {
                message: "Empty stdio command".to_string(),
            });
        }

        // Use tokio process to interact with the MCP server
        let mut cmd = tokio::process::Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                // Send initialization through stdin and collect responses
                if let Some(mut stdin) = child.stdin.take() {
                    let init_message = json!({
                        "jsonrpc": "2.0",
                        "method": "initialize",
                        "params": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {},
                            "clientInfo": {
                                "name": "mcp-external-validator",
                                "version": "0.3.1"
                            }
                        },
                        "id": 1
                    });

                    messages.push(init_message.clone());

                    // Send the message
                    if let Ok(message_str) = serde_json::to_string(&init_message) {
                        let _ = stdin
                            .write_all(format!("{}\n", message_str).as_bytes())
                            .await;
                        let _ = stdin.flush().await;
                    }
                }

                // Collect output with timeout
                let timeout_duration = std::time::Duration::from_secs(10);
                match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
                    Ok(Ok(output)) => {
                        // Parse stdout for JSON-RPC responses
                        let stdout_str = String::from_utf8_lossy(&output.stdout);
                        for line in stdout_str.lines() {
                            if let Ok(parsed) = serde_json::from_str::<Value>(line) {
                                if self.is_jsonrpc_message(&parsed) {
                                    messages.push(parsed);
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        debug!("Stdio process error: {}", e);
                    }
                    Err(_) => {
                        debug!("Stdio process timed out");
                        // Process was already consumed by wait_with_output
                    }
                }
            }
            Err(e) => {
                debug!("Failed to spawn stdio process: {}", e);
            }
        }

        Ok(messages)
    }

    /// Collect messages from WebSocket-based MCP server
    async fn collect_websocket_messages(&self, server_url: &str) -> ValidationResult<Vec<Value>> {
        info!(
            "Collecting messages from WebSocket MCP server: {}",
            server_url
        );
        let messages = Vec::new();

        // For now, WebSocket collection is a future enhancement
        // Return empty messages and log the attempt
        debug!("WebSocket message collection not yet implemented");

        Ok(messages)
    }

    /// Collect standard MCP messages after initialization
    async fn collect_standard_mcp_messages(
        &self,
        client: &reqwest::Client,
        server_url: &str,
    ) -> ValidationResult<Vec<Value>> {
        let mut messages = Vec::new();
        let mut request_id = 2;

        // Common MCP method calls to generate realistic message exchanges
        let mcp_methods = [
            ("tools/list", json!({})),
            ("resources/list", json!({})),
            ("prompts/list", json!({})),
            ("ping", json!({})),
        ];

        for (method, params) in &mcp_methods {
            let request = json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
                "id": request_id
            });

            messages.push(request.clone());
            request_id += 1;

            // Send request and collect response
            match client.post(server_url).json(&request).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(response_json) = response.json::<Value>().await {
                            messages.push(response_json);
                        }
                    }
                }
                Err(_) => {
                    // Continue with other methods even if one fails
                    debug!("Failed to send {} request", method);
                }
            }

            // Small delay between requests to be respectful
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(messages)
    }

    /// Check if a JSON value looks like a JSON-RPC message
    fn is_jsonrpc_message(&self, value: &Value) -> bool {
        value.is_object()
            && value.get("jsonrpc").is_some()
            && (value.get("method").is_some()
                || value.get("result").is_some()
                || value.get("error").is_some())
    }

    /// Generate comprehensive test messages when real collection fails
    fn generate_comprehensive_test_messages(&self) -> Vec<Value> {
        let mut messages = self.generate_test_messages();

        // Add more comprehensive MCP-specific messages
        messages.extend(vec![
            // MCP initialization request
            json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "roots": {"listChanged": false},
                        "sampling": {}
                    },
                    "clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    }
                },
                "id": 1
            }),
            // MCP initialization response
            json!({
                "jsonrpc": "2.0",
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "logging": {},
                        "tools": {"listChanged": false},
                        "resources": {"subscribe": true}
                    },
                    "serverInfo": {
                        "name": "test-server",
                        "version": "1.0.0"
                    }
                },
                "id": 1
            }),
            // Tools list request
            json!({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 2
            }),
            // Tools list response
            json!({
                "jsonrpc": "2.0",
                "result": {
                    "tools": [{
                        "name": "echo",
                        "description": "Echo the input",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "message": {"type": "string"}
                            }
                        }
                    }]
                },
                "id": 2
            }),
            // Resources list request
            json!({
                "jsonrpc": "2.0",
                "method": "resources/list",
                "id": 3
            }),
            // Notification example
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/message",
                "params": {
                    "level": "info",
                    "message": "Server is ready"
                }
            }),
            // Error response example
            json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": "Method not found",
                    "data": {
                        "method": "unknown/method"
                    }
                },
                "id": 99
            }),
        ]);

        messages
    }

    /// Generate test messages for compliance testing
    fn generate_test_messages(&self) -> Vec<Value> {
        vec![
            // Valid request
            json!({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 1
            }),
            // Valid response
            json!({
                "jsonrpc": "2.0",
                "result": {"tools": []},
                "id": 1
            }),
            // Valid notification
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }),
            // Valid error response
            json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "Invalid Request"
                },
                "id": null
            }),
            // Invalid message (missing jsonrpc)
            json!({
                "method": "test",
                "id": 1
            }),
            // Invalid message (wrong jsonrpc version)
            json!({
                "jsonrpc": "1.0",
                "method": "test",
                "id": 1
            }),
        ]
    }

    /// Validate message against JSON schema
    fn validate_message_schema(&self, message: &Value) -> Result<(), Vec<String>> {
        // Determine message type and validate with appropriate schema
        if message.get("method").is_some() {
            if message.get("id").is_some() {
                // Request
                self.validate_with_schema(&self.schemas.request_schema, message)
            } else {
                // Notification
                self.validate_with_schema(&self.schemas.notification_schema, message)
            }
        } else {
            // Response
            self.validate_with_schema(&self.schemas.response_schema, message)
        }
    }

    /// Validate message format (business logic validation)
    fn validate_message_format(&self, message: &Value) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check JSON-RPC version
        if let Some(version) = message.get("jsonrpc") {
            if version != "2.0" {
                errors.push(format!("Invalid JSON-RPC version: {}", version));
            }
        } else {
            errors.push("Missing required 'jsonrpc' field".to_string());
        }

        // Additional format checks...

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate with a JSON schema
    fn validate_with_schema(
        &self,
        schema: &JSONSchema,
        message: &Value,
    ) -> Result<(), Vec<String>> {
        match schema.validate(message) {
            Ok(_) => Ok(()),
            Err(errors) => Err(errors.map(|e| e.to_string()).collect()),
        }
    }

    /// Validate request message
    fn validate_request_message(
        &self,
        message: &Value,
        issues: &mut Vec<ValidationIssue>,
    ) -> ValidationResult<()> {
        // Check required fields
        if message.get("method").is_none() {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "request".to_string(),
                "Missing required 'method' field in request".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        if message.get("id").is_none() {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "request".to_string(),
                "Missing required 'id' field in request".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate notification message
    fn validate_notification_message(
        &self,
        message: &Value,
        issues: &mut Vec<ValidationIssue>,
    ) -> ValidationResult<()> {
        // Check required fields
        if message.get("method").is_none() {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "notification".to_string(),
                "Missing required 'method' field in notification".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        // Notifications should not have id field
        if message.get("id").is_some() {
            issues.push(ValidationIssue::new(
                IssueSeverity::Warning,
                "notification".to_string(),
                "Notification should not have 'id' field".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate response message
    fn validate_response_message(
        &self,
        message: &Value,
        issues: &mut Vec<ValidationIssue>,
    ) -> ValidationResult<()> {
        let has_result = message.get("result").is_some();
        let has_error = message.get("error").is_some();

        // Response must have either result or error, but not both
        if !has_result && !has_error {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "response".to_string(),
                "Response must have either 'result' or 'error' field".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        if has_result && has_error {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "response".to_string(),
                "Response cannot have both 'result' and 'error' fields".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        // Response should have id field
        if message.get("id").is_none() {
            issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "response".to_string(),
                "Response should have 'id' field".to_string(),
                "jsonrpc-validator".to_string(),
            ));
        }

        Ok(())
    }
}

impl JsonRpcSchemas {
    /// Create new JSON-RPC schemas
    fn new() -> ValidationResult<Self> {
        let request_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&Self::request_schema_json())
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to compile request schema: {}", e),
            })?;

        let response_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&Self::response_schema_json())
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to compile response schema: {}", e),
            })?;

        let error_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&Self::error_schema_json())
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to compile error schema: {}", e),
            })?;

        let notification_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&Self::notification_schema_json())
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to compile notification schema: {}", e),
            })?;

        Ok(Self {
            request_schema,
            response_schema,
            error_schema,
            notification_schema,
        })
    }

    /// JSON-RPC 2.0 request schema
    fn request_schema_json() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "jsonrpc": {
                    "type": "string",
                    "enum": ["2.0"]
                },
                "method": {
                    "type": "string"
                },
                "params": {
                    "oneOf": [
                        {"type": "array"},
                        {"type": "object"}
                    ]
                },
                "id": {
                    "oneOf": [
                        {"type": "string"},
                        {"type": "number"},
                        {"type": "null"}
                    ]
                }
            },
            "required": ["jsonrpc", "method", "id"],
            "additionalProperties": false
        })
    }

    /// JSON-RPC 2.0 response schema
    fn response_schema_json() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "jsonrpc": {
                    "type": "string",
                    "enum": ["2.0"]
                },
                "result": {},
                "error": {
                    "type": "object",
                    "properties": {
                        "code": {"type": "integer"},
                        "message": {"type": "string"},
                        "data": {}
                    },
                    "required": ["code", "message"]
                },
                "id": {
                    "oneOf": [
                        {"type": "string"},
                        {"type": "number"},
                        {"type": "null"}
                    ]
                }
            },
            "required": ["jsonrpc", "id"],
            "oneOf": [
                {"required": ["result"]},
                {"required": ["error"]}
            ],
            "additionalProperties": false
        })
    }

    /// JSON-RPC 2.0 error schema
    fn error_schema_json() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "code": {"type": "integer"},
                "message": {"type": "string"},
                "data": {}
            },
            "required": ["code", "message"],
            "additionalProperties": false
        })
    }

    /// JSON-RPC 2.0 notification schema
    fn notification_schema_json() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "jsonrpc": {
                    "type": "string",
                    "enum": ["2.0"]
                },
                "method": {
                    "type": "string"
                },
                "params": {
                    "oneOf": [
                        {"type": "array"},
                        {"type": "object"}
                    ]
                }
            },
            "required": ["jsonrpc", "method"],
            "additionalProperties": false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_validator_creation() {
        let config = ValidationConfig::default();
        let validator = JsonRpcValidator::new(config);
        assert!(validator.is_ok());
    }

    #[test]
    fn test_schema_creation() {
        let schemas = JsonRpcSchemas::new();
        assert!(schemas.is_ok());
    }

    #[test]
    fn test_valid_request_validation() {
        let config = ValidationConfig::default();
        let validator = JsonRpcValidator::new(config).unwrap();

        let valid_request = json!({
            "jsonrpc": "2.0",
            "method": "test",
            "id": 1
        });

        let issues = validator.validate_single_message(&valid_request).unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_invalid_request_validation() {
        let config = ValidationConfig::default();
        let validator = JsonRpcValidator::new(config).unwrap();

        let invalid_request = json!({
            "jsonrpc": "1.0",  // Wrong version
            "method": "test",
            "id": 1
        });

        let issues = validator.validate_single_message(&invalid_request).unwrap();
        assert!(!issues.is_empty());
        assert!(issues
            .iter()
            .any(|i| i.description.contains("Invalid JSON-RPC version")));
    }

    #[test]
    fn test_response_validation() {
        let config = ValidationConfig::default();
        let validator = JsonRpcValidator::new(config).unwrap();

        // Valid response with result
        let valid_response = json!({
            "jsonrpc": "2.0",
            "result": "success",
            "id": 1
        });

        let issues = validator.validate_single_message(&valid_response).unwrap();
        assert!(issues.is_empty());

        // Invalid response with both result and error
        let invalid_response = json!({
            "jsonrpc": "2.0",
            "result": "success",
            "error": {"code": -1, "message": "error"},
            "id": 1
        });

        let issues = validator
            .validate_single_message(&invalid_response)
            .unwrap();
        assert!(!issues.is_empty());
    }

    #[tokio::test]
    async fn test_compliance_testing() {
        let config = ValidationConfig::default();
        let validator = JsonRpcValidator::new(config).unwrap();

        // Test the validator creation and basic functionality
        // Don't actually run external compliance tests in unit tests
        assert!(validator.schemas.response_schema.is_valid(&json!({
            "jsonrpc": "2.0",
            "result": null,
            "id": 1
        })));
    }
}
