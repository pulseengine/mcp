//! Authentication integration for external validation
//!
//! This module provides integration between the authentication framework
//! and the external validation system, enabling authentication-aware
//! validation and security testing.

use crate::{
    ValidationConfig, ValidationError, ValidationResult,
    report::{IssueSeverity, TestScore, ValidationIssue},
};
use pulseengine_mcp_auth::{
    AuthenticationManager, RateLimitStats, Role, ValidationConfig as AuthValidationConfig,
    validation::permissions,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info, warn};

/// Authentication integration tester
pub struct AuthIntegrationTester {
    /// Validation configuration
    config: ValidationConfig,
    /// Authentication manager for testing
    auth_manager: Option<AuthenticationManager>,
    /// HTTP client for requests
    http_client: Client,
    /// Test scenarios for authentication
    test_scenarios: Vec<AuthTestScenario>,
}

/// Authentication integration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthIntegrationResult {
    /// Authentication framework availability
    pub framework_available: bool,
    /// API key management functionality score
    pub api_key_management: TestScore,
    /// Rate limiting effectiveness score
    pub rate_limiting: TestScore,
    /// Permission validation score
    pub permission_validation: TestScore,
    /// Session security score
    pub session_security: TestScore,
    /// Integration compatibility score
    pub integration_compatibility: TestScore,
    /// Framework security configuration score
    pub security_configuration: TestScore,
    /// Overall authentication integration score (0-100)
    pub overall_score: f64,
    /// Issues found during integration testing
    pub issues: Vec<ValidationIssue>,
    /// Authentication statistics
    pub auth_stats: Option<AuthStatistics>,
}

/// Authentication test scenario
#[derive(Debug, Clone)]
pub struct AuthTestScenario {
    /// Scenario name
    pub name: String,
    /// Scenario description
    pub description: String,
    /// Test type
    pub test_type: AuthTestType,
    /// Expected outcome
    pub expected_outcome: AuthTestOutcome,
    /// Test data/payload
    pub test_data: Value,
}

/// Types of authentication tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthTestType {
    /// Test API key creation and validation
    ApiKeyLifecycle,
    /// Test rate limiting functionality
    RateLimiting,
    /// Test role-based permissions
    RoleBasedAccess,
    /// Test IP whitelisting
    IpWhitelisting,
    /// Test session management
    SessionManagement,
    /// Test authentication bypass attempts
    AuthBypassAttempt,
    /// Test framework integration points
    FrameworkIntegration,
    /// Test security configuration
    SecurityConfiguration,
}

/// Expected authentication test outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthTestOutcome {
    /// Authentication should succeed
    Success,
    /// Authentication should fail
    Failure,
    /// Rate limiting should trigger
    RateLimited,
    /// Permission should be denied
    PermissionDenied,
    /// Framework integration should work
    IntegrationSuccess,
    /// Security configuration should be valid
    ConfigurationValid,
}

/// Authentication statistics from testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatistics {
    /// Total API keys created during testing
    pub keys_created: u32,
    /// Total validation attempts
    pub validation_attempts: u32,
    /// Successful validations
    pub successful_validations: u32,
    /// Failed validations
    pub failed_validations: u32,
    /// Rate limit statistics
    pub rate_limit_stats: Option<RateLimitStats>,
    /// Test duration
    pub test_duration_seconds: f64,
}

impl AuthIntegrationTester {
    /// Create a new authentication integration tester
    pub fn new(config: ValidationConfig) -> ValidationResult<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30)) // Default 30 second timeout
            .build()
            .map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let test_scenarios = Self::create_default_test_scenarios();

        Ok(Self {
            config,
            auth_manager: None,
            http_client,
            test_scenarios,
        })
    }

    /// Initialize authentication manager for testing
    pub async fn initialize_auth_manager(&mut self) -> ValidationResult<()> {
        use pulseengine_mcp_auth::{AuthConfig, config::StorageConfig};

        // Create temporary in-memory authentication configuration for testing
        let auth_config = AuthConfig {
            enabled: true,
            storage: StorageConfig::Memory,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 3,
            rate_limit_window_secs: 300,
        };

        let auth_validation_config = AuthValidationConfig {
            max_failed_attempts: 3,
            failed_attempt_window_minutes: 5,
            block_duration_minutes: 10,
            session_timeout_minutes: 60,
            strict_ip_validation: true,
            enable_role_based_rate_limiting: false,
            role_rate_limits: HashMap::new(),
        };

        match AuthenticationManager::new_with_validation(auth_config, auth_validation_config).await
        {
            Ok(manager) => {
                info!("Authentication manager initialized for testing");
                self.auth_manager = Some(manager);
                Ok(())
            }
            Err(e) => {
                error!("Failed to initialize authentication manager: {}", e);
                Err(ValidationError::ConfigurationError {
                    message: format!("Authentication manager setup failed: {}", e),
                })
            }
        }
    }

    /// Run comprehensive authentication integration tests
    pub async fn test_auth_integration(
        &mut self,
        server_url: &str,
    ) -> ValidationResult<AuthIntegrationResult> {
        let start_time = std::time::Instant::now();
        let mut result = AuthIntegrationResult {
            framework_available: false,
            api_key_management: TestScore::new(0, 100),
            rate_limiting: TestScore::new(0, 100),
            permission_validation: TestScore::new(0, 100),
            session_security: TestScore::new(0, 100),
            integration_compatibility: TestScore::new(0, 100),
            security_configuration: TestScore::new(0, 100),
            overall_score: 0.0,
            issues: Vec::new(),
            auth_stats: None,
        };

        // Check if authentication framework is available
        result.framework_available = self.check_framework_availability(&mut result).await;

        if result.framework_available {
            // Run authentication test scenarios
            let mut stats = AuthStatistics {
                keys_created: 0,
                validation_attempts: 0,
                successful_validations: 0,
                failed_validations: 0,
                rate_limit_stats: None,
                test_duration_seconds: 0.0,
            };

            // Test API key management
            result.api_key_management = self.test_api_key_management(&mut result, &mut stats).await;

            // Test rate limiting
            result.rate_limiting = self.test_rate_limiting(&mut result, &mut stats).await;

            // Test permission validation
            result.permission_validation = self
                .test_permission_validation(&mut result, &mut stats)
                .await;

            // Test session security
            result.session_security = self.test_session_security(&mut result, &mut stats).await;

            // Test integration compatibility
            result.integration_compatibility = self
                .test_integration_compatibility(server_url, &mut result, &mut stats)
                .await;

            // Test security configuration
            result.security_configuration = self
                .test_security_configuration(&mut result, &mut stats)
                .await;

            // Get rate limit stats from auth manager
            if let Some(auth_manager) = &self.auth_manager {
                stats.rate_limit_stats = Some(auth_manager.get_rate_limit_stats().await);
            }

            stats.test_duration_seconds = start_time.elapsed().as_secs_f64();
            result.auth_stats = Some(stats);
        }

        // Calculate overall score
        result.overall_score = self.calculate_overall_score(&result);

        Ok(result)
    }

    /// Check if the authentication framework is available and functional
    async fn check_framework_availability(&mut self, result: &mut AuthIntegrationResult) -> bool {
        match self.initialize_auth_manager().await {
            Ok(_) => {
                info!("Authentication framework is available and functional");
                true
            }
            Err(e) => {
                error!("Authentication framework is not available: {}", e);
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Critical,
                    "framework-availability".to_string(),
                    format!("Authentication framework unavailable: {}", e),
                    "auth-integration-tester".to_string(),
                ));
                false
            }
        }
    }

    /// Test API key management functionality
    async fn test_api_key_management(
        &mut self,
        result: &mut AuthIntegrationResult,
        stats: &mut AuthStatistics,
    ) -> TestScore {
        let mut passed_tests = 0;
        let total_tests = 4; // Creation, validation, listing, revocation

        let auth_manager = match &self.auth_manager {
            Some(manager) => manager,
            None => {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Critical,
                    "api-key-management".to_string(),
                    "No authentication manager available for testing".to_string(),
                    "auth-integration-tester".to_string(),
                ));
                return TestScore::new(0, total_tests);
            }
        };

        // Test API key creation
        match auth_manager
            .create_api_key(
                "test-admin-key".to_string(),
                Role::Admin,
                None,
                Some(vec!["192.168.1.0/24".to_string()]),
            )
            .await
        {
            Ok(key) => {
                info!("Successfully created test API key: {}", key.id);
                passed_tests += 1;
                stats.keys_created += 1;

                // Test API key validation
                stats.validation_attempts += 1;
                match auth_manager
                    .validate_api_key(&key.key, Some("192.168.1.100"))
                    .await
                {
                    Ok(Some(_context)) => {
                        info!("API key validation successful");
                        passed_tests += 1;
                        stats.successful_validations += 1;
                    }
                    Ok(None) => {
                        warn!("API key validation returned None");
                        stats.failed_validations += 1;
                        result.issues.push(ValidationIssue::new(
                            IssueSeverity::Warning,
                            "api-key-validation".to_string(),
                            "API key validation returned None for valid key".to_string(),
                            "auth-integration-tester".to_string(),
                        ));
                    }
                    Err(e) => {
                        error!("API key validation failed: {}", e);
                        stats.failed_validations += 1;
                        result.issues.push(ValidationIssue::new(
                            IssueSeverity::Error,
                            "api-key-validation".to_string(),
                            format!("API key validation error: {}", e),
                            "auth-integration-tester".to_string(),
                        ));
                    }
                }

                // Test key listing
                let keys = auth_manager.list_keys().await;
                if keys.len() >= 1 {
                    info!("API key listing functional: {} keys found", keys.len());
                    passed_tests += 1;
                } else {
                    result.issues.push(ValidationIssue::new(
                        IssueSeverity::Warning,
                        "api-key-listing".to_string(),
                        "API key listing returned unexpected results".to_string(),
                        "auth-integration-tester".to_string(),
                    ));
                }

                // Test key revocation
                match auth_manager.revoke_key(&key.id).await {
                    Ok(true) => {
                        info!("API key revocation successful");
                        passed_tests += 1;
                    }
                    Ok(false) => {
                        warn!("API key revocation returned false");
                        result.issues.push(ValidationIssue::new(
                            IssueSeverity::Warning,
                            "api-key-revocation".to_string(),
                            "API key revocation returned false for existing key".to_string(),
                            "auth-integration-tester".to_string(),
                        ));
                    }
                    Err(e) => {
                        error!("API key revocation failed: {}", e);
                        result.issues.push(ValidationIssue::new(
                            IssueSeverity::Error,
                            "api-key-revocation".to_string(),
                            format!("API key revocation error: {}", e),
                            "auth-integration-tester".to_string(),
                        ));
                    }
                }
            }
            Err(e) => {
                error!("Failed to create test API key: {}", e);
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Critical,
                    "api-key-creation".to_string(),
                    format!("API key creation failed: {}", e),
                    "auth-integration-tester".to_string(),
                ));
            }
        }

        TestScore::new(passed_tests, total_tests)
    }

    /// Test rate limiting functionality
    async fn test_rate_limiting(
        &mut self,
        result: &mut AuthIntegrationResult,
        stats: &mut AuthStatistics,
    ) -> TestScore {
        let mut passed_tests = 0;
        let total_tests = 3;

        let auth_manager = match &self.auth_manager {
            Some(manager) => manager,
            None => return TestScore::new(0, total_tests),
        };

        // Test rate limiting by making multiple failed authentication attempts
        let test_ip = "192.168.1.200";
        let invalid_key = "invalid_key_for_testing";

        for i in 1..=5 {
            stats.validation_attempts += 1;
            match auth_manager
                .validate_api_key(invalid_key, Some(test_ip))
                .await
            {
                Err(e) if e.to_string().contains("rate limited") => {
                    info!("Rate limiting triggered on attempt {}", i);
                    passed_tests += 1;
                    break;
                }
                Err(_) => {
                    // Expected for invalid key
                    stats.failed_validations += 1;
                    if i == 1 {
                        passed_tests += 1; // First failure is expected
                    }
                }
                Ok(_) => {
                    warn!("Unexpected successful validation with invalid key");
                    result.issues.push(ValidationIssue::new(
                        IssueSeverity::Error,
                        "rate-limiting".to_string(),
                        "Invalid API key was accepted during rate limiting test".to_string(),
                        "auth-integration-tester".to_string(),
                    ));
                    break;
                }
            }
        }

        // Test rate limit statistics
        let rate_stats = auth_manager.get_rate_limit_stats().await;
        if rate_stats.total_tracked_ips > 0 {
            info!(
                "Rate limiting statistics available: {} tracked IPs",
                rate_stats.total_tracked_ips
            );
            passed_tests += 1;
        }

        TestScore::new(passed_tests, total_tests)
    }

    /// Test permission validation
    async fn test_permission_validation(
        &mut self,
        result: &mut AuthIntegrationResult,
        _stats: &mut AuthStatistics,
    ) -> TestScore {
        let mut passed_tests = 0;
        let total_tests = 3;
        let auth_manager = match &self.auth_manager {
            Some(manager) => manager,
            None => return TestScore::new(0, total_tests),
        };

        // Test different role permissions
        let roles_to_test = vec![
            ("admin", Role::Admin, permissions::ADMIN_CREATE_KEY),
            ("operator", Role::Operator, permissions::DEVICE_CONTROL),
            ("monitor", Role::Monitor, permissions::SYSTEM_STATUS),
        ];

        for (role_name, role, permission) in roles_to_test {
            match auth_manager
                .create_api_key(format!("test-{}-key", role_name), role.clone(), None, None)
                .await
            {
                Ok(key) => {
                    match auth_manager
                        .validate_api_key(&key.key, Some("127.0.0.1"))
                        .await
                    {
                        Ok(Some(context)) => {
                            if context.has_permission(permission) {
                                info!("Permission validation successful for {} role", role_name);
                                passed_tests += 1;
                            } else {
                                result.issues.push(ValidationIssue::new(
                                    IssueSeverity::Warning,
                                    "permission-validation".to_string(),
                                    format!(
                                        "Role {} missing expected permission {}",
                                        role_name, permission
                                    ),
                                    "auth-integration-tester".to_string(),
                                ));
                            }
                        }
                        _ => {
                            result.issues.push(ValidationIssue::new(
                                IssueSeverity::Error,
                                "permission-validation".to_string(),
                                format!("Failed to validate API key for {} role", role_name),
                                "auth-integration-tester".to_string(),
                            ));
                        }
                    }

                    // Clean up
                    let _ = auth_manager.revoke_key(&key.id).await;
                }
                Err(e) => {
                    error!("Failed to create test key for {} role: {}", role_name, e);
                }
            }
        }

        TestScore::new(passed_tests, total_tests)
    }

    /// Test session security
    async fn test_session_security(
        &mut self,
        result: &mut AuthIntegrationResult,
        _stats: &mut AuthStatistics,
    ) -> TestScore {
        let mut passed_tests = 1; // Base score for having session management
        let total_tests = 3;

        // Test IP whitelisting
        let auth_manager = match &self.auth_manager {
            Some(manager) => manager,
            None => return TestScore::new(0, total_tests),
        };

        match auth_manager
            .create_api_key(
                "test-ip-restricted-key".to_string(),
                Role::Operator,
                None,
                Some(vec!["192.168.1.0/24".to_string()]),
            )
            .await
        {
            Ok(key) => {
                // Test with allowed IP
                match auth_manager
                    .validate_api_key(&key.key, Some("192.168.1.100"))
                    .await
                {
                    Ok(Some(_)) => {
                        info!("IP whitelisting allows authorized IP");
                        passed_tests += 1;
                    }
                    _ => {
                        result.issues.push(ValidationIssue::new(
                            IssueSeverity::Warning,
                            "ip-whitelisting".to_string(),
                            "IP whitelisting rejected authorized IP".to_string(),
                            "auth-integration-tester".to_string(),
                        ));
                    }
                }

                // Test with disallowed IP
                match auth_manager
                    .validate_api_key(&key.key, Some("10.0.0.100"))
                    .await
                {
                    Err(_) => {
                        info!("IP whitelisting correctly blocks unauthorized IP");
                        passed_tests += 1;
                    }
                    Ok(_) => {
                        result.issues.push(ValidationIssue::new(
                            IssueSeverity::Error,
                            "ip-whitelisting".to_string(),
                            "IP whitelisting failed to block unauthorized IP".to_string(),
                            "auth-integration-tester".to_string(),
                        ));
                    }
                }

                // Clean up
                let _ = auth_manager.revoke_key(&key.id).await;
            }
            Err(e) => {
                error!("Failed to create IP-restricted key: {}", e);
            }
        }

        TestScore::new(passed_tests, total_tests)
    }

    /// Test integration compatibility with external systems
    async fn test_integration_compatibility(
        &mut self,
        _server_url: &str,
        result: &mut AuthIntegrationResult,
        _stats: &mut AuthStatistics,
    ) -> TestScore {
        let mut passed_tests = 0;
        let total_tests = 4;

        // Test HTTP header extraction
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            "Bearer test_token_123".to_string(),
        );
        headers.insert("x-api-key".to_string(), "test_api_key_456".to_string());
        headers.insert(
            "x-forwarded-for".to_string(),
            "192.168.1.1, 10.0.0.1".to_string(),
        );

        // Test authentication header extraction
        let extracted_token = pulseengine_mcp_auth::validation::extract_api_key(&headers, None);
        if extracted_token == Some("test_token_123".to_string()) {
            info!("Authentication header extraction works correctly");
            passed_tests += 1;
        } else {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Warning,
                "header-extraction".to_string(),
                "Failed to extract authentication token from Bearer header".to_string(),
                "auth-integration-tester".to_string(),
            ));
        }

        // Test IP extraction
        let extracted_ip = pulseengine_mcp_auth::validation::extract_client_ip(&headers);
        if extracted_ip == "192.168.1.1" {
            info!("Client IP extraction works correctly");
            passed_tests += 1;
        } else {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Warning,
                "ip-extraction".to_string(),
                "Failed to extract client IP from forwarded headers".to_string(),
                "auth-integration-tester".to_string(),
            ));
        }

        // Test input validation utilities
        if pulseengine_mcp_auth::validation::is_valid_uuid("550e8400-e29b-41d4-a716-446655440000") {
            passed_tests += 1;
        }

        if pulseengine_mcp_auth::validation::is_valid_ip_address("192.168.1.1") {
            passed_tests += 1;
        }

        TestScore::new(passed_tests, total_tests)
    }

    /// Test security configuration
    async fn test_security_configuration(
        &mut self,
        result: &mut AuthIntegrationResult,
        _stats: &mut AuthStatistics,
    ) -> TestScore {
        let mut passed_tests = 1; // Base score for having configuration
        let total_tests = 4;

        // Test input sanitization
        let dangerous_input = "test<script>alert('xss')</script>";
        let sanitized = pulseengine_mcp_auth::validation::sanitize_input(dangerous_input);
        if !sanitized.contains("<script>") {
            info!("Input sanitization works correctly");
            passed_tests += 1;
        } else {
            result.issues.push(ValidationIssue::new(
                IssueSeverity::Error,
                "input-sanitization".to_string(),
                "Input sanitization failed to remove dangerous content".to_string(),
                "auth-integration-tester".to_string(),
            ));
        }

        // Test input format validation
        match pulseengine_mcp_auth::validation::validate_input_format("valid_input", 20, false) {
            Ok(_) => {
                passed_tests += 1;
            }
            Err(e) => {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Warning,
                    "input-validation".to_string(),
                    format!("Input format validation error: {}", e),
                    "auth-integration-tester".to_string(),
                ));
            }
        }

        // Test dangerous input rejection
        match pulseengine_mcp_auth::validation::validate_input_format("dangerous@input", 20, false)
        {
            Err(_) => {
                info!("Input validation correctly rejects dangerous characters");
                passed_tests += 1;
            }
            Ok(_) => {
                result.issues.push(ValidationIssue::new(
                    IssueSeverity::Warning,
                    "input-validation".to_string(),
                    "Input validation failed to reject dangerous characters".to_string(),
                    "auth-integration-tester".to_string(),
                ));
            }
        }

        TestScore::new(passed_tests, total_tests)
    }

    /// Calculate overall authentication integration score
    fn calculate_overall_score(&self, result: &AuthIntegrationResult) -> f64 {
        if !result.framework_available {
            return 0.0;
        }

        let scores = vec![
            result.api_key_management.score as f64 * 0.25,
            result.rate_limiting.score as f64 * 0.15,
            result.permission_validation.score as f64 * 0.20,
            result.session_security.score as f64 * 0.15,
            result.integration_compatibility.score as f64 * 0.15,
            result.security_configuration.score as f64 * 0.10,
        ];

        scores.iter().sum::<f64>() * 100.0
    }

    /// Create default test scenarios
    fn create_default_test_scenarios() -> Vec<AuthTestScenario> {
        vec![
            AuthTestScenario {
                name: "API Key Lifecycle".to_string(),
                description: "Test complete API key lifecycle: create, validate, list, revoke"
                    .to_string(),
                test_type: AuthTestType::ApiKeyLifecycle,
                expected_outcome: AuthTestOutcome::Success,
                test_data: json!({"role": "admin", "expires_days": 30}),
            },
            AuthTestScenario {
                name: "Rate Limiting".to_string(),
                description: "Test rate limiting with multiple failed authentication attempts"
                    .to_string(),
                test_type: AuthTestType::RateLimiting,
                expected_outcome: AuthTestOutcome::RateLimited,
                test_data: json!({"max_attempts": 5, "test_ip": "192.168.1.200"}),
            },
            AuthTestScenario {
                name: "Role-Based Access".to_string(),
                description: "Test role-based permission validation".to_string(),
                test_type: AuthTestType::RoleBasedAccess,
                expected_outcome: AuthTestOutcome::Success,
                test_data: json!({"roles": ["admin", "operator", "monitor"]}),
            },
            AuthTestScenario {
                name: "IP Whitelisting".to_string(),
                description: "Test IP address whitelisting functionality".to_string(),
                test_type: AuthTestType::IpWhitelisting,
                expected_outcome: AuthTestOutcome::PermissionDenied,
                test_data: json!({"allowed_ips": ["192.168.1.0/24"], "test_ip": "10.0.0.100"}),
            },
        ]
    }
}

/// Create default authentication integration tester
pub async fn create_auth_integration_tester(
    config: ValidationConfig,
) -> ValidationResult<AuthIntegrationTester> {
    AuthIntegrationTester::new(config)
}
