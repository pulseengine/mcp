//! MCP Request Security Validation and Sanitization
//!
//! This module provides comprehensive security validation for MCP requests,
//! including parameter sanitization, size limits, and injection protection.

use crate::AuthContext;
use pulseengine_mcp_protocol::Request;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use tracing::{debug, error, warn};

/// Errors that can occur during security validation
#[derive(Debug, Error)]
pub enum SecurityValidationError {
    #[error("Request too large: {current} bytes exceeds limit of {limit} bytes")]
    RequestTooLarge { current: usize, limit: usize },

    #[error("Parameter value too large: {param} has {current} bytes, limit is {limit} bytes")]
    ParameterTooLarge {
        param: String,
        current: usize,
        limit: usize,
    },

    #[error("Too many parameters: {current} exceeds limit of {limit}")]
    TooManyParameters { current: usize, limit: usize },

    #[error("Invalid parameter name: {name}")]
    InvalidParameterName { name: String },

    #[error("Potential injection attack detected in parameter: {param}")]
    InjectionDetected { param: String },

    #[error("Malicious content detected: {reason}")]
    MaliciousContent { reason: String },

    #[error("Rate limit exceeded for method: {method}")]
    RateLimitExceeded { method: String },

    #[error("Unsupported method: {method}")]
    UnsupportedMethod { method: String },
}

/// Security violation details for logging and monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityViolation {
    /// Type of violation
    pub violation_type: SecurityViolationType,

    /// Severity level
    pub severity: SecuritySeverity,

    /// Description of the violation
    pub description: String,

    /// Parameter or field involved
    pub field: Option<String>,

    /// Original value that triggered the violation
    pub value: Option<String>,

    /// Timestamp of the violation
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of security violations
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SecurityViolationType {
    SizeLimit,
    ParameterLimit,
    InjectionAttempt,
    MaliciousContent,
    InvalidFormat,
    RateLimit,
    UnauthorizedMethod,
}

/// Security severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Configuration for request size and complexity limits
#[derive(Debug, Clone)]
pub struct RequestLimitsConfig {
    /// Maximum request size in bytes
    pub max_request_size: usize,

    /// Maximum number of parameters
    pub max_parameters: usize,

    /// Maximum size for any single parameter value
    pub max_parameter_size: usize,

    /// Maximum string length for text parameters
    pub max_string_length: usize,

    /// Maximum array length
    pub max_array_length: usize,

    /// Maximum object depth (nested objects)
    pub max_object_depth: usize,

    /// Maximum number of keys in an object
    pub max_object_keys: usize,
}

impl Default for RequestLimitsConfig {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024, // 10MB
            max_parameters: 100,
            max_parameter_size: 1024 * 1024, // 1MB
            max_string_length: 10000,
            max_array_length: 1000,
            max_object_depth: 10,
            max_object_keys: 100,
        }
    }
}

/// Configuration for request security validation
#[derive(Debug, Clone)]
pub struct RequestSecurityConfig {
    /// Enable request validation
    pub enabled: bool,

    /// Request size and complexity limits
    pub limits: RequestLimitsConfig,

    /// Enable injection attack detection
    pub enable_injection_detection: bool,

    /// Enable parameter sanitization
    pub enable_sanitization: bool,

    /// Allowed methods (empty means all allowed)
    pub allowed_methods: HashSet<String>,

    /// Blocked methods
    pub blocked_methods: HashSet<String>,

    /// Enable rate limiting per method
    pub enable_method_rate_limiting: bool,

    /// Method rate limits (method -> requests per minute)
    pub method_rate_limits: HashMap<String, u32>,

    /// Log security violations
    pub log_violations: bool,

    /// Fail on security violations (vs warn and continue)
    pub fail_on_violations: bool,
}

impl Default for RequestSecurityConfig {
    fn default() -> Self {
        let mut method_rate_limits = HashMap::new();
        method_rate_limits.insert("tools/call".to_string(), 60); // 1 per second
        method_rate_limits.insert("resources/read".to_string(), 120); // 2 per second

        Self {
            enabled: true,
            limits: RequestLimitsConfig::default(),
            enable_injection_detection: true,
            enable_sanitization: true,
            allowed_methods: HashSet::new(), // Empty means all allowed
            blocked_methods: HashSet::new(),
            enable_method_rate_limiting: false, // Disabled by default
            method_rate_limits,
            log_violations: true,
            fail_on_violations: true,
        }
    }
}

/// Input sanitizer for removing/escaping dangerous content
pub struct InputSanitizer {
    /// SQL injection patterns
    sql_patterns: Vec<Regex>,

    /// XSS patterns
    xss_patterns: Vec<Regex>,

    /// Command injection patterns
    command_patterns: Vec<Regex>,

    /// Path traversal patterns
    path_traversal_patterns: Vec<Regex>,
}

impl InputSanitizer {
    /// Create a new input sanitizer
    pub fn new() -> Self {
        Self {
            sql_patterns: Self::build_sql_patterns(),
            xss_patterns: Self::build_xss_patterns(),
            command_patterns: Self::build_command_patterns(),
            path_traversal_patterns: Self::build_path_traversal_patterns(),
        }
    }

    /// Build SQL injection detection patterns
    fn build_sql_patterns() -> Vec<Regex> {
        let patterns = [
            r"(?i)(union\s+select|select\s+.*\s+from|insert\s+into|delete\s+from|drop\s+table)",
            r"(?i)(exec\s*\(|execute\s*\(|sp_|xp_)",
            r"(?i)(\bor\b\s+\d+\s*=\s*\d+|\band\b\s+\d+\s*=\s*\d+)",
            r"(?i)(sleep\s*\(|benchmark\s*\(|waitfor\s+delay)",
            r#"['";]\s*(\bunion\b|\bselect\b|\binsert\b|\bdelete\b|\bdrop\b)"#,
        ];

        patterns
            .iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect()
    }

    /// Build XSS detection patterns
    fn build_xss_patterns() -> Vec<Regex> {
        let patterns = [
            r"(?i)<script[^>]*>.*?</script>",
            r"(?i)javascript:",
            r"(?i)on\w+\s*=",
            r"(?i)<iframe[^>]*>.*?</iframe>",
            r"(?i)eval\s*\(",
        ];

        patterns
            .iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect()
    }

    /// Build command injection detection patterns
    fn build_command_patterns() -> Vec<Regex> {
        let patterns = [
            r#"[;&|`$()]"#,
            r"(?i)(cmd|powershell|bash|sh)\s",
            r"\.\.\/",
            r"(?i)(\bcat\b|\bls\b|\bpwd\b|\bwhoami\b|\bps\b|\btop\b)",
        ];

        patterns
            .iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect()
    }

    /// Build path traversal detection patterns
    fn build_path_traversal_patterns() -> Vec<Regex> {
        let patterns = [
            r"\.\.\/",
            r"\.\.\\",
            r"%2e%2e%2f",
            r"%2e%2e%5c",
            r"(?i)\.\.[\\/]",
        ];

        patterns
            .iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect()
    }

    /// Check if a string contains potential injection attempts
    pub fn detect_injection(&self, value: &str) -> Vec<String> {
        let mut violations = Vec::new();

        // Check SQL injection
        for pattern in &self.sql_patterns {
            if pattern.is_match(value) {
                violations.push("SQL injection attempt detected".to_string());
                break;
            }
        }

        // Check XSS
        for pattern in &self.xss_patterns {
            if pattern.is_match(value) {
                violations.push("XSS attempt detected".to_string());
                break;
            }
        }

        // Check command injection
        for pattern in &self.command_patterns {
            if pattern.is_match(value) {
                violations.push("Command injection attempt detected".to_string());
                break;
            }
        }

        // Check path traversal
        for pattern in &self.path_traversal_patterns {
            if pattern.is_match(value) {
                violations.push("Path traversal attempt detected".to_string());
                break;
            }
        }

        violations
    }

    /// Sanitize a string by removing/escaping dangerous content
    pub fn sanitize_string(&self, value: &str) -> String {
        let mut sanitized = value.to_string();

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Escape potentially dangerous characters
        sanitized = sanitized.replace('<', "&lt;");
        sanitized = sanitized.replace('>', "&gt;");
        sanitized = sanitized.replace('\"', "&quot;");
        sanitized = sanitized.replace('\'', "&#x27;");

        // Remove control characters (except \t, \n, \r)
        sanitized = sanitized
            .chars()
            .filter(|&c| !c.is_control() || c == '\t' || c == '\n' || c == '\r')
            .collect();

        sanitized
    }
}

impl Default for InputSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Main request security validator
pub struct RequestSecurityValidator {
    config: RequestSecurityConfig,
    sanitizer: InputSanitizer,
    violation_log: std::sync::Arc<std::sync::Mutex<Vec<SecurityViolation>>>,
}

impl RequestSecurityValidator {
    /// Create a new request security validator
    pub fn new(config: RequestSecurityConfig) -> Self {
        Self {
            config,
            sanitizer: InputSanitizer::new(),
            violation_log: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(RequestSecurityConfig::default())
    }

    /// Validate an MCP request for security issues
    pub async fn validate_request(
        &self,
        request: &Request,
        auth_context: Option<&AuthContext>,
    ) -> Result<(), SecurityValidationError> {
        if !self.config.enabled {
            return Ok(());
        }

        debug!("Validating request security for method: {}", request.method);

        // Apply user-specific security rules based on authentication context
        if let Some(context) = auth_context {
            self.validate_user_specific_rules(request, context)?;
        }

        // Validate method
        self.validate_method(&request.method)?;

        // Validate request size
        let request_size = serde_json::to_string(request)
            .map_err(|_| SecurityValidationError::MaliciousContent {
                reason: "Request serialization failed".to_string(),
            })?
            .len();

        if request_size > self.config.limits.max_request_size {
            self.log_violation(SecurityViolation {
                violation_type: SecurityViolationType::SizeLimit,
                severity: SecuritySeverity::High,
                description: format!(
                    "Request size {} exceeds limit {}",
                    request_size, self.config.limits.max_request_size
                ),
                field: None,
                value: None,
                timestamp: chrono::Utc::now(),
            });

            return Err(SecurityValidationError::RequestTooLarge {
                current: request_size,
                limit: self.config.limits.max_request_size,
            });
        }

        // Validate parameters
        self.validate_parameters(&request.params, "params")?;

        // Check for injection attempts
        if self.config.enable_injection_detection {
            self.detect_injection_attempts(&request.params, "params")?;
        }

        debug!("Request passed security validation");
        Ok(())
    }

    /// Sanitize an MCP request
    pub async fn sanitize_request(&self, mut request: Request) -> Request {
        if !self.config.enabled || !self.config.enable_sanitization {
            return request;
        }

        debug!("Sanitizing request parameters");
        request.params = self.sanitize_value(&request.params);
        request
    }

    /// Validate method name
    fn validate_method(&self, method: &str) -> Result<(), SecurityValidationError> {
        // Check blocked methods
        if self.config.blocked_methods.contains(method) {
            return Err(SecurityValidationError::UnsupportedMethod {
                method: method.to_string(),
            });
        }

        // Check allowed methods (if specified)
        if !self.config.allowed_methods.is_empty() && !self.config.allowed_methods.contains(method)
        {
            return Err(SecurityValidationError::UnsupportedMethod {
                method: method.to_string(),
            });
        }

        Ok(())
    }

    /// Validate parameters recursively
    fn validate_parameters(
        &self,
        value: &Value,
        path: &str,
    ) -> Result<(), SecurityValidationError> {
        self.validate_value_size(value, path)?;

        match value {
            Value::Object(obj) => {
                if obj.len() > self.config.limits.max_object_keys {
                    return Err(SecurityValidationError::TooManyParameters {
                        current: obj.len(),
                        limit: self.config.limits.max_object_keys,
                    });
                }

                for (key, val) in obj {
                    let new_path = format!("{}.{}", path, key);
                    self.validate_parameters(val, &new_path)?;
                }
            }
            Value::Array(arr) => {
                if arr.len() > self.config.limits.max_array_length {
                    return Err(SecurityValidationError::TooManyParameters {
                        current: arr.len(),
                        limit: self.config.limits.max_array_length,
                    });
                }

                for (i, val) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, i);
                    self.validate_parameters(val, &new_path)?;
                }
            }
            Value::String(s) => {
                if s.len() > self.config.limits.max_string_length {
                    return Err(SecurityValidationError::ParameterTooLarge {
                        param: path.to_string(),
                        current: s.len(),
                        limit: self.config.limits.max_string_length,
                    });
                }
            }
            _ => {} // Other types are fine
        }

        Ok(())
    }

    /// Validate the size of a value
    fn validate_value_size(
        &self,
        value: &Value,
        path: &str,
    ) -> Result<(), SecurityValidationError> {
        let size = serde_json::to_string(value)
            .map_err(|_| SecurityValidationError::MaliciousContent {
                reason: "Parameter serialization failed".to_string(),
            })?
            .len();

        if size > self.config.limits.max_parameter_size {
            return Err(SecurityValidationError::ParameterTooLarge {
                param: path.to_string(),
                current: size,
                limit: self.config.limits.max_parameter_size,
            });
        }

        Ok(())
    }

    /// Detect injection attempts in parameters
    fn detect_injection_attempts(
        &self,
        value: &Value,
        path: &str,
    ) -> Result<(), SecurityValidationError> {
        match value {
            Value::String(s) => {
                let violations = self.sanitizer.detect_injection(s);
                if !violations.is_empty() {
                    self.log_violation(SecurityViolation {
                        violation_type: SecurityViolationType::InjectionAttempt,
                        severity: SecuritySeverity::Critical,
                        description: violations.join(", "),
                        field: Some(path.to_string()),
                        value: Some(s.clone()),
                        timestamp: chrono::Utc::now(),
                    });

                    return Err(SecurityValidationError::InjectionDetected {
                        param: path.to_string(),
                    });
                }
            }
            Value::Object(obj) => {
                for (key, val) in obj {
                    let new_path = format!("{}.{}", path, key);
                    self.detect_injection_attempts(val, &new_path)?;
                }
            }
            Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, i);
                    self.detect_injection_attempts(val, &new_path)?;
                }
            }
            _ => {} // Other types are safe
        }

        Ok(())
    }

    /// Sanitize a JSON value recursively
    fn sanitize_value(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.sanitizer.sanitize_string(s)),
            Value::Object(obj) => {
                let sanitized_obj: serde_json::Map<String, Value> = obj
                    .iter()
                    .map(|(k, v)| (k.clone(), self.sanitize_value(v)))
                    .collect();
                Value::Object(sanitized_obj)
            }
            Value::Array(arr) => {
                let sanitized_arr: Vec<Value> =
                    arr.iter().map(|v| self.sanitize_value(v)).collect();
                Value::Array(sanitized_arr)
            }
            _ => value.clone(), // Numbers, bools, null are safe
        }
    }

    /// Log a security violation
    fn log_violation(&self, violation: SecurityViolation) {
        if self.config.log_violations {
            match violation.severity {
                SecuritySeverity::Critical => {
                    error!("Critical security violation: {}", violation.description)
                }
                SecuritySeverity::High => {
                    warn!("High security violation: {}", violation.description)
                }
                SecuritySeverity::Medium => {
                    warn!("Medium security violation: {}", violation.description)
                }
                SecuritySeverity::Low => {
                    debug!("Low security violation: {}", violation.description)
                }
            }
        }

        if let Ok(mut log) = self.violation_log.lock() {
            log.push(violation);

            // Keep only last 1000 violations to prevent memory bloat
            if log.len() > 1000 {
                log.drain(0..100);
            }
        }
    }

    /// Get recent security violations
    pub fn get_violations(&self) -> Vec<SecurityViolation> {
        self.violation_log
            .lock()
            .map(|log| log.clone())
            .unwrap_or_default()
    }

    /// Clear violation log
    pub fn clear_violations(&self) {
        if let Ok(mut log) = self.violation_log.lock() {
            log.clear();
        }
    }

    /// Validate user-specific security rules based on authentication context
    fn validate_user_specific_rules(
        &self,
        request: &Request,
        auth_context: &AuthContext,
    ) -> Result<(), SecurityValidationError> {
        // Apply stricter limits for lower-privilege users
        let user_limits = self.get_user_specific_limits(auth_context);

        // Validate request size against user-specific limits
        let request_size = serde_json::to_string(request)
            .map_err(|_| SecurityValidationError::MaliciousContent {
                reason: "Request serialization failed for user validation".to_string(),
            })?
            .len();

        if request_size > user_limits.max_request_size {
            self.log_violation(SecurityViolation {
                violation_type: SecurityViolationType::SizeLimit,
                severity: SecuritySeverity::High,
                description: format!(
                    "User {} exceeded request size limit: {} > {}",
                    auth_context.user_id.as_deref().unwrap_or("unknown"),
                    request_size,
                    user_limits.max_request_size
                ),
                field: None,
                value: None,
                timestamp: chrono::Utc::now(),
            });

            return Err(SecurityValidationError::RequestTooLarge {
                current: request_size,
                limit: user_limits.max_request_size,
            });
        }

        // Apply method-specific restrictions based on user role
        if let Some(restricted_methods) = self.get_restricted_methods_for_user(auth_context) {
            if restricted_methods.contains(&request.method) {
                self.log_violation(SecurityViolation {
                    violation_type: SecurityViolationType::UnauthorizedMethod,
                    severity: SecuritySeverity::Critical,
                    description: format!(
                        "User {} attempted to access restricted method: {}",
                        auth_context.user_id.as_deref().unwrap_or("unknown"),
                        request.method
                    ),
                    field: Some("method".to_string()),
                    value: Some(request.method.clone()),
                    timestamp: chrono::Utc::now(),
                });

                return Err(SecurityValidationError::UnsupportedMethod {
                    method: request.method.clone(),
                });
            }
        }

        // Apply enhanced injection detection for anonymous users
        if auth_context.user_id.is_none() {
            // Anonymous users get stricter validation
            self.validate_anonymous_user_request(request)?;
        }

        Ok(())
    }

    /// Get user-specific request limits based on role and permissions
    fn get_user_specific_limits(&self, auth_context: &AuthContext) -> RequestLimitsConfig {
        use crate::models::Role;

        // Default to the configured limits
        let mut limits = self.config.limits.clone();

        // Apply role-based limits
        let has_admin_role = auth_context
            .roles
            .iter()
            .any(|role| matches!(role, Role::Admin));
        let has_operator_role = auth_context
            .roles
            .iter()
            .any(|role| matches!(role, Role::Operator));
        let has_device_role = auth_context
            .roles
            .iter()
            .any(|role| matches!(role, Role::Device { .. }));

        if has_device_role && !has_admin_role {
            // Devices get smaller limits to prevent resource exhaustion
            limits.max_request_size = std::cmp::min(limits.max_request_size, 64 * 1024); // 64KB max
            limits.max_parameter_size = std::cmp::min(limits.max_parameter_size, 8 * 1024); // 8KB max
            limits.max_string_length = std::cmp::min(limits.max_string_length, 1000);
            limits.max_array_length = std::cmp::min(limits.max_array_length, 50);
            limits.max_object_keys = std::cmp::min(limits.max_object_keys, 20);
        } else if !has_admin_role && !has_operator_role {
            // Regular users get moderate limits
            limits.max_request_size = std::cmp::min(limits.max_request_size, 256 * 1024); // 256KB max
            limits.max_parameter_size = std::cmp::min(limits.max_parameter_size, 32 * 1024); // 32KB max
            limits.max_string_length = std::cmp::min(limits.max_string_length, 5000);
            limits.max_array_length = std::cmp::min(limits.max_array_length, 200);
            limits.max_object_keys = std::cmp::min(limits.max_object_keys, 50);
        }
        // Admins and operators get full configured limits

        limits
    }

    /// Get restricted methods for specific user based on role and permissions
    fn get_restricted_methods_for_user(
        &self,
        auth_context: &AuthContext,
    ) -> Option<HashSet<String>> {
        use crate::models::Role;

        let has_admin_role = auth_context
            .roles
            .iter()
            .any(|role| matches!(role, Role::Admin));

        // Admins have no method restrictions
        if has_admin_role {
            return None;
        }

        let mut restricted = HashSet::new();

        // Device role restrictions
        let has_device_role = auth_context
            .roles
            .iter()
            .any(|role| matches!(role, Role::Device { .. }));
        if has_device_role {
            // Devices cannot access administrative methods
            restricted.insert("logging/setLevel".to_string());
            restricted.insert("server/shutdown".to_string());
            restricted.insert("auth/createKey".to_string());
            restricted.insert("auth/revokeKey".to_string());
        }

        // Monitor role restrictions
        let has_monitor_role = auth_context
            .roles
            .iter()
            .any(|role| matches!(role, Role::Monitor));
        if has_monitor_role
            && !auth_context
                .roles
                .iter()
                .any(|role| matches!(role, Role::Operator))
        {
            // Monitor-only users cannot access state-changing methods
            restricted.insert("tools/call".to_string());
            restricted.insert("resources/write".to_string());
        }

        if restricted.is_empty() {
            None
        } else {
            Some(restricted)
        }
    }

    /// Apply enhanced validation for anonymous users
    fn validate_anonymous_user_request(
        &self,
        request: &Request,
    ) -> Result<(), SecurityValidationError> {
        // Check method parameters more strictly
        self.detect_injection_attempts_strict(&request.params, "params")?;

        // Anonymous users are limited to read-only operations
        let read_only_methods = [
            "ping",
            "initialize",
            "resources/list",
            "resources/read",
            "tools/list",
            "completion/complete",
        ];

        if !read_only_methods.contains(&request.method.as_str()) {
            self.log_violation(SecurityViolation {
                violation_type: SecurityViolationType::UnauthorizedMethod,
                severity: SecuritySeverity::High,
                description: format!(
                    "Anonymous user attempted non-read-only method: {}",
                    request.method
                ),
                field: Some("method".to_string()),
                value: Some(request.method.clone()),
                timestamp: chrono::Utc::now(),
            });

            return Err(SecurityValidationError::UnsupportedMethod {
                method: request.method.clone(),
            });
        }

        Ok(())
    }

    /// Enhanced injection detection with stricter rules
    fn detect_injection_attempts_strict(
        &self,
        value: &Value,
        path: &str,
    ) -> Result<(), SecurityValidationError> {
        match value {
            Value::String(s) => {
                // More aggressive injection detection for anonymous users
                let violations = self.sanitizer.detect_injection(s);

                // Additional checks for anonymous users
                let suspicious_patterns = [
                    "eval", "exec", "system", "cmd", "shell", "script", "import", "require",
                    "include", "load",
                ];

                let lower_s = s.to_lowercase();
                for pattern in &suspicious_patterns {
                    if lower_s.contains(pattern) {
                        self.log_violation(SecurityViolation {
                            violation_type: SecurityViolationType::InjectionAttempt,
                            severity: SecuritySeverity::Critical,
                            description: format!(
                                "Suspicious pattern '{}' detected in anonymous user request",
                                pattern
                            ),
                            field: Some(path.to_string()),
                            value: Some(s.clone()),
                            timestamp: chrono::Utc::now(),
                        });

                        return Err(SecurityValidationError::InjectionDetected {
                            param: path.to_string(),
                        });
                    }
                }

                if !violations.is_empty() {
                    self.log_violation(SecurityViolation {
                        violation_type: SecurityViolationType::InjectionAttempt,
                        severity: SecuritySeverity::Critical,
                        description: format!(
                            "Enhanced injection detection: {}",
                            violations.join(", ")
                        ),
                        field: Some(path.to_string()),
                        value: Some(s.clone()),
                        timestamp: chrono::Utc::now(),
                    });

                    return Err(SecurityValidationError::InjectionDetected {
                        param: path.to_string(),
                    });
                }
            }
            Value::Object(obj) => {
                for (key, val) in obj {
                    let new_path = format!("{}.{}", path, key);
                    self.detect_injection_attempts_strict(val, &new_path)?;
                }
            }
            Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, i);
                    self.detect_injection_attempts_strict(val, &new_path)?;
                }
            }
            _ => {} // Other types are safe
        }

        Ok(())
    }
}

/// Helper for creating security configurations
impl RequestSecurityConfig {
    /// Create a permissive configuration (minimal validation)
    pub fn permissive() -> Self {
        Self {
            enabled: true,
            limits: RequestLimitsConfig {
                max_request_size: 100 * 1024 * 1024, // 100MB
                max_parameters: 1000,
                max_parameter_size: 10 * 1024 * 1024, // 10MB
                max_string_length: 100000,
                max_array_length: 10000,
                max_object_depth: 20,
                max_object_keys: 1000,
            },
            enable_injection_detection: false,
            enable_sanitization: false,
            allowed_methods: HashSet::new(),
            blocked_methods: HashSet::new(),
            enable_method_rate_limiting: false,
            method_rate_limits: HashMap::new(),
            log_violations: true,
            fail_on_violations: false,
        }
    }

    /// Create a strict configuration (maximum security)
    pub fn strict() -> Self {
        let mut blocked_methods = HashSet::new();
        blocked_methods.insert("logging/setLevel".to_string()); // Admin only

        Self {
            enabled: true,
            limits: RequestLimitsConfig {
                max_request_size: 1024 * 1024, // 1MB
                max_parameters: 50,
                max_parameter_size: 100 * 1024, // 100KB
                max_string_length: 1000,
                max_array_length: 100,
                max_object_depth: 5,
                max_object_keys: 20,
            },
            enable_injection_detection: true,
            enable_sanitization: true,
            allowed_methods: HashSet::new(),
            blocked_methods,
            enable_method_rate_limiting: true,
            method_rate_limits: {
                let mut limits = HashMap::new();
                limits.insert("tools/call".to_string(), 30); // 0.5 per second
                limits.insert("resources/read".to_string(), 60); // 1 per second
                limits
            },
            log_violations: true,
            fail_on_violations: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_input_sanitizer_sql_injection() {
        let sanitizer = InputSanitizer::new();

        let malicious_input = "'; DROP TABLE users; --";
        let violations = sanitizer.detect_injection(malicious_input);
        assert!(!violations.is_empty());
        assert!(violations[0].contains("SQL injection"));
    }

    #[test]
    fn test_input_sanitizer_xss() {
        let sanitizer = InputSanitizer::new();

        let malicious_input = "<script>alert('xss')</script>";
        let violations = sanitizer.detect_injection(malicious_input);
        assert!(!violations.is_empty());
        assert!(violations[0].contains("XSS"));
    }

    #[test]
    fn test_input_sanitizer_command_injection() {
        let sanitizer = InputSanitizer::new();

        let malicious_input = "; cat /etc/passwd";
        let violations = sanitizer.detect_injection(malicious_input);
        assert!(!violations.is_empty());
        assert!(violations[0].contains("Command injection"));
    }

    #[test]
    fn test_string_sanitization() {
        let sanitizer = InputSanitizer::new();

        let dirty_string = "<script>alert('test')</script>";
        let clean_string = sanitizer.sanitize_string(dirty_string);
        assert_eq!(
            clean_string,
            "&lt;script&gt;alert(&#x27;test&#x27;)&lt;/script&gt;"
        );
    }

    #[tokio::test]
    async fn test_request_size_validation() {
        let config = RequestSecurityConfig {
            limits: RequestLimitsConfig {
                max_request_size: 100, // Very small limit
                ..Default::default()
            },
            ..Default::default()
        };

        let validator = RequestSecurityValidator::new(config);

        let large_request = Request {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: json!({
                "large_param": "a".repeat(1000)
            }),
            id: serde_json::Value::Number(1.into()),
        };

        let result = validator.validate_request(&large_request, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityValidationError::RequestTooLarge { .. }
        ));
    }

    #[tokio::test]
    async fn test_parameter_injection_detection() {
        let validator = RequestSecurityValidator::default();

        let malicious_request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: json!({
                "name": "test_tool",
                "arguments": {
                    "query": "'; DROP TABLE users; --"
                }
            }),
            id: serde_json::Value::Number(1.into()),
        };

        let result = validator.validate_request(&malicious_request, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityValidationError::InjectionDetected { .. }
        ));
    }

    #[tokio::test]
    async fn test_method_blocking() {
        let config = RequestSecurityConfig {
            blocked_methods: {
                let mut set = HashSet::new();
                set.insert("dangerous_method".to_string());
                set
            },
            ..Default::default()
        };

        let validator = RequestSecurityValidator::new(config);

        let blocked_request = Request {
            jsonrpc: "2.0".to_string(),
            method: "dangerous_method".to_string(),
            params: json!({}),
            id: serde_json::Value::Number(1.into()),
        };

        let result = validator.validate_request(&blocked_request, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityValidationError::UnsupportedMethod { .. }
        ));
    }

    #[tokio::test]
    async fn test_request_sanitization() {
        let validator = RequestSecurityValidator::default();

        let dirty_request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: json!({
                "name": "test_tool",
                "arguments": {
                    "message": "<script>alert('test')</script>"
                }
            }),
            id: serde_json::Value::Number(1.into()),
        };

        let clean_request = validator.sanitize_request(dirty_request).await;
        let clean_message = clean_request.params["arguments"]["message"]
            .as_str()
            .unwrap();
        assert!(!clean_message.contains("<script>"));
        assert!(clean_message.contains("&lt;script&gt;"));
    }
}
