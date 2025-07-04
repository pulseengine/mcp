//! Fuzzing tests for MCP protocol compliance
//!
//! This module provides fuzzing capabilities to test protocol robustness
//! against malformed, unexpected, or malicious inputs.

use crate::{
    report::{IssueSeverity, ValidationIssue},
    ValidationConfig, ValidationError, ValidationResult,
};
use arbitrary::{Arbitrary, Unstructured};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Fuzzing test runner for MCP protocol
pub struct McpFuzzer {
    config: ValidationConfig,
    seed: Option<u64>,
    max_iterations: usize,
}

/// Fuzzing target types
#[derive(Debug, Clone, Copy)]
pub enum FuzzTarget {
    /// Fuzz JSON-RPC message structure
    JsonRpcStructure,

    /// Fuzz MCP method names
    MethodNames,

    /// Fuzz parameter values
    ParameterValues,

    /// Fuzz protocol version strings
    ProtocolVersions,

    /// Fuzz transport layer
    TransportLayer,

    /// Fuzz authentication tokens
    Authentication,

    /// Fuzz resource URIs
    ResourceUris,

    /// Fuzz tool arguments
    ToolArguments,
}

/// Fuzzing result for a single test
#[derive(Debug)]
pub struct FuzzResult {
    /// Target that was fuzzed
    pub target: FuzzTarget,

    /// Number of iterations performed
    pub iterations: usize,

    /// Number of crashes detected
    pub crashes: usize,

    /// Number of hangs detected
    pub hangs: usize,

    /// Number of invalid responses
    pub invalid_responses: usize,

    /// Unique issues found
    pub issues: Vec<FuzzIssue>,

    /// Total fuzzing duration
    pub duration: Duration,
}

/// Individual fuzzing issue
#[derive(Debug, Clone)]
pub struct FuzzIssue {
    /// Issue type
    pub issue_type: FuzzIssueType,

    /// Input that caused the issue
    pub input: Vec<u8>,

    /// Input as string (if valid UTF-8)
    pub input_string: Option<String>,

    /// Response received (if any)
    pub response: Option<String>,

    /// Error message
    pub error: String,

    /// Reproducible
    pub reproducible: bool,
}

/// Types of issues found during fuzzing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzIssueType {
    /// Server crashed
    Crash,

    /// Server hung/timeout
    Hang,

    /// Invalid JSON response
    InvalidJson,

    /// Protocol violation
    ProtocolViolation,

    /// Security issue
    SecurityIssue,

    /// Memory exhaustion
    MemoryExhaustion,

    /// Unexpected behavior
    UnexpectedBehavior,
}

/// Arbitrary JSON-RPC message for fuzzing
#[derive(Debug, Clone)]
struct FuzzJsonRpcMessage {
    jsonrpc: FuzzJsonRpcVersion,
    method: Option<FuzzMethod>,
    params: Option<FuzzParams>,
    result: Option<String>,
    error: Option<FuzzError>,
    id: Option<FuzzId>,
    extra_fields: Vec<(String, String)>,
}

/// Fuzzing JSON-RPC versions
#[derive(Debug, Clone, Arbitrary)]
enum FuzzJsonRpcVersion {
    Valid,
    InvalidString(String),
    Number(f64),
    Object(Vec<(String, String)>),
    Array(Vec<String>),
    Null,
}

/// Fuzzing method names
#[derive(Debug, Clone, Arbitrary)]
enum FuzzMethod {
    ValidMcp(String),
    InvalidChars(Vec<u8>),
    ExtremelyLong(String),
    EmptyString,
    Whitespace(String),
    ControlChars(String),
    Unicode(String),
    SqlInjection(String),
    PathTraversal(String),
}

/// Fuzzing parameters
#[derive(Debug, Clone, Arbitrary)]
enum FuzzParams {
    Object(Vec<(String, String)>),
    Array(Vec<String>),
    String(String),
    Number(f64),
    DeepNesting(Box<FuzzParams>),
    Circular,
    Null,
}

/// Fuzzing IDs
#[derive(Debug, Clone, Arbitrary)]
enum FuzzId {
    String(String),
    Number(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<String>),
    Object(Vec<(String, String)>),
    Null,
}

/// Fuzzing error objects
#[derive(Debug, Clone, Arbitrary)]
struct FuzzError {
    code: FuzzErrorCode,
    message: String,
    data: Option<String>,
}

/// Fuzzing error codes
#[derive(Debug, Clone, Arbitrary)]
enum FuzzErrorCode {
    Valid(i32),
    OutOfRange(i64),
    Float(f64),
    String(String),
    Null,
}

impl<'a> Arbitrary<'a> for FuzzJsonRpcMessage {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(FuzzJsonRpcMessage {
            jsonrpc: u.arbitrary()?,
            method: u.arbitrary()?,
            params: u.arbitrary()?,
            result: u.arbitrary()?,
            error: u.arbitrary()?,
            id: u.arbitrary()?,
            extra_fields: u.arbitrary()?,
        })
    }
}

impl McpFuzzer {
    /// Create a new fuzzer with configuration
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            seed: None,
            max_iterations: 10000,
        }
    }

    /// Set random seed for reproducible fuzzing
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Run fuzzing against a server
    pub async fn fuzz_server(
        &self,
        server_url: &str,
        target: FuzzTarget,
    ) -> ValidationResult<FuzzResult> {
        info!("Starting fuzzing for {:?} against {}", target, server_url);

        let start_time = Instant::now();
        let mut result = FuzzResult {
            target,
            iterations: 0,
            crashes: 0,
            hangs: 0,
            invalid_responses: 0,
            issues: Vec::new(),
            duration: Duration::from_secs(0),
        };

        // Create HTTP client with short timeout for hang detection
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        // Initialize random data source
        let mut rng = if let Some(seed) = self.seed {
            fastrand::Rng::with_seed(seed)
        } else {
            fastrand::Rng::new()
        };

        // Run fuzzing iterations
        for i in 0..self.max_iterations {
            result.iterations = i + 1;

            // Generate fuzzed input based on target
            let fuzzed_input = self.generate_fuzzed_input(&mut rng, target)?;

            // Send fuzzed input to server
            match self
                .send_fuzzed_input(&client, server_url, &fuzzed_input)
                .await
            {
                Ok(response) => {
                    // Check response validity
                    if let Err(e) = self.validate_response(&response) {
                        result.invalid_responses += 1;

                        let issue = FuzzIssue {
                            issue_type: FuzzIssueType::InvalidJson,
                            input: fuzzed_input.as_bytes().to_vec(),
                            input_string: Some(fuzzed_input.clone()),
                            response: Some(response),
                            error: e.to_string(),
                            reproducible: false,
                        };

                        if !self.is_duplicate_issue(&result.issues, &issue) {
                            result.issues.push(issue);
                        }
                    }
                }
                Err(e) => {
                    // Classify error
                    let issue_type = self.classify_error(&e);

                    match issue_type {
                        FuzzIssueType::Crash => result.crashes += 1,
                        FuzzIssueType::Hang => result.hangs += 1,
                        _ => {}
                    }

                    let issue = FuzzIssue {
                        issue_type,
                        input: fuzzed_input.as_bytes().to_vec(),
                        input_string: Some(fuzzed_input),
                        response: None,
                        error: e.to_string(),
                        reproducible: false,
                    };

                    if !self.is_duplicate_issue(&result.issues, &issue) {
                        // Test reproducibility
                        let mut issue = issue;
                        issue.reproducible = self
                            .test_reproducibility(
                                &client,
                                server_url,
                                &String::from_utf8_lossy(&issue.input),
                            )
                            .await;

                        result.issues.push(issue);
                    }
                }
            }

            // Progress reporting
            if i % 1000 == 0 && i > 0 {
                debug!("Fuzzing progress: {}/{} iterations", i, self.max_iterations);
            }
        }

        result.duration = start_time.elapsed();

        info!(
            "Fuzzing completed: {} iterations, {} crashes, {} hangs, {} issues found",
            result.iterations,
            result.crashes,
            result.hangs,
            result.issues.len()
        );

        Ok(result)
    }

    /// Generate fuzzed input based on target
    fn generate_fuzzed_input(
        &self,
        rng: &mut fastrand::Rng,
        target: FuzzTarget,
    ) -> ValidationResult<String> {
        let mut data = vec![0u8; rng.usize(1..=10000)];
        rng.fill(&mut data);

        let mut u = Unstructured::new(&data);

        let json_value = match target {
            FuzzTarget::JsonRpcStructure => {
                let msg = FuzzJsonRpcMessage::arbitrary(&mut u).map_err(|e| {
                    ValidationError::ConfigurationError {
                        message: format!("Failed to generate fuzz message: {}", e),
                    }
                })?;

                self.fuzz_message_to_json(&msg)
            }

            FuzzTarget::MethodNames => {
                let method = FuzzMethod::arbitrary(&mut u).map_err(|e| {
                    ValidationError::ConfigurationError {
                        message: format!("Failed to generate fuzz method: {}", e),
                    }
                })?;

                json!({
                    "jsonrpc": "2.0",
                    "method": self.fuzz_method_to_string(&method),
                    "params": {},
                    "id": rng.u64(..)
                })
            }

            FuzzTarget::ParameterValues => {
                let params = FuzzParams::arbitrary(&mut u).map_err(|e| {
                    ValidationError::ConfigurationError {
                        message: format!("Failed to generate fuzz params: {}", e),
                    }
                })?;

                json!({
                    "jsonrpc": "2.0",
                    "method": "tools/list",
                    "params": self.fuzz_params_to_value(&params),
                    "id": 1
                })
            }

            FuzzTarget::ProtocolVersions => {
                let version = self.generate_fuzz_version(rng);

                json!({
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": version,
                        "capabilities": {},
                        "clientInfo": {
                            "name": "fuzzer",
                            "version": "1.0"
                        }
                    },
                    "id": 1
                })
            }

            FuzzTarget::ResourceUris => {
                let uri = self.generate_fuzz_uri(rng);

                json!({
                    "jsonrpc": "2.0",
                    "method": "resources/read",
                    "params": {
                        "uri": uri
                    },
                    "id": 1
                })
            }

            FuzzTarget::ToolArguments => {
                let args = self.generate_fuzz_tool_args(&mut u)?;

                json!({
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "test-tool",
                        "arguments": args
                    },
                    "id": 1
                })
            }

            FuzzTarget::Authentication => {
                let auth = self.generate_fuzz_auth(rng);

                json!({
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "authentication": auth
                        },
                        "clientInfo": {
                            "name": "fuzzer",
                            "version": "1.0"
                        }
                    },
                    "id": 1
                })
            }

            FuzzTarget::TransportLayer => {
                // Generate malformed HTTP headers or WebSocket frames
                // This would require lower-level access
                json!({
                    "jsonrpc": "2.0",
                    "method": "test",
                    "id": 1
                })
            }
        };

        // Sometimes generate completely invalid JSON
        if rng.bool() && rng.f32() < 0.1 {
            let json_str = serde_json::to_string(&json_value)?;
            Ok(self.corrupt_json(&json_str, rng))
        } else {
            serde_json::to_string(&json_value).map_err(|e| ValidationError::InvalidResponseFormat {
                details: format!("Failed to serialize fuzz input: {}", e),
            })
        }
    }

    /// Convert fuzz message to JSON
    fn fuzz_message_to_json(&self, msg: &FuzzJsonRpcMessage) -> Value {
        let mut obj = serde_json::Map::new();

        // Add jsonrpc field
        obj.insert(
            "jsonrpc".to_string(),
            match &msg.jsonrpc {
                FuzzJsonRpcVersion::Valid => json!("2.0"),
                FuzzJsonRpcVersion::InvalidString(s) => json!(s),
                FuzzJsonRpcVersion::Number(n) => json!(n),
                FuzzJsonRpcVersion::Object(fields) => {
                    let mut o = serde_json::Map::new();
                    for (k, v) in fields {
                        o.insert(k.clone(), json!(v));
                    }
                    Value::Object(o)
                }
                FuzzJsonRpcVersion::Array(arr) => {
                    Value::Array(arr.iter().map(|s| json!(s)).collect())
                }
                FuzzJsonRpcVersion::Null => Value::Null,
            },
        );

        // Add optional fields
        if let Some(method) = &msg.method {
            obj.insert(
                "method".to_string(),
                json!(self.fuzz_method_to_string(method)),
            );
        }

        if let Some(params) = &msg.params {
            obj.insert("params".to_string(), self.fuzz_params_to_value(params));
        }

        if let Some(result) = &msg.result {
            obj.insert("result".to_string(), json!(result));
        }

        if let Some(error) = &msg.error {
            obj.insert("error".to_string(), self.fuzz_error_to_json(error));
        }

        if let Some(id) = &msg.id {
            obj.insert("id".to_string(), self.fuzz_id_to_json(id));
        }

        // Add extra fields
        for (key, value) in &msg.extra_fields {
            obj.insert(key.clone(), json!(value));
        }

        Value::Object(obj)
    }

    /// Convert fuzz method to string
    fn fuzz_method_to_string(&self, method: &FuzzMethod) -> String {
        match method {
            FuzzMethod::ValidMcp(s) => s.clone(),
            FuzzMethod::InvalidChars(bytes) => String::from_utf8_lossy(bytes).to_string(),
            FuzzMethod::ExtremelyLong(s) => s.repeat(1000),
            FuzzMethod::EmptyString => String::new(),
            FuzzMethod::Whitespace(s) => format!("  {}  ", s),
            FuzzMethod::ControlChars(s) => format!("{}\0\r\n\t", s),
            FuzzMethod::Unicode(s) => format!("{}🔥💥🚀", s),
            FuzzMethod::SqlInjection(s) => format!("{}'; DROP TABLE tools; --", s),
            FuzzMethod::PathTraversal(s) => format!("{}/../../../etc/passwd", s),
        }
    }

    /// Convert fuzz params to value
    fn fuzz_params_to_value(&self, params: &FuzzParams) -> Value {
        match params {
            FuzzParams::Object(fields) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in fields {
                    obj.insert(k.clone(), json!(v));
                }
                Value::Object(obj)
            }
            FuzzParams::Array(arr) => Value::Array(arr.iter().map(|s| json!(s)).collect()),
            FuzzParams::String(s) => json!(s),
            FuzzParams::Number(n) => json!(n),
            FuzzParams::DeepNesting(inner) => {
                json!({
                    "nested": self.fuzz_params_to_value(inner)
                })
            }
            FuzzParams::Circular => {
                // Can't actually create circular JSON
                json!({"circular": "reference"})
            }
            FuzzParams::Null => Value::Null,
        }
    }

    /// Convert fuzz error to JSON
    fn fuzz_error_to_json(&self, error: &FuzzError) -> Value {
        let mut obj = serde_json::Map::new();

        obj.insert(
            "code".to_string(),
            match &error.code {
                FuzzErrorCode::Valid(n) => json!(n),
                FuzzErrorCode::OutOfRange(n) => json!(n),
                FuzzErrorCode::Float(n) => json!(n),
                FuzzErrorCode::String(s) => json!(s),
                FuzzErrorCode::Null => Value::Null,
            },
        );

        obj.insert("message".to_string(), json!(error.message));

        if let Some(data) = &error.data {
            obj.insert("data".to_string(), json!(data));
        }

        Value::Object(obj)
    }

    /// Convert fuzz ID to JSON
    fn fuzz_id_to_json(&self, id: &FuzzId) -> Value {
        match id {
            FuzzId::String(s) => json!(s),
            FuzzId::Number(n) => json!(n),
            FuzzId::Float(f) => json!(f),
            FuzzId::Bool(b) => json!(b),
            FuzzId::Array(arr) => Value::Array(arr.iter().map(|s| json!(s)).collect()),
            FuzzId::Object(fields) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in fields {
                    obj.insert(k.clone(), json!(v));
                }
                Value::Object(obj)
            }
            FuzzId::Null => Value::Null,
        }
    }

    /// Generate fuzz version string
    fn generate_fuzz_version(&self, rng: &mut fastrand::Rng) -> String {
        match rng.usize(0..10) {
            0 => "2024-11-05".to_string(),       // Valid
            1 => "2025-03-26".to_string(),       // Valid
            2 => "1.0".to_string(),              // Old format
            3 => "".to_string(),                 // Empty
            4 => "🔥".to_string(),               // Unicode
            5 => "9999-99-99".to_string(),       // Future
            6 => format!("{}", rng.u64(..)),     // Number
            7 => "null".to_string(),             // Null string
            8 => "\0\r\n".to_string(),           // Control chars
            _ => "x".repeat(rng.usize(1..1000)), // Long string
        }
    }

    /// Generate fuzz URI
    fn generate_fuzz_uri(&self, rng: &mut fastrand::Rng) -> String {
        match rng.usize(0..15) {
            0 => "file:///etc/passwd".to_string(),
            1 => "http://localhost/../../".to_string(),
            2 => "javascript:alert(1)".to_string(),
            3 => "data:text/html,<script>alert(1)</script>".to_string(),
            4 => "".to_string(),
            5 => "null".to_string(),
            6 => "\0".to_string(),
            7 => "x".repeat(10000),
            8 => format!("resource://{}", "a".repeat(1000)),
            9 => "resource://\n\rSet-Cookie: admin=true".to_string(),
            10 => "resource://;rm -rf /".to_string(),
            11 => "resource://<script>".to_string(),
            12 => format!(
                "resource://{}",
                std::char::from_u32(rng.u32(..) % 0x110000).unwrap_or('?')
            ),
            13 => "resource://127.0.0.1:22".to_string(),
            _ => format!("resource://test-{}", rng.u64(..)),
        }
    }

    /// Generate fuzz tool arguments
    fn generate_fuzz_tool_args(&self, u: &mut Unstructured) -> ValidationResult<Value> {
        // Generate deeply nested or large objects
        let depth = u.int_in_range(0..=10).unwrap_or(0);
        self.generate_nested_value(u, depth)
    }

    /// Generate nested value with specified depth
    fn generate_nested_value(&self, u: &mut Unstructured, depth: usize) -> ValidationResult<Value> {
        if depth == 0 {
            // Base case
            match u.int_in_range(0..=5).unwrap_or(0) {
                0 => Ok(Value::Null),
                1 => Ok(json!(u.arbitrary::<bool>().unwrap_or(false))),
                2 => Ok(json!(u.arbitrary::<f64>().unwrap_or(0.0))),
                3 => Ok(json!(u.arbitrary::<String>().unwrap_or_default())),
                4 => Ok(json!(u.arbitrary::<Vec<u8>>().unwrap_or_default())),
                _ => Ok(json!({})),
            }
        } else {
            // Recursive case
            match u.int_in_range(0..=1).unwrap_or(0) {
                0 => {
                    // Array
                    let len = u.int_in_range(0..=100).unwrap_or(0);
                    let mut arr = Vec::new();
                    for _ in 0..len {
                        arr.push(self.generate_nested_value(u, depth - 1)?);
                    }
                    Ok(Value::Array(arr))
                }
                _ => {
                    // Object
                    let len = u.int_in_range(0..=100).unwrap_or(0);
                    let mut obj = serde_json::Map::new();
                    for _ in 0..len {
                        let key = u.arbitrary::<String>().unwrap_or_else(|_| {
                            format!("key{}", u.arbitrary::<u32>().unwrap_or(0))
                        });
                        let value = self.generate_nested_value(u, depth - 1)?;
                        obj.insert(key, value);
                    }
                    Ok(Value::Object(obj))
                }
            }
        }
    }

    /// Generate fuzz authentication
    fn generate_fuzz_auth(&self, rng: &mut fastrand::Rng) -> Value {
        match rng.usize(0..10) {
            0 => json!({"type": "bearer", "token": "x".repeat(10000)}),
            1 => json!({"type": "unknown", "data": null}),
            2 => json!({"type": "oauth", "token": "../../etc/passwd"}),
            3 => json!({"type": "basic", "username": "\0admin\0", "password": "' OR '1'='1"}),
            4 => json!(null),
            5 => json!([]),
            6 => json!("not-an-object"),
            7 => json!({"type": "x".repeat(1000)}),
            8 => json!({"type": "bearer", "token": std::f64::INFINITY}),
            _ => json!({}),
        }
    }

    /// Corrupt valid JSON
    fn corrupt_json(&self, json: &str, rng: &mut fastrand::Rng) -> String {
        let mut chars: Vec<char> = json.chars().collect();
        let corruption_type = rng.usize(0..10);

        match corruption_type {
            0 => {
                // Remove random character
                if !chars.is_empty() {
                    chars.remove(rng.usize(0..chars.len()));
                }
            }
            1 => {
                // Add random character
                let pos = rng.usize(0..=chars.len());
                chars.insert(pos, rng.char(..));
            }
            2 => {
                // Swap two characters
                if chars.len() >= 2 {
                    let i = rng.usize(0..chars.len());
                    let j = rng.usize(0..chars.len());
                    chars.swap(i, j);
                }
            }
            3 => {
                // Remove closing braces
                chars.retain(|&c| c != '}' && c != ']');
            }
            4 => {
                // Double quotes
                for i in 0..chars.len() {
                    if chars[i] == '"' && rng.bool() {
                        chars[i] = '\'';
                    }
                }
            }
            5 => {
                // Insert null bytes
                let pos = rng.usize(0..=chars.len());
                chars.insert(pos, '\0');
            }
            6 => {
                // Truncate
                let new_len = rng.usize(0..chars.len());
                chars.truncate(new_len);
            }
            7 => {
                // Insert UTF-8 BOM
                chars.insert(0, '\u{FEFF}');
            }
            8 => {
                // Make extremely long
                let repeat = rng.usize(10..100);
                let mut new_chars = Vec::new();
                for _ in 0..repeat {
                    new_chars.extend_from_slice(&chars);
                }
                chars = new_chars;
            }
            _ => {
                // Random corruption
                for i in 0..chars.len() {
                    if rng.f32() < 0.05 {
                        chars[i] = rng.char(..);
                    }
                }
            }
        }

        chars.into_iter().collect()
    }

    /// Send fuzzed input to server
    async fn send_fuzzed_input(
        &self,
        client: &reqwest::Client,
        server_url: &str,
        input: &str,
    ) -> Result<String, reqwest::Error> {
        let response = client
            .post(server_url)
            .header("Content-Type", "application/json")
            .body(input.to_string())
            .send()
            .await?;

        response.text().await
    }

    /// Validate response format
    fn validate_response(&self, response: &str) -> ValidationResult<()> {
        // Try to parse as JSON
        let value: Value =
            serde_json::from_str(response).map_err(|e| ValidationError::InvalidResponseFormat {
                details: format!("Invalid JSON: {}", e),
            })?;

        // Check for required JSON-RPC fields
        if let Some(obj) = value.as_object() {
            if !obj.contains_key("jsonrpc") {
                return Err(ValidationError::InvalidResponseFormat {
                    details: "Missing jsonrpc field".to_string(),
                });
            }

            // Must have either result or error, not both
            let has_result = obj.contains_key("result");
            let has_error = obj.contains_key("error");

            if has_result && has_error {
                return Err(ValidationError::InvalidResponseFormat {
                    details: "Response has both result and error".to_string(),
                });
            }

            if !has_result && !has_error && obj.contains_key("id") {
                return Err(ValidationError::InvalidResponseFormat {
                    details: "Response has neither result nor error".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Classify error type
    fn classify_error(&self, error: &reqwest::Error) -> FuzzIssueType {
        if error.is_timeout() {
            FuzzIssueType::Hang
        } else if error.is_connect() {
            FuzzIssueType::Crash
        } else if error.is_decode() {
            FuzzIssueType::InvalidJson
        } else {
            FuzzIssueType::UnexpectedBehavior
        }
    }

    /// Check if issue is duplicate
    fn is_duplicate_issue(&self, existing: &[FuzzIssue], new: &FuzzIssue) -> bool {
        existing
            .iter()
            .any(|issue| issue.issue_type == new.issue_type && issue.error == new.error)
    }

    /// Test if issue is reproducible
    async fn test_reproducibility(
        &self,
        client: &reqwest::Client,
        server_url: &str,
        input: &str,
    ) -> bool {
        // Try to reproduce 3 times
        for _ in 0..3 {
            match self.send_fuzzed_input(client, server_url, input).await {
                Ok(_) => return false, // Didn't reproduce
                Err(_) => continue,    // Error reproduced
            }
        }
        true // Reproduced all 3 times
    }
}

/// Convert fuzzing results to validation issues
pub fn fuzz_results_to_issues(results: &[FuzzResult]) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for result in results {
        for fuzz_issue in &result.issues {
            let severity = match fuzz_issue.issue_type {
                FuzzIssueType::Crash => IssueSeverity::Critical,
                FuzzIssueType::SecurityIssue => IssueSeverity::Critical,
                FuzzIssueType::MemoryExhaustion => IssueSeverity::Error,
                FuzzIssueType::Hang => IssueSeverity::Error,
                FuzzIssueType::ProtocolViolation => IssueSeverity::Error,
                FuzzIssueType::InvalidJson => IssueSeverity::Warning,
                FuzzIssueType::UnexpectedBehavior => IssueSeverity::Warning,
            };

            let mut issue = ValidationIssue::new(
                severity,
                format!("fuzzing-{:?}", result.target),
                format!("{:?}: {}", fuzz_issue.issue_type, fuzz_issue.error),
                "fuzzer".to_string(),
            );

            if let Some(input_str) = &fuzz_issue.input_string {
                issue = issue.with_detail("input".to_string(), json!(input_str));
            }

            if fuzz_issue.reproducible {
                issue = issue.with_detail("reproducible".to_string(), json!(true));
            }

            issues.push(issue);
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzer_creation() {
        let config = ValidationConfig::default();
        let fuzzer = McpFuzzer::new(config)
            .with_seed(12345)
            .with_max_iterations(1000);

        assert_eq!(fuzzer.seed, Some(12345));
        assert_eq!(fuzzer.max_iterations, 1000);
    }

    #[test]
    fn test_json_corruption() {
        let config = ValidationConfig::default();
        let fuzzer = McpFuzzer::new(config);
        let mut rng = fastrand::Rng::with_seed(42);

        let valid_json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let corrupted = fuzzer.corrupt_json(valid_json, &mut rng);

        // Should be different from original
        assert_ne!(valid_json, corrupted);
    }

    #[test]
    fn test_fuzz_results_conversion() {
        // Test the conversion function for fuzz results
        let results = vec![FuzzResult {
            target: FuzzTarget::JsonRpcStructure,
            iterations: 100,
            crashes: 1,
            hangs: 0,
            invalid_responses: 2,
            issues: vec![],
            duration: Duration::from_secs(10),
        }];

        let validation_issues = fuzz_results_to_issues(&results);
        assert_eq!(validation_issues.len(), 0); // No issues in the empty issues vec
    }
}
