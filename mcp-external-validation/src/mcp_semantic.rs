//! MCP Protocol Semantic Validation
//!
//! This module provides comprehensive semantic validation of MCP protocol messages
//! beyond basic JSON-RPC compliance. It validates MCP-specific semantics, state
//! transitions, and protocol compliance.

use crate::{
    report::{IssueSeverity, TestScore, ValidationIssue},
    ValidationConfig, ValidationResult,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tracing::info;

/// MCP protocol semantic validator
pub struct McpSemanticValidator {
    config: ValidationConfig,
    /// Track initialization state
    initialized: bool,
    /// Track discovered capabilities
    server_capabilities: HashMap<String, Value>,
    /// Track client capabilities
    client_capabilities: HashMap<String, Value>,
    /// Track available tools
    available_tools: HashSet<String>,
    /// Track available resources
    available_resources: HashSet<String>,
    /// Track available prompts
    available_prompts: HashSet<String>,
    /// Track message sequence for state validation
    message_sequence: Vec<MessageContext>,
}

/// Context for tracking message sequences and state
#[derive(Debug, Clone)]
struct MessageContext {
    /// Message type (request, response, notification)
    message_type: MessageType,
    /// Method name for requests/notifications
    method: Option<String>,
    /// Request ID for correlation
    id: Option<Value>,
    /// Timestamp for ordering
    timestamp: std::time::SystemTime,
    /// Success status for responses
    success: bool,
}

/// Message type classification
#[derive(Debug, Clone, PartialEq)]
enum MessageType {
    Request,
    Response,
    Notification,
    Error,
}

/// MCP protocol validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSemanticResult {
    /// Initialization sequence validation
    pub initialization: TestScore,
    /// Capability validation
    pub capabilities: TestScore,
    /// Method compliance validation  
    pub method_compliance: TestScore,
    /// State transition validation
    pub state_transitions: TestScore,
    /// Schema compliance validation
    pub schema_compliance: TestScore,
    /// Security validation
    pub security: TestScore,
    /// Issues found during validation
    pub issues: Vec<ValidationIssue>,
}

impl McpSemanticValidator {
    /// Create a new MCP semantic validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            initialized: false,
            server_capabilities: HashMap::new(),
            client_capabilities: HashMap::new(),
            available_tools: HashSet::new(),
            available_resources: HashSet::new(),
            available_prompts: HashSet::new(),
            message_sequence: Vec::new(),
        }
    }

    /// Validate MCP protocol semantics for a sequence of messages
    pub async fn validate_protocol_semantics(
        &mut self,
        messages: &[Value],
    ) -> ValidationResult<McpSemanticResult> {
        info!(
            "Starting MCP protocol semantic validation for {} messages",
            messages.len()
        );

        let mut result = McpSemanticResult {
            initialization: TestScore::new(0, 0),
            capabilities: TestScore::new(0, 0),
            method_compliance: TestScore::new(0, 0),
            state_transitions: TestScore::new(0, 0),
            schema_compliance: TestScore::new(0, 0),
            security: TestScore::new(0, 0),
            issues: Vec::new(),
        };

        // Reset validator state
        self.reset_state();

        // Process messages in sequence to track state transitions
        for (index, message) in messages.iter().enumerate() {
            self.process_message(message, index, &mut result).await?;
        }

        // Validate overall protocol compliance
        self.validate_protocol_flow(&mut result)?;

        info!(
            "MCP semantic validation completed with {} issues",
            result.issues.len()
        );
        Ok(result)
    }

    /// Reset validator state for a new validation session
    fn reset_state(&mut self) {
        self.initialized = false;
        self.server_capabilities.clear();
        self.client_capabilities.clear();
        self.available_tools.clear();
        self.available_resources.clear();
        self.available_prompts.clear();
        self.message_sequence.clear();
    }

    /// Process a single message and update validation state
    async fn process_message(
        &mut self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Classify the message
        let message_context = self.classify_message(message, index);

        // Track the message in our sequence
        self.message_sequence.push(message_context.clone());

        // Validate based on message type and current state
        match message_context.message_type {
            MessageType::Request => {
                self.validate_request_message(message, index, result)
                    .await?;
            }
            MessageType::Response => {
                self.validate_response_message(message, index, result)
                    .await?;
            }
            MessageType::Notification => {
                self.validate_notification_message(message, index, result)
                    .await?;
            }
            MessageType::Error => {
                self.validate_error_message(message, index, result).await?;
            }
        }

        Ok(())
    }

    /// Classify a message by type
    fn classify_message(&self, message: &Value, _index: usize) -> MessageContext {
        let timestamp = std::time::SystemTime::now();
        let id = message.get("id").cloned();

        let message_type = if message.get("method").is_some() {
            if id.is_some() {
                MessageType::Request
            } else {
                MessageType::Notification
            }
        } else if message.get("error").is_some() {
            MessageType::Error
        } else if message.get("result").is_some() || id.is_some() {
            MessageType::Response
        } else {
            MessageType::Request // Default fallback
        };

        let method = message
            .get("method")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        let success = message.get("error").is_none();

        MessageContext {
            message_type,
            method,
            id,
            timestamp,
            success,
        }
    }

    /// Validate an MCP request message
    async fn validate_request_message(
        &mut self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
            result.method_compliance.total += 1;

            match method {
                "initialize" => {
                    self.validate_initialize_request(message, index, result)?;
                }
                "initialized" => {
                    self.validate_initialized_notification(message, index, result)?;
                }
                "tools/list" => {
                    self.validate_tools_list_request(message, index, result)?;
                }
                "tools/call" => {
                    self.validate_tools_call_request(message, index, result)?;
                }
                "resources/list" => {
                    self.validate_resources_list_request(message, index, result)?;
                }
                "resources/read" => {
                    self.validate_resources_read_request(message, index, result)?;
                }
                "prompts/list" => {
                    self.validate_prompts_list_request(message, index, result)?;
                }
                "prompts/get" => {
                    self.validate_prompts_get_request(message, index, result)?;
                }
                "ping" => {
                    self.validate_ping_request(message, index, result)?;
                }
                "logging/setLevel" => {
                    self.validate_logging_request(message, index, result)?;
                }
                "sampling/createMessage" => {
                    self.validate_sampling_request(message, index, result)?;
                }
                _ => {
                    // Unknown method - check if it follows MCP naming conventions
                    self.validate_custom_method(method, message, index, result)?;
                }
            }

            if result
                .issues
                .iter()
                .filter(|i| i.category == "method_compliance")
                .count()
                == 0
            {
                result.method_compliance.passed += 1;
            }
        }

        Ok(())
    }

    /// Validate MCP initialize request
    fn validate_initialize_request(
        &mut self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        result.initialization.total += 1;

        // Check if already initialized
        if self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "initialization".to_string(),
                format!(
                    "Message {}: Multiple initialize requests not allowed",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
            return Ok(());
        }

        if let Some(params) = message.get("params") {
            // Validate required parameters
            self.validate_required_field(params, "protocolVersion", index, result)?;
            self.validate_required_field(params, "capabilities", index, result)?;
            self.validate_required_field(params, "clientInfo", index, result)?;

            // Validate protocol version
            if let Some(version) = params.get("protocolVersion").and_then(|v| v.as_str()) {
                if !self.is_supported_protocol_version(version) {
                    result.issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "initialization".to_string(),
                        format!(
                            "Message {}: Unsupported protocol version: {}",
                            index, version
                        ),
                        "mcp-semantic".to_string(),
                    ));
                }
            }

            // Store client capabilities
            if let Some(capabilities) = params.get("capabilities") {
                self.client_capabilities = self.extract_capabilities(capabilities);
            }

            result.initialization.passed += 1;
        } else {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "initialization".to_string(),
                format!("Message {}: Initialize request missing params", index),
                "mcp-semantic".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate MCP initialize response
    async fn validate_response_message(
        &mut self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Find the corresponding request
        let request_id = message.get("id");
        let corresponding_request = self.find_corresponding_request(request_id);

        if let Some(request_method) = corresponding_request {
            match request_method.as_str() {
                "initialize" => {
                    self.validate_initialize_response(message, index, result)?;
                }
                "tools/list" => {
                    self.validate_tools_list_response(message, index, result)?;
                }
                "resources/list" => {
                    self.validate_resources_list_response(message, index, result)?;
                }
                "prompts/list" => {
                    self.validate_prompts_list_response(message, index, result)?;
                }
                _ => {
                    // Generic response validation
                    self.validate_generic_response(message, index, result)?;
                }
            }
        }

        Ok(())
    }

    /// Validate notification messages
    async fn validate_notification_message(
        &mut self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if let Some(method) = message.get("method").and_then(|m| m.as_str()) {
            match method {
                "notifications/initialized" => {
                    self.validate_initialized_notification(message, index, result)?;
                }
                "notifications/cancelled" => {
                    self.validate_cancelled_notification(message, index, result)?;
                }
                "notifications/progress" => {
                    self.validate_progress_notification(message, index, result)?;
                }
                "notifications/message" => {
                    self.validate_message_notification(message, index, result)?;
                }
                "notifications/resources/updated" => {
                    self.validate_resources_updated_notification(message, index, result)?;
                }
                "notifications/tools/listChanged" => {
                    self.validate_tools_list_changed_notification(message, index, result)?;
                }
                _ => {
                    result.issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "notifications".to_string(),
                        format!("Message {}: Unknown notification method: {}", index, method),
                        "mcp-semantic".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate error messages
    async fn validate_error_message(
        &mut self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if let Some(error) = message.get("error") {
            // Validate error structure
            self.validate_required_field(error, "code", index, result)?;
            self.validate_required_field(error, "message", index, result)?;

            // Validate error codes are within MCP specification
            if let Some(code) = error.get("code").and_then(|c| c.as_i64()) {
                if !self.is_valid_mcp_error_code(code) {
                    result.issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "error_handling".to_string(),
                        format!("Message {}: Non-standard MCP error code: {}", index, code),
                        "mcp-semantic".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate overall protocol flow
    fn validate_protocol_flow(&mut self, result: &mut McpSemanticResult) -> ValidationResult<()> {
        result.state_transitions.total += 1;

        // Check that initialization happened first if we have initialize messages
        let has_initialize = self
            .message_sequence
            .iter()
            .any(|msg| msg.method.as_ref().map_or(false, |m| m == "initialize"));

        if has_initialize {
            // Find first initialize message
            let first_initialize_index = self
                .message_sequence
                .iter()
                .position(|msg| msg.method.as_ref().map_or(false, |m| m == "initialize"));

            // Check that no non-initialization messages come before initialize
            if let Some(init_index) = first_initialize_index {
                for (i, msg) in self.message_sequence.iter().enumerate() {
                    if i < init_index && msg.message_type == MessageType::Request {
                        if let Some(method) = &msg.method {
                            if method != "initialize" {
                                result.issues.push(ValidationIssue::new(
                                    IssueSeverity::Error,
                                    "state_transitions".to_string(),
                                    format!("Method '{}' called before initialization", method),
                                    "mcp-semantic".to_string(),
                                ));
                                return Ok(());
                            }
                        }
                    }
                }
            }

            result.state_transitions.passed += 1;
        }

        Ok(())
    }

    /// Helper methods for specific validations
    fn validate_required_field(
        &self,
        obj: &Value,
        field: &str,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if obj.get(field).is_none() {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "schema_compliance".to_string(),
                format!("Message {}: Missing required field '{}'", index, field),
                "mcp-semantic".to_string(),
            ));
        }
        Ok(())
    }

    fn is_supported_protocol_version(&self, version: &str) -> bool {
        // Current MCP protocol versions
        matches!(
            version,
            "2025-06-18" | "2025-03-26" | "2024-11-05" | "2024-10-07" | "2024-09-25"
        )
    }

    fn is_valid_mcp_error_code(&self, code: i64) -> bool {
        // Standard JSON-RPC errors plus MCP-specific errors
        match code {
            // JSON-RPC standard errors
            -32700..=-32000 => true,
            // MCP-specific error codes (if any)
            -1000..=-1 => true,
            _ => false,
        }
    }

    fn extract_capabilities(&self, capabilities: &Value) -> HashMap<String, Value> {
        let mut caps = HashMap::new();
        if let Some(obj) = capabilities.as_object() {
            for (key, value) in obj {
                caps.insert(key.clone(), value.clone());
            }
        }
        caps
    }

    fn find_corresponding_request(&self, response_id: Option<&Value>) -> Option<String> {
        if let Some(id) = response_id {
            // Find the most recent request with matching ID
            for msg in self.message_sequence.iter().rev() {
                if msg.message_type == MessageType::Request && msg.id.as_ref() == Some(id) {
                    return msg.method.clone();
                }
            }
        }
        None
    }

    // Specific validation implementations
    fn validate_initialized_notification(
        &mut self,
        _message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        result.initialization.total += 1;

        if self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "initialization".to_string(),
                format!(
                    "Message {}: Already initialized, duplicate notification",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
            return Ok(());
        }

        // Check for initialization context
        let has_prior_initialize = self
            .message_sequence
            .iter()
            .any(|msg| msg.method.as_ref().map_or(false, |m| m == "initialize"));

        if !has_prior_initialize {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "initialization".to_string(),
                format!(
                    "Message {}: Initialized notification without prior initialize request",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
            return Ok(());
        }

        self.initialized = true;
        result.initialization.passed += 1;
        Ok(())
    }

    fn validate_tools_list_request(
        &self,
        _message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if !self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "state_transitions".to_string(),
                format!("Message {}: tools/list called before initialization", index),
                "mcp-semantic".to_string(),
            ));
            return Ok(());
        }
        Ok(())
    }

    fn validate_tools_call_request(
        &self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if !self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "state_transitions".to_string(),
                format!("Message {}: tools/call called before initialization", index),
                "mcp-semantic".to_string(),
            ));
            return Ok(());
        }

        // Validate tool name is provided
        if let Some(params) = message.get("params") {
            if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                if !self.available_tools.contains(name) {
                    result.issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "method_compliance".to_string(),
                        format!("Message {}: Calling unknown tool '{}'", index, name),
                        "mcp-semantic".to_string(),
                    ));
                }
            } else {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "method_compliance".to_string(),
                    format!(
                        "Message {}: tools/call missing required 'name' parameter",
                        index
                    ),
                    "mcp-semantic".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_resources_list_request(
        &self,
        _message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if !self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "state_transitions".to_string(),
                format!(
                    "Message {}: resources/list called before initialization",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_resources_read_request(
        &self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if !self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "state_transitions".to_string(),
                format!(
                    "Message {}: resources/read called before initialization",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
            return Ok(());
        }

        // Validate URI is provided
        if let Some(params) = message.get("params") {
            if params.get("uri").is_none() {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "method_compliance".to_string(),
                    format!(
                        "Message {}: resources/read missing required 'uri' parameter",
                        index
                    ),
                    "mcp-semantic".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_prompts_list_request(
        &self,
        _message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if !self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "state_transitions".to_string(),
                format!(
                    "Message {}: prompts/list called before initialization",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_prompts_get_request(
        &self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if !self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "state_transitions".to_string(),
                format!(
                    "Message {}: prompts/get called before initialization",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
            return Ok(());
        }

        // Validate name is provided
        if let Some(params) = message.get("params") {
            if params.get("name").is_none() {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "method_compliance".to_string(),
                    format!(
                        "Message {}: prompts/get missing required 'name' parameter",
                        index
                    ),
                    "mcp-semantic".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_ping_request(
        &self,
        _message: &Value,
        _index: usize,
        _result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Ping can be called at any time
        Ok(())
    }

    fn validate_logging_request(
        &self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Validate log level if provided
        if let Some(params) = message.get("params") {
            if let Some(level) = params.get("level").and_then(|l| l.as_str()) {
                if !matches!(level, "debug" | "info" | "warning" | "error") {
                    result.issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "method_compliance".to_string(),
                        format!("Message {}: Invalid logging level '{}'", index, level),
                        "mcp-semantic".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_sampling_request(
        &self,
        _message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if !self.initialized {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "state_transitions".to_string(),
                format!(
                    "Message {}: sampling/createMessage called before initialization",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_custom_method(
        &self,
        method: &str,
        _message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Check if method follows MCP naming conventions
        if !method.contains('/') {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Warning,
                "method_compliance".to_string(),
                format!("Message {}: Custom method '{}' doesn't follow MCP naming convention (should contain '/')", index, method),
                "mcp-semantic".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_initialize_response(
        &mut self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        result.initialization.total += 1;

        if let Some(response_result) = message.get("result") {
            // Validate required fields
            self.validate_required_field(response_result, "protocolVersion", index, result)?;
            self.validate_required_field(response_result, "capabilities", index, result)?;
            self.validate_required_field(response_result, "serverInfo", index, result)?;

            // Store server capabilities
            if let Some(capabilities) = response_result.get("capabilities") {
                self.server_capabilities = self.extract_capabilities(capabilities);
            }

            result.initialization.passed += 1;
        } else {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "initialization".to_string(),
                format!("Message {}: Initialize response missing result", index),
                "mcp-semantic".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_tools_list_response(
        &mut self,
        message: &Value,
        _index: usize,
        _result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if let Some(response_result) = message.get("result") {
            if let Some(tools) = response_result.get("tools").and_then(|t| t.as_array()) {
                for tool in tools {
                    if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                        self.available_tools.insert(name.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_resources_list_response(
        &mut self,
        message: &Value,
        _index: usize,
        _result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if let Some(response_result) = message.get("result") {
            if let Some(resources) = response_result.get("resources").and_then(|r| r.as_array()) {
                for resource in resources {
                    if let Some(uri) = resource.get("uri").and_then(|u| u.as_str()) {
                        self.available_resources.insert(uri.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_prompts_list_response(
        &mut self,
        message: &Value,
        _index: usize,
        _result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        if let Some(response_result) = message.get("result") {
            if let Some(prompts) = response_result.get("prompts").and_then(|p| p.as_array()) {
                for prompt in prompts {
                    if let Some(name) = prompt.get("name").and_then(|n| n.as_str()) {
                        self.available_prompts.insert(name.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_generic_response(
        &self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Generic response validation - check it has either result or error
        if message.get("result").is_none() && message.get("error").is_none() {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "schema_compliance".to_string(),
                format!(
                    "Message {}: Response missing both 'result' and 'error'",
                    index
                ),
                "mcp-semantic".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_cancelled_notification(
        &self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Validate request ID is provided
        if let Some(params) = message.get("params") {
            if params.get("requestId").is_none() {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "method_compliance".to_string(),
                    format!(
                        "Message {}: cancelled notification missing 'requestId'",
                        index
                    ),
                    "mcp-semantic".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_progress_notification(
        &self,
        message: &Value,
        index: usize,
        result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Validate progress token and value
        if let Some(params) = message.get("params") {
            if params.get("progressToken").is_none() {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Error,
                    "method_compliance".to_string(),
                    format!(
                        "Message {}: progress notification missing 'progressToken'",
                        index
                    ),
                    "mcp-semantic".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_message_notification(
        &self,
        _message: &Value,
        _index: usize,
        _result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Message notifications are generally free-form
        Ok(())
    }

    fn validate_resources_updated_notification(
        &self,
        _message: &Value,
        _index: usize,
        _result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Resources updated notifications don't require specific validation
        Ok(())
    }

    fn validate_tools_list_changed_notification(
        &self,
        _message: &Value,
        _index: usize,
        _result: &mut McpSemanticResult,
    ) -> ValidationResult<()> {
        // Tools list changed notifications don't require specific validation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_semantic_validator_creation() {
        let config = ValidationConfig::default();
        let validator = McpSemanticValidator::new(config);
        assert!(!validator.initialized);
    }

    #[tokio::test]
    async fn test_initialize_sequence_validation() {
        let mut validator = McpSemanticValidator::new(ValidationConfig::default());

        let messages = vec![
            json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "test", "version": "1.0"}
                },
                "id": 1
            }),
            json!({
                "jsonrpc": "2.0",
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "serverInfo": {"name": "test-server", "version": "1.0"}
                },
                "id": 1
            }),
        ];

        let result = validator
            .validate_protocol_semantics(&messages)
            .await
            .unwrap();
        // The validation counts both request and response
        assert_eq!(result.initialization.passed, 2);
        assert_eq!(result.initialization.total, 2);
    }

    #[tokio::test]
    async fn test_invalid_protocol_version() {
        let mut validator = McpSemanticValidator::new(ValidationConfig::default());

        let messages = vec![json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "invalid-version",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            },
            "id": 1
        })];

        let result = validator
            .validate_protocol_semantics(&messages)
            .await
            .unwrap();
        assert!(result
            .issues
            .iter()
            .any(|i| i.description.contains("Unsupported protocol version")));
    }

    #[tokio::test]
    async fn test_method_before_initialization() {
        let mut validator = McpSemanticValidator::new(ValidationConfig::default());

        let messages = vec![
            json!({
                "jsonrpc": "2.0",
                "method": "tools/list",
                "id": 1
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "test", "version": "1.0"}
                },
                "id": 2
            }),
        ];

        let result = validator
            .validate_protocol_semantics(&messages)
            .await
            .unwrap();
        assert!(result
            .issues
            .iter()
            .any(|i| i.description.contains("called before initialization")));
    }
}
