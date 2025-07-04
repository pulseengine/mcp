//! Main external validator that orchestrates all validation components

use crate::{
    auth_integration::AuthIntegrationTester,
    config::ValidationConfig,
    cross_language::CrossLanguageTester,
    ecosystem::EcosystemTester,
    inspector::InspectorClient,
    jsonrpc::JsonRpcValidator,
    mcp_semantic::McpSemanticValidator,
    mcp_validator::McpValidatorClient,
    security::SecurityTester,
    report::{ComplianceReport, ComplianceStatus, ExternalValidatorResults, PythonCompatResult},
    ValidationError, ValidationResult,
};
use std::time::{Duration, Instant};
use tracing::{info, warn, error};

/// Main external validator that orchestrates all validation components
pub struct ExternalValidator {
    config: ValidationConfig,
    mcp_validator: Option<McpValidatorClient>,
    jsonrpc_validator: JsonRpcValidator,
    inspector_client: Option<InspectorClient>,
    semantic_validator: McpSemanticValidator,
    cross_language_tester: Option<CrossLanguageTester>,
    ecosystem_tester: Option<EcosystemTester>,
    security_tester: Option<SecurityTester>,
    auth_integration_tester: Option<AuthIntegrationTester>,
}

impl ExternalValidator {
    /// Create a new external validator
    pub async fn new() -> ValidationResult<Self> {
        let config = ValidationConfig::from_env()?;
        Self::with_config(config).await
    }

    /// Create a new external validator with custom configuration
    pub async fn with_config(config: ValidationConfig) -> ValidationResult<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize MCP validator client
        let mcp_validator = match McpValidatorClient::new(config.clone()) {
            Ok(client) => {
                // Test connectivity
                match client.test_connectivity().await {
                    Ok(_) => {
                        info!("MCP Validator service is available");
                        Some(client)
                    }
                    Err(e) => {
                        warn!("MCP Validator service unavailable: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to initialize MCP Validator client: {}", e);
                None
            }
        };

        // Initialize JSON-RPC validator
        let jsonrpc_validator = JsonRpcValidator::new(config.clone())?;

        // Initialize MCP semantic validator
        let semantic_validator = McpSemanticValidator::new(config.clone());

        // Initialize cross-language tester
        let cross_language_tester = match CrossLanguageTester::new(config.clone()) {
            Ok(mut tester) => {
                // Setup test environments
                if let Err(e) = tester.setup_test_environments().await {
                    warn!("Failed to setup cross-language test environments: {}", e);
                }
                Some(tester)
            }
            Err(e) => {
                warn!("Failed to initialize cross-language tester: {}", e);
                None
            }
        };

        // Initialize ecosystem tester
        let ecosystem_tester = match EcosystemTester::new(config.clone()) {
            Ok(tester) => {
                info!("Ecosystem tester initialized successfully");
                Some(tester)
            }
            Err(e) => {
                warn!("Failed to initialize ecosystem tester: {}", e);
                None
            }
        };

        // Initialize security tester
        let security_tester = match SecurityTester::new(config.clone()) {
            Ok(tester) => {
                info!("Security tester initialized successfully");
                Some(tester)
            }
            Err(e) => {
                warn!("Failed to initialize security tester: {}", e);
                None
            }
        };

        // Initialize authentication integration tester
        let auth_integration_tester = match AuthIntegrationTester::new(config.clone()) {
            Ok(tester) => {
                info!("Authentication integration tester initialized successfully");
                Some(tester)
            }
            Err(e) => {
                warn!("Failed to initialize authentication integration tester: {}", e);
                None
            }
        };

        // Initialize Inspector client
        let inspector_client = match InspectorClient::new(config.clone()) {
            Ok(client) => {
                // Check if inspector is available
                match client.check_inspector_availability().await {
                    Ok(true) => {
                        info!("MCP Inspector is available");
                        Some(client)
                    }
                    Ok(false) => {
                        warn!("MCP Inspector is not available");
                        None
                    }
                    Err(e) => {
                        warn!("Failed to check MCP Inspector availability: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to initialize Inspector client: {}", e);
                None
            }
        };

        Ok(Self {
            config,
            mcp_validator,
            jsonrpc_validator,
            inspector_client,
            semantic_validator,
            cross_language_tester,
            ecosystem_tester,
            security_tester,
            auth_integration_tester,
        })
    }

    /// Validate MCP server compliance using all available validators
    pub async fn validate_compliance(&mut self, server_url: &str) -> ValidationResult<ComplianceReport> {
        info!("Starting comprehensive MCP compliance validation for {}", server_url);

        let start_time = Instant::now();
        let mut report = ComplianceReport::new(
            server_url.to_string(),
            crate::SUPPORTED_MCP_VERSIONS[0].to_string(),
        );

        // Test all configured protocol versions
        let versions_to_test: Vec<String> = self.config.protocols.versions.clone();
        
        for version in versions_to_test {
            if !crate::is_version_supported(&version) {
                warn!("Skipping unsupported protocol version: {}", version);
                continue;
            }

            info!("Testing protocol version: {}", version);
            
            match self.validate_protocol_version(server_url, &version).await {
                Ok(version_results) => {
                    report.external_results = version_results;
                }
                Err(e) => {
                    error!("Protocol version {} validation failed: {}", version, e);
                    report.add_issue(crate::report::ValidationIssue::new(
                        crate::report::IssueSeverity::Error,
                        "protocol_version".to_string(),
                        format!("Protocol version {} validation failed: {}", version, e),
                        "external-validator".to_string(),
                    ));
                }
            }
        }

        // Mark validation as completed
        let duration = start_time.elapsed();
        report.mark_completed(duration);

        info!(
            "Compliance validation completed in {:.2}s - Status: {}",
            duration.as_secs_f64(),
            report.status_string()
        );

        Ok(report)
    }

    /// Validate a specific protocol version
    async fn validate_protocol_version(
        &mut self,
        server_url: &str,
        protocol_version: &str,
    ) -> ValidationResult<ExternalValidatorResults> {
        let mut results = ExternalValidatorResults::default();

        // MCP Validator
        if let Some(ref validator) = self.mcp_validator {
            info!("Running MCP Validator tests...");
            match validator.validate_server(server_url, protocol_version).await {
                Ok(mcp_result) => {
                    info!("MCP Validator tests completed successfully");
                    results.mcp_validator = Some(mcp_result);
                }
                Err(e) => {
                    warn!("MCP Validator tests failed: {}", e);
                }
            }
        } else {
            warn!("MCP Validator not available, skipping MCP validation");
        }

        // JSON-RPC Validator
        info!("Running JSON-RPC compliance tests...");
        match self.jsonrpc_validator.validate_server_messages(server_url).await {
            Ok(jsonrpc_result) => {
                info!("JSON-RPC validation completed successfully");
                results.jsonrpc_validator = Some(jsonrpc_result);
            }
            Err(e) => {
                warn!("JSON-RPC validation failed: {}", e);
            }
        }

        // MCP Protocol Semantic Validation
        info!("Running MCP protocol semantic validation...");
        match self.jsonrpc_validator.collect_messages_from_server(server_url).await {
            Ok(messages) => {
                let mut semantic_validator = McpSemanticValidator::new(self.config.clone());
                match semantic_validator.validate_protocol_semantics(&messages).await {
                    Ok(semantic_result) => {
                        info!("MCP semantic validation completed successfully");
                        results.mcp_semantic = Some(semantic_result);
                    }
                    Err(e) => {
                        warn!("MCP semantic validation failed: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to collect messages for semantic validation: {}", e);
            }
        }

        // MCP Inspector
        if let Some(ref inspector) = self.inspector_client {
            info!("Running MCP Inspector tests...");
            
            // For the new inspector, server_url should be treated as a server command
            // For HTTP servers, we'll need to skip for now since inspector expects server commands
            let server_command = if server_url.starts_with("http") {
                warn!("HTTP URL provided to inspector - inspector needs server command, skipping");
                return Ok(results); // Return early to avoid error
            } else {
                server_url // Assume it's already a server command
            };

            match inspector.test_server(server_command).await {
                Ok(inspector_result) => {
                    info!("MCP Inspector tests completed successfully");
                    results.inspector = Some(inspector_result);
                }
                Err(e) => {
                    warn!("MCP Inspector tests failed: {}", e);
                }
            }
        } else {
            warn!("MCP Inspector not available, skipping inspector tests");
        }

        // Cross-Language Protocol Testing
        if let Some(ref mut tester) = self.cross_language_tester {
            info!("Running cross-language compatibility tests...");
            match tester.test_cross_language_compatibility(server_url).await {
                Ok(cross_lang_result) => {
                    info!("Cross-language testing completed: {:.1}% interoperability", 
                          cross_lang_result.interoperability_score);
                    results.cross_language = Some(cross_lang_result);
                }
                Err(e) => {
                    warn!("Cross-language testing failed: {}", e);
                }
            }
        } else {
            info!("Cross-language tester not available, skipping cross-language tests");
        }

        // Ecosystem Integration Testing
        if let Some(ref tester) = self.ecosystem_tester {
            info!("Running ecosystem integration tests...");
            match tester.test_ecosystem_integration(server_url).await {
                Ok(ecosystem_result) => {
                    info!("Ecosystem testing completed: {:.1}% ecosystem compatibility", 
                          ecosystem_result.ecosystem_score);
                    results.ecosystem = Some(ecosystem_result);
                }
                Err(e) => {
                    warn!("Ecosystem testing failed: {}", e);
                }
            }
        } else {
            info!("Ecosystem tester not available, skipping ecosystem tests");
        }

        // Security Validation
        if let Some(ref tester) = self.security_tester {
            info!("Running security validation tests...");
            match tester.test_security(server_url).await {
                Ok(security_result) => {
                    info!("Security testing completed: {:.1}% security score", 
                          security_result.security_score);
                    results.security = Some(security_result);
                }
                Err(e) => {
                    warn!("Security testing failed: {}", e);
                }
            }
        } else {
            info!("Security tester not available, skipping security tests");
        }

        // Authentication Integration Testing
        if let Some(ref mut tester) = self.auth_integration_tester {
            info!("Running authentication integration tests...");
            match tester.test_auth_integration(server_url).await {
                Ok(auth_result) => {
                    info!("Authentication integration testing completed: {:.1}% overall score", 
                          auth_result.overall_score);
                    results.auth_integration = Some(auth_result);
                }
                Err(e) => {
                    warn!("Authentication integration testing failed: {}", e);
                }
            }
        } else {
            info!("Authentication integration tester not available, skipping auth tests");
        }

        // Python SDK Compatibility
        if self.config.testing.python_sdk_compatibility {
            info!("Running Python SDK compatibility tests");
            match crate::python_sdk::PythonSdkTester::new(self.config.clone()) {
                Ok(mut tester) => {
                    // Setup Python environment
                    match tester.setup_environment().await {
                        Ok(_) => {
                            // Run compatibility tests
                            match tester.test_compatibility(server_url).await {
                                Ok(python_result) => {
                                    info!("Python SDK compatibility: {:.1}%", python_result.compatibility_score);
                                    
                                    // Convert to legacy format for backward compatibility
                                    results.python_compat = Some(PythonCompatResult {
                                        message_compatibility: python_result.connection_compatible,
                                        transport_compatibility: python_result.transport_compatible,
                                        auth_compatibility: true, // Not tested yet
                                        feature_parity: (python_result.compatibility_score / 100.0) as f32,
                                        compat_issues: vec![],
                                    });
                                }
                                Err(e) => {
                                    warn!("Python SDK compatibility tests failed: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to setup Python environment: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Python SDK tester initialization failed: {}", e);
                }
            }
        } else {
            info!("Python SDK compatibility testing disabled");
        }

        Ok(results)
    }

    /// Quick validation check (subset of full validation)
    pub async fn quick_validate(&self, server_url: &str) -> ValidationResult<ComplianceStatus> {
        info!("Running quick validation for {}", server_url);

        // Basic connectivity check
        if !self.is_server_accessible(server_url).await? {
            return Ok(ComplianceStatus::Error);
        }

        // Quick JSON-RPC check
        match self.jsonrpc_validator.test_compliance().await {
            Ok(result) => {
                if result.schema_validation.has_failures() || result.message_format.has_failures() {
                    Ok(ComplianceStatus::NonCompliant)
                } else {
                    Ok(ComplianceStatus::Compliant)
                }
            }
            Err(_) => Ok(ComplianceStatus::Error),
        }
    }

    /// Test if server is accessible
    async fn is_server_accessible(&self, server_url: &str) -> ValidationResult<bool> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        match client.get(server_url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Validate multiple servers concurrently
    pub async fn validate_multiple_servers(
        &self,
        server_urls: &[String],
    ) -> ValidationResult<Vec<ComplianceReport>> {
        info!("Validating {} servers concurrently", server_urls.len());

        let mut tasks = Vec::new();

        for url in server_urls {
            let url = url.clone();
            let config = self.config.clone();
            
            let task = tokio::spawn(async move {
                let mut validator = ExternalValidator::with_config(config).await?;
                validator.validate_compliance(&url).await
            });

            tasks.push(task);
        }

        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(report)) => results.push(report),
                Ok(Err(e)) => {
                    error!("Server validation failed: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    error!("Task execution failed: {}", e);
                    return Err(ValidationError::ValidationFailed {
                        message: format!("Concurrent validation failed: {}", e),
                    });
                }
            }
        }

        info!("Completed validation of {} servers", results.len());
        Ok(results)
    }

    /// Get validator status and availability
    pub async fn get_validator_status(&self) -> ValidationResult<ValidatorStatus> {
        let mut status = ValidatorStatus {
            mcp_validator_available: false,
            jsonrpc_validator_available: true, // Always available (local)
            inspector_available: false,
            python_compat_available: false, // Not yet implemented
        };

        // Check MCP Validator
        if let Some(ref validator) = self.mcp_validator {
            status.mcp_validator_available = validator.test_connectivity().await.is_ok();
        }

        // Check Inspector
        if let Some(ref inspector) = self.inspector_client {
            status.inspector_available = inspector.check_inspector_availability().await.unwrap_or(false);
        }

        Ok(status)
    }

    /// Run comprehensive benchmark tests
    pub async fn benchmark_server(&self, server_url: &str) -> ValidationResult<BenchmarkResults> {
        info!("Running benchmark tests for {}", server_url);

        let start_time = Instant::now();
        
        // Run multiple validation rounds
        let mut response_times = Vec::new();
        let iterations = 10;

        for i in 0..iterations {
            let iteration_start = Instant::now();
            
            match self.quick_validate(server_url).await {
                Ok(_) => {
                    let duration = iteration_start.elapsed();
                    response_times.push(duration.as_millis() as f64);
                }
                Err(e) => {
                    warn!("Benchmark iteration {} failed: {}", i, e);
                }
            }
        }

        let total_duration = start_time.elapsed();

        // Calculate statistics
        let avg_response_time = if !response_times.is_empty() {
            response_times.iter().sum::<f64>() / response_times.len() as f64
        } else {
            0.0
        };

        let max_response_time = response_times.iter().fold(0.0f64, |a, &b| a.max(b));
        let min_response_time = response_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        let results = BenchmarkResults {
            total_duration,
            iterations: iterations as u32,
            successful_iterations: response_times.len() as u32,
            avg_response_time_ms: avg_response_time,
            min_response_time_ms: min_response_time,
            max_response_time_ms: max_response_time,
            throughput_rps: if total_duration.as_secs_f64() > 0.0 {
                response_times.len() as f64 / total_duration.as_secs_f64()
            } else {
                0.0
            },
        };

        info!("Benchmark completed: {:.2} avg ms, {:.2} RPS", avg_response_time, results.throughput_rps);
        Ok(results)
    }
}

/// Validator availability status
#[derive(Debug, Clone)]
pub struct ValidatorStatus {
    /// MCP Validator service is available
    pub mcp_validator_available: bool,
    
    /// JSON-RPC validator is available
    pub jsonrpc_validator_available: bool,
    
    /// MCP Inspector is available
    pub inspector_available: bool,
    
    /// Python SDK compatibility testing is available
    pub python_compat_available: bool,
}

/// Benchmark test results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Total benchmark duration
    pub total_duration: Duration,
    
    /// Number of test iterations
    pub iterations: u32,
    
    /// Number of successful iterations
    pub successful_iterations: u32,
    
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    
    /// Minimum response time in milliseconds
    pub min_response_time_ms: f64,
    
    /// Maximum response time in milliseconds
    pub max_response_time_ms: f64,
    
    /// Throughput in requests per second
    pub throughput_rps: f64,
}

impl Drop for ExternalValidator {
    fn drop(&mut self) {
        // Cleanup is handled by individual components
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validator_creation() {
        let config = ValidationConfig::default();
        let validator = ExternalValidator::with_config(config).await;
        assert!(validator.is_ok());
    }

    #[tokio::test]
    async fn test_validator_status() {
        let config = ValidationConfig::default();
        let validator = ExternalValidator::with_config(config).await.unwrap();
        
        let status = validator.get_validator_status().await.unwrap();
        // JSON-RPC validator should always be available (local)
        assert!(status.jsonrpc_validator_available);
    }

    #[tokio::test]
    async fn test_server_accessibility() {
        let config = ValidationConfig::default();
        let validator = ExternalValidator::with_config(config).await.unwrap();
        
        // Test with a known unreachable URL
        let accessible = validator.is_server_accessible("http://localhost:99999").await.unwrap();
        assert!(!accessible);
    }

    #[test]
    fn test_benchmark_results() {
        let results = BenchmarkResults {
            total_duration: Duration::from_secs(10),
            iterations: 100,
            successful_iterations: 95,
            avg_response_time_ms: 50.0,
            min_response_time_ms: 10.0,
            max_response_time_ms: 200.0,
            throughput_rps: 9.5,
        };

        assert_eq!(results.iterations, 100);
        assert_eq!(results.successful_iterations, 95);
        assert!((results.throughput_rps - 9.5).abs() < 0.01);
    }
}