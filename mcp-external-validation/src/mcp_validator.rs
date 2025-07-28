//! MCP Validator client for external protocol validation
//!
//! This module provides integration with the official MCP protocol validator
//! (Janix-ai/mcp-protocol-validator) to ensure compliance with MCP specifications.

use crate::{
    ValidationConfig, ValidationError, ValidationResult,
    report::{IssueSeverity, McpValidatorResult, TestScore, ValidationIssue},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
// use std::time::Duration;  // Removed unused import
use tracing::{debug, info, warn};

/// Client for the MCP Validator service
pub struct McpValidatorClient {
    client: Client,
    config: ValidationConfig,
}

/// Request to MCP Validator API
#[derive(Debug, Serialize)]
struct ValidatorRequest {
    /// Server URL to validate
    server_url: String,

    /// Protocol version to test
    protocol_version: String,

    /// Test categories to run
    test_categories: Vec<String>,

    /// Additional configuration
    config: ValidatorRequestConfig,
}

/// Configuration for validator request
#[derive(Debug, Serialize)]
struct ValidatorRequestConfig {
    /// Timeout for individual tests (seconds)
    timeout: u64,

    /// Enable strict compliance checking
    strict_mode: bool,

    /// Test OAuth 2.1 authentication
    test_oauth: bool,

    /// Test backward compatibility
    test_backward_compat: bool,
}

/// Response from MCP Validator API
#[derive(Debug, Deserialize)]
struct ValidatorResponse {
    /// Overall validation status
    status: String,

    /// Detailed test results
    results: ValidatorTestResults,

    /// List of issues found
    issues: Vec<ValidatorIssue>,

    /// Performance metrics
    performance: ValidatorPerformance,

    /// Validation metadata
    metadata: ValidatorMetadata,
}

/// Detailed test results from validator
#[derive(Debug, Deserialize)]
struct ValidatorTestResults {
    /// HTTP compliance tests (should be 7/7)
    http_compliance: TestResultDetail,

    /// OAuth 2.1 framework tests (should be 6/6)
    oauth_framework: TestResultDetail,

    /// Protocol features tests (should be 7/7)
    protocol_features: TestResultDetail,

    /// Multi-protocol support (should be 3/3)
    multi_protocol: TestResultDetail,

    /// Backward compatibility tests
    backward_compatibility: TestResultDetail,

    /// Security features tests
    security_features: Option<TestResultDetail>,
}

/// Individual test result details
#[derive(Debug, Deserialize)]
struct TestResultDetail {
    /// Number of tests passed
    passed: u32,

    /// Total number of tests
    total: u32,

    /// Test duration in milliseconds
    duration_ms: u64,

    /// Specific test failures
    failures: Vec<TestFailure>,

    /// Additional test metadata
    metadata: Option<serde_json::Value>,
}

/// Individual test failure
#[derive(Debug, Deserialize)]
struct TestFailure {
    /// Test name that failed
    test_name: String,

    /// Failure reason
    reason: String,

    /// Expected vs actual values
    expected: Option<serde_json::Value>,
    actual: Option<serde_json::Value>,

    /// Suggested fix
    suggestion: Option<String>,
}

/// Issue found by validator
#[derive(Debug, Deserialize)]
struct ValidatorIssue {
    /// Issue severity
    severity: String,

    /// Issue category
    category: String,

    /// Issue description
    description: String,

    /// Location where issue was found
    location: Option<String>,

    /// Suggested resolution
    suggestion: Option<String>,

    /// Additional issue details
    details: Option<serde_json::Value>,
}

/// Performance metrics from validator
#[derive(Debug, Deserialize)]
struct ValidatorPerformance {
    /// Total validation time (milliseconds)
    total_time_ms: u64,

    /// Average response time (milliseconds)
    avg_response_time_ms: f64,

    /// Maximum response time (milliseconds)
    max_response_time_ms: f64,

    /// Number of requests made
    total_requests: u32,

    /// Number of failed requests
    failed_requests: u32,
}

/// Validation metadata
#[derive(Debug, Deserialize)]
struct ValidatorMetadata {
    /// Validator version used
    validator_version: String,

    /// Timestamp of validation
    timestamp: String,

    /// Server capabilities detected
    server_capabilities: Vec<String>,

    /// Transport methods tested
    transport_methods: Vec<String>,
}

impl McpValidatorClient {
    /// Create a new MCP validator client
    pub fn new(config: ValidationConfig) -> ValidationResult<Self> {
        let client = Client::builder()
            .timeout(config.validator_timeout_duration())
            .default_headers(config.validator_headers())
            .build()
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self { client, config })
    }

    /// Validate an MCP server using the external validator
    pub async fn validate_server(
        &self,
        server_url: &str,
        protocol_version: &str,
    ) -> ValidationResult<McpValidatorResult> {
        info!("Starting MCP validator validation for {}", server_url);

        let request = ValidatorRequest {
            server_url: server_url.to_string(),
            protocol_version: protocol_version.to_string(),
            test_categories: vec![
                "http_compliance".to_string(),
                "oauth_framework".to_string(),
                "protocol_features".to_string(),
                "multi_protocol".to_string(),
                "backward_compatibility".to_string(),
            ],
            config: ValidatorRequestConfig {
                timeout: self.config.testing.test_timeout,
                strict_mode: self.config.protocols.strict_compliance,
                test_oauth: true,
                test_backward_compat: self.config.protocols.test_backward_compatibility,
            },
        };

        debug!("Sending validation request: {:?}", request);

        let response = self
            .client
            .post(&format!("{}/validate", self.config.validator.api_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                warn!("Failed to send validation request: {}", e);
                ValidationError::NetworkError { source: e }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ValidationError::ExternalValidatorError {
                message: format!("Validator API returned {}: {}", status, error_text),
            });
        }

        let validator_response: ValidatorResponse =
            response
                .json()
                .await
                .map_err(|e| ValidationError::InvalidResponseFormat {
                    details: format!("Failed to parse validator response: {}", e),
                })?;

        debug!("Received validation response: {:?}", validator_response);

        let result = self.convert_validator_response(validator_response)?;

        info!("MCP validation completed successfully");
        Ok(result)
    }

    /// Test connectivity to the MCP validator service
    pub async fn test_connectivity(&self) -> ValidationResult<()> {
        debug!("Testing MCP validator connectivity");

        let response = self
            .client
            .get(&format!("{}/health", self.config.validator.api_url))
            .send()
            .await
            .map_err(|e| ValidationError::NetworkError { source: e })?;

        if response.status().is_success() {
            info!("MCP validator service is accessible");
            Ok(())
        } else {
            Err(ValidationError::ExternalValidatorError {
                message: format!(
                    "MCP validator service returned status: {}",
                    response.status()
                ),
            })
        }
    }

    /// Get validator service information
    pub async fn get_validator_info(&self) -> ValidationResult<ValidatorInfo> {
        debug!("Fetching MCP validator service info");

        let response = self
            .client
            .get(&format!("{}/info", self.config.validator.api_url))
            .send()
            .await
            .map_err(|e| ValidationError::NetworkError { source: e })?;

        if !response.status().is_success() {
            return Err(ValidationError::ExternalValidatorError {
                message: format!("Failed to get validator info: {}", response.status()),
            });
        }

        let info: ValidatorInfo =
            response
                .json()
                .await
                .map_err(|e| ValidationError::InvalidResponseFormat {
                    details: format!("Failed to parse validator info: {}", e),
                })?;

        Ok(info)
    }

    /// Convert validator response to our result format
    fn convert_validator_response(
        &self,
        response: ValidatorResponse,
    ) -> ValidationResult<McpValidatorResult> {
        let results = response.results;

        Ok(McpValidatorResult {
            http_compliance: TestScore::new(
                results.http_compliance.passed,
                results.http_compliance.total,
            ),
            oauth_framework: TestScore::new(
                results.oauth_framework.passed,
                results.oauth_framework.total,
            ),
            protocol_features: TestScore::new(
                results.protocol_features.passed,
                results.protocol_features.total,
            ),
            multi_protocol: TestScore::new(
                results.multi_protocol.passed,
                results.multi_protocol.total,
            ),
            backward_compatibility: TestScore::new(
                results.backward_compatibility.passed,
                results.backward_compatibility.total,
            ),
        })
    }

    /// Convert validator issues to our issue format
    fn convert_validator_issues(&self, issues: Vec<ValidatorIssue>) -> Vec<ValidationIssue> {
        issues
            .into_iter()
            .map(|issue| {
                let severity = match issue.severity.to_lowercase().as_str() {
                    "critical" => IssueSeverity::Critical,
                    "error" => IssueSeverity::Error,
                    "warning" => IssueSeverity::Warning,
                    _ => IssueSeverity::Info,
                };

                let mut validation_issue = ValidationIssue::new(
                    severity,
                    issue.category,
                    issue.description,
                    "mcp-validator".to_string(),
                );

                if let Some(location) = issue.location {
                    validation_issue = validation_issue.with_location(location);
                }

                if let Some(suggestion) = issue.suggestion {
                    validation_issue = validation_issue.with_suggestion(suggestion);
                }

                if let Some(details) = issue.details {
                    validation_issue =
                        validation_issue.with_detail("raw_details".to_string(), details);
                }

                validation_issue
            })
            .collect()
    }
}

/// Information about the validator service
#[derive(Debug, Deserialize)]
pub struct ValidatorInfo {
    /// Validator service version
    pub version: String,

    /// Supported MCP protocol versions
    pub supported_protocols: Vec<String>,

    /// Available test categories
    pub test_categories: Vec<String>,

    /// Service capabilities
    pub capabilities: Vec<String>,

    /// API rate limits
    pub rate_limits: Option<RateLimits>,
}

/// Rate limiting information
#[derive(Debug, Deserialize)]
pub struct RateLimits {
    /// Requests per minute
    pub requests_per_minute: u32,

    /// Burst limit
    pub burst_limit: u32,

    /// Reset time in seconds
    pub reset_time_seconds: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ValidationConfig;

    #[test]
    fn test_validator_client_creation() {
        let config = ValidationConfig::default();
        let client = McpValidatorClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_request_serialization() {
        let request = ValidatorRequest {
            server_url: "http://localhost:3000".to_string(),
            protocol_version: "2025-03-26".to_string(),
            test_categories: vec!["http_compliance".to_string()],
            config: ValidatorRequestConfig {
                timeout: 30,
                strict_mode: true,
                test_oauth: true,
                test_backward_compat: true,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("http://localhost:3000"));
        assert!(json.contains("2025-03-26"));
    }

    #[tokio::test]
    async fn test_connectivity_check() {
        // This test requires the validator service to be running
        // Skip in CI unless service is available
        if std::env::var("MCP_VALIDATOR_TEST").is_err() {
            return;
        }

        let config = ValidationConfig::default();
        let client = McpValidatorClient::new(config).unwrap();

        // This will fail if service is not available, which is expected
        let _ = client.test_connectivity().await;
    }
}
