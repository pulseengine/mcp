//! Validation reports and issue tracking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

// Import McpSemanticResult from mcp_semantic module
pub use crate::mcp_semantic::McpSemanticResult;
// Import CrossLanguageResult from cross_language module
pub use crate::cross_language::CrossLanguageResult;
// Import EcosystemResult from ecosystem module
pub use crate::ecosystem::EcosystemResult;
// Import SecurityResult from security module
pub use crate::security::SecurityResult;
// Import AuthIntegrationResult from auth_integration module
pub use crate::auth_integration::AuthIntegrationResult;

/// Comprehensive validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// Server URL that was tested
    pub server_url: String,

    /// Timestamp when validation was performed
    pub timestamp: SystemTime,

    /// Total duration of validation
    pub duration: Duration,

    /// Protocol version tested
    pub protocol_version: String,

    /// Overall compliance status
    pub status: ComplianceStatus,

    /// List of validation issues found
    pub issues: Vec<ValidationIssue>,

    /// Detailed test results by category
    pub test_results: HashMap<String, TestCategoryResult>,

    /// Performance metrics collected during testing
    pub performance: PerformanceMetrics,

    /// External validator results
    pub external_results: ExternalValidatorResults,
}

/// Overall compliance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    /// Fully compliant with all specifications
    Compliant,
    /// Minor issues that don't break compatibility
    Warning,
    /// Significant issues that may cause problems
    NonCompliant,
    /// Validation could not be completed
    Error,
}

/// Individual validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Severity level of the issue
    pub severity: IssueSeverity,

    /// Category of the issue (e.g., "protocol", "transport", "security")
    pub category: String,

    /// Human-readable description of the issue
    pub description: String,

    /// Specific location where issue was found (optional)
    pub location: Option<String>,

    /// Suggested fix for the issue
    pub suggestion: Option<String>,

    /// External validator that found this issue
    pub validator: String,

    /// Additional context or details
    pub details: HashMap<String, serde_json::Value>,
}

/// Severity levels for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Informational message
    Info,
    /// Warning that should be addressed
    Warning,
    /// Error that breaks compatibility
    Error,
    /// Critical error that prevents operation
    Critical,
}

/// Test results for a specific category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCategoryResult {
    /// Category name (e.g., "MCP Protocol", "JSON-RPC", "Transport")
    pub category: String,

    /// Number of tests passed
    pub passed: u32,

    /// Number of tests failed
    pub failed: u32,

    /// Number of tests skipped
    pub skipped: u32,

    /// Duration of tests in this category
    pub duration: Duration,

    /// Specific test details
    pub tests: Vec<IndividualTestResult>,
}

/// Result of an individual test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualTestResult {
    /// Test name
    pub name: String,

    /// Test result
    pub result: TestResult,

    /// Duration of the test
    pub duration: Duration,

    /// Error message if test failed
    pub error: Option<String>,

    /// Additional test details
    pub details: HashMap<String, serde_json::Value>,
}

/// Individual test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestResult {
    Passed,
    Failed,
    Skipped,
    Error,
}

/// Performance metrics collected during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average response time for tool calls (milliseconds)
    pub avg_response_time_ms: f64,

    /// 95th percentile response time (milliseconds)
    pub p95_response_time_ms: f64,

    /// 99th percentile response time (milliseconds)
    pub p99_response_time_ms: f64,

    /// Maximum response time observed (milliseconds)
    pub max_response_time_ms: f64,

    /// Number of requests that timed out
    pub timeouts: u32,

    /// Number of requests that failed
    pub failures: u32,

    /// Total number of requests made
    pub total_requests: u32,

    /// Throughput (requests per second)
    pub throughput_rps: f64,

    /// Memory usage statistics (if available)
    pub memory_usage: Option<MemoryStats>,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Peak memory usage (bytes)
    pub peak_memory_bytes: u64,

    /// Average memory usage (bytes)
    pub avg_memory_bytes: u64,

    /// Memory usage at end of test (bytes)
    pub final_memory_bytes: u64,
}

/// Results from external validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalValidatorResults {
    /// MCP Validator results
    pub mcp_validator: Option<McpValidatorResult>,

    /// JSON-RPC validator results
    pub jsonrpc_validator: Option<JsonRpcValidatorResult>,

    /// MCP Inspector results
    pub inspector: Option<InspectorResult>,

    /// Python SDK compatibility results
    pub python_compat: Option<PythonCompatResult>,

    /// MCP protocol semantic validation results
    pub mcp_semantic: Option<McpSemanticResult>,

    /// Cross-language compatibility results
    pub cross_language: Option<CrossLanguageResult>,

    /// Ecosystem integration results
    pub ecosystem: Option<EcosystemResult>,

    /// Security validation results
    pub security: Option<SecurityResult>,

    /// Authentication integration results
    pub auth_integration: Option<AuthIntegrationResult>,
}

/// MCP Validator specific results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpValidatorResult {
    /// HTTP compliance test results (7/7 tests)
    pub http_compliance: TestScore,

    /// OAuth 2.1 framework results (6/6 tests)
    pub oauth_framework: TestScore,

    /// Protocol features results (7/7 tests)
    pub protocol_features: TestScore,

    /// Multi-protocol support (3/3 versions)
    pub multi_protocol: TestScore,

    /// Backward compatibility score
    pub backward_compatibility: TestScore,
}

/// JSON-RPC validator specific results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcValidatorResult {
    /// Schema validation score
    pub schema_validation: TestScore,

    /// Message format compliance
    pub message_format: TestScore,

    /// Error handling compliance
    pub error_handling: TestScore,

    /// Request/response correlation
    pub correlation: TestScore,
}

impl JsonRpcValidatorResult {
    /// Get total number of issues (failed tests)
    pub fn get_total_issues(&self) -> u32 {
        let schema_issues = self.schema_validation.total - self.schema_validation.passed;
        let format_issues = self.message_format.total - self.message_format.passed;
        let error_issues = self.error_handling.total - self.error_handling.passed;
        let correlation_issues = self.correlation.total - self.correlation.passed;

        schema_issues + format_issues + error_issues + correlation_issues
    }
}

/// MCP Inspector specific results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorResult {
    /// Successfully connected to server
    pub connection_success: bool,

    /// Authentication worked correctly
    pub auth_success: bool,

    /// Tools were discovered and callable
    pub tools_discoverable: bool,

    /// Resources were accessible
    pub resources_accessible: bool,

    /// Export configurations worked
    pub export_success: bool,

    /// Inspector-specific issues found
    pub inspector_issues: Vec<String>,
}

/// Python SDK compatibility results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonCompatResult {
    /// Message format compatibility
    pub message_compatibility: bool,

    /// Transport layer compatibility
    pub transport_compatibility: bool,

    /// Authentication compatibility
    pub auth_compatibility: bool,

    /// Feature parity score (0.0 to 1.0)
    pub feature_parity: f32,

    /// Specific compatibility issues
    pub compat_issues: Vec<String>,
}

/// Python SDK test results (detailed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonSdkResult {
    /// Python SDK version
    pub sdk_version: String,

    /// Connection compatible
    pub connection_compatible: bool,

    /// Tools compatible
    pub tools_compatible: bool,

    /// Resources compatible
    pub resources_compatible: bool,

    /// Transport compatible
    pub transport_compatible: bool,

    /// Error handling compatible
    pub error_handling_compatible: bool,

    /// Overall compatibility score (0-100)
    pub compatibility_score: f64,
}

/// Test score representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScore {
    /// Number of tests passed
    pub passed: u32,

    /// Total number of tests
    pub total: u32,

    /// Score as percentage (0.0 to 1.0)
    pub score: f32,
}

impl ComplianceReport {
    /// Create a new compliance report
    pub fn new(server_url: String, protocol_version: String) -> Self {
        Self {
            server_url,
            timestamp: SystemTime::now(),
            duration: Duration::from_secs(0),
            protocol_version,
            status: ComplianceStatus::Error,
            issues: Vec::new(),
            test_results: HashMap::new(),
            performance: PerformanceMetrics::default(),
            external_results: ExternalValidatorResults::default(),
        }
    }

    /// Check if the server is compliant
    pub fn is_compliant(&self) -> bool {
        matches!(
            self.status,
            ComplianceStatus::Compliant | ComplianceStatus::Warning
        )
    }

    /// Get all issues found during validation
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Get issues by severity level
    pub fn issues_by_severity(&self, severity: IssueSeverity) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == severity)
            .collect()
    }

    /// Get critical and error issues
    pub fn critical_issues(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Critical | IssueSeverity::Error))
            .collect()
    }

    /// Add a validation issue
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        // Update overall status based on issue severity
        match issue.severity {
            IssueSeverity::Critical | IssueSeverity::Error => {
                self.status = ComplianceStatus::NonCompliant;
            }
            IssueSeverity::Warning => {
                if self.status == ComplianceStatus::Compliant {
                    self.status = ComplianceStatus::Warning;
                }
            }
            IssueSeverity::Info => {} // Don't change status for info
        }

        self.issues.push(issue);
    }

    /// Mark validation as completed successfully
    pub fn mark_completed(&mut self, duration: Duration) {
        self.duration = duration;

        // If no critical issues were found, mark as compliant or warning
        if !self
            .issues
            .iter()
            .any(|i| matches!(i.severity, IssueSeverity::Critical | IssueSeverity::Error))
        {
            self.status = if self
                .issues
                .iter()
                .any(|i| i.severity == IssueSeverity::Warning)
            {
                ComplianceStatus::Warning
            } else {
                ComplianceStatus::Compliant
            };
        }
    }

    /// Get overall test statistics
    pub fn test_statistics(&self) -> (u32, u32, u32) {
        let (mut passed, mut failed, mut skipped) = (0, 0, 0);

        for result in self.test_results.values() {
            passed += result.passed;
            failed += result.failed;
            skipped += result.skipped;
        }

        (passed, failed, skipped)
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        let (passed, failed, skipped) = self.test_statistics();
        let total = passed + failed + skipped;

        format!(
            "MCP Compliance Report: {} - {}/{} tests passed, {} issues found",
            self.status_string(),
            passed,
            total,
            self.issues.len()
        )
    }

    /// Get status as string
    pub fn status_string(&self) -> &'static str {
        match self.status {
            ComplianceStatus::Compliant => "COMPLIANT",
            ComplianceStatus::Warning => "WARNING",
            ComplianceStatus::NonCompliant => "NON-COMPLIANT",
            ComplianceStatus::Error => "ERROR",
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_response_time_ms: 0.0,
            p95_response_time_ms: 0.0,
            p99_response_time_ms: 0.0,
            max_response_time_ms: 0.0,
            timeouts: 0,
            failures: 0,
            total_requests: 0,
            throughput_rps: 0.0,
            memory_usage: None,
        }
    }
}

impl Default for ExternalValidatorResults {
    fn default() -> Self {
        Self {
            mcp_validator: None,
            jsonrpc_validator: None,
            inspector: None,
            python_compat: None,
            mcp_semantic: None,
            cross_language: None,
            ecosystem: None,
            security: None,
            auth_integration: None,
        }
    }
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(
        severity: IssueSeverity,
        category: String,
        description: String,
        validator: String,
    ) -> Self {
        Self {
            severity,
            category,
            description,
            location: None,
            suggestion: None,
            validator,
            details: HashMap::new(),
        }
    }

    /// Add a suggestion for fixing the issue
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Add location information
    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

    /// Add additional details
    pub fn with_detail(mut self, key: String, value: serde_json::Value) -> Self {
        self.details.insert(key, value);
        self
    }
}

impl TestScore {
    /// Create a new test score
    pub fn new(passed: u32, total: u32) -> Self {
        let score = if total > 0 {
            passed as f32 / total as f32
        } else {
            0.0
        };

        Self {
            passed,
            total,
            score,
        }
    }

    /// Check if all tests passed
    pub fn is_perfect(&self) -> bool {
        self.passed == self.total && self.total > 0
    }

    /// Check if any tests failed
    pub fn has_failures(&self) -> bool {
        self.passed < self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_report() {
        let mut report = ComplianceReport::new(
            "http://localhost:3000".to_string(),
            "2025-03-26".to_string(),
        );

        assert_eq!(report.status, ComplianceStatus::Error);
        assert!(report.issues().is_empty());

        // Add a warning
        report.add_issue(ValidationIssue::new(
            IssueSeverity::Warning,
            "protocol".to_string(),
            "Minor issue".to_string(),
            "test".to_string(),
        ));

        // Add a critical issue
        report.add_issue(ValidationIssue::new(
            IssueSeverity::Critical,
            "protocol".to_string(),
            "Critical issue".to_string(),
            "test".to_string(),
        ));

        assert_eq!(report.status, ComplianceStatus::NonCompliant);
        assert_eq!(report.issues().len(), 2);
        assert_eq!(report.critical_issues().len(), 1);
    }

    #[test]
    fn test_test_score() {
        let perfect = TestScore::new(7, 7);
        assert!(perfect.is_perfect());
        assert!(!perfect.has_failures());
        assert_eq!(perfect.score, 1.0);

        let partial = TestScore::new(5, 7);
        assert!(!partial.is_perfect());
        assert!(partial.has_failures());
        assert!((partial.score - 0.714).abs() < 0.01);
    }

    #[test]
    fn test_validation_issue() {
        let issue = ValidationIssue::new(
            IssueSeverity::Error,
            "transport".to_string(),
            "Connection failed".to_string(),
            "inspector".to_string(),
        )
        .with_suggestion("Check server is running".to_string())
        .with_location("http://localhost:3000".to_string())
        .with_detail("error_code".to_string(), serde_json::json!(500));

        assert_eq!(issue.severity, IssueSeverity::Error);
        assert_eq!(
            issue.suggestion,
            Some("Check server is running".to_string())
        );
        assert_eq!(issue.location, Some("http://localhost:3000".to_string()));
        assert_eq!(
            issue.details.get("error_code"),
            Some(&serde_json::json!(500))
        );
    }
}
