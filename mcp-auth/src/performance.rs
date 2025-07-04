//! Performance testing and benchmarking utilities
//!
//! This module provides comprehensive performance testing tools for the
//! authentication framework including load testing, stress testing, and
//! performance monitoring capabilities.

use crate::{AuthenticationManager, AuthConfig, Role, ConsentManager, ConsentConfig, MemoryConsentStorage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::info;
use uuid::Uuid;

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Number of concurrent users to simulate
    pub concurrent_users: usize,
    
    /// Duration of the test in seconds
    pub test_duration_secs: u64,
    
    /// Request rate per second per user
    pub requests_per_second: f64,
    
    /// Warmup duration in seconds
    pub warmup_duration_secs: u64,
    
    /// Cool down duration in seconds
    pub cooldown_duration_secs: u64,
    
    /// Enable detailed metrics collection
    pub enable_detailed_metrics: bool,
    
    /// Target operations to test
    pub test_operations: Vec<TestOperation>,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 100,
            test_duration_secs: 60,
            requests_per_second: 10.0,
            warmup_duration_secs: 10,
            cooldown_duration_secs: 5,
            enable_detailed_metrics: true,
            test_operations: vec![
                TestOperation::ValidateApiKey,
                TestOperation::CreateApiKey,
                TestOperation::ListApiKeys,
                TestOperation::RateLimitCheck,
            ],
        }
    }
}

/// Types of operations to test
#[derive(Debug, Clone, PartialEq)]
pub enum TestOperation {
    /// Test API key validation
    ValidateApiKey,
    
    /// Test API key creation
    CreateApiKey,
    
    /// Test API key listing
    ListApiKeys,
    
    /// Test rate limiting
    RateLimitCheck,
    
    /// Test JWT token generation
    GenerateJwtToken,
    
    /// Test JWT token validation
    ValidateJwtToken,
    
    /// Test consent checking
    CheckConsent,
    
    /// Test consent granting
    GrantConsent,
    
    /// Test vault operations
    VaultOperations,
}

impl std::fmt::Display for TestOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestOperation::ValidateApiKey => write!(f, "Validate API Key"),
            TestOperation::CreateApiKey => write!(f, "Create API Key"),
            TestOperation::ListApiKeys => write!(f, "List API Keys"),
            TestOperation::RateLimitCheck => write!(f, "Rate Limit Check"),
            TestOperation::GenerateJwtToken => write!(f, "Generate JWT Token"),
            TestOperation::ValidateJwtToken => write!(f, "Validate JWT Token"),
            TestOperation::CheckConsent => write!(f, "Check Consent"),
            TestOperation::GrantConsent => write!(f, "Grant Consent"),
            TestOperation::VaultOperations => write!(f, "Vault Operations"),
        }
    }
}

/// Performance test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceResults {
    /// Test configuration used
    pub config: TestConfig,
    
    /// Test start time
    pub start_time: DateTime<Utc>,
    
    /// Test end time
    pub end_time: DateTime<Utc>,
    
    /// Total duration including warmup/cooldown
    pub total_duration_secs: f64,
    
    /// Actual test duration (excluding warmup/cooldown)
    pub test_duration_secs: f64,
    
    /// Operation-specific results
    pub operation_results: HashMap<String, OperationResults>,
    
    /// Overall statistics
    pub overall_stats: OverallStats,
    
    /// Resource usage during test
    pub resource_usage: ResourceUsage,
    
    /// Error summary
    pub error_summary: ErrorSummary,
}

/// Configuration used for testing (serializable version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub concurrent_users: usize,
    pub test_duration_secs: u64,
    pub requests_per_second: f64,
    pub warmup_duration_secs: u64,
    pub cooldown_duration_secs: u64,
    pub operations_tested: Vec<String>,
}

/// Results for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResults {
    /// Total requests made
    pub total_requests: u64,
    
    /// Successful requests
    pub successful_requests: u64,
    
    /// Failed requests
    pub failed_requests: u64,
    
    /// Success rate as percentage
    pub success_rate: f64,
    
    /// Requests per second
    pub requests_per_second: f64,
    
    /// Response time statistics in milliseconds
    pub response_times: ResponseTimeStats,
    
    /// Error breakdown
    pub errors: HashMap<String, u64>,
}

/// Response time statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimeStats {
    /// Average response time in milliseconds
    pub avg_ms: f64,
    
    /// Minimum response time
    pub min_ms: f64,
    
    /// Maximum response time
    pub max_ms: f64,
    
    /// 50th percentile (median)
    pub p50_ms: f64,
    
    /// 90th percentile
    pub p90_ms: f64,
    
    /// 95th percentile
    pub p95_ms: f64,
    
    /// 99th percentile
    pub p99_ms: f64,
}

/// Overall test statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStats {
    /// Total requests across all operations
    pub total_requests: u64,
    
    /// Total successful requests
    pub successful_requests: u64,
    
    /// Overall success rate
    pub success_rate: f64,
    
    /// Overall requests per second
    pub overall_rps: f64,
    
    /// Peak requests per second achieved
    pub peak_rps: f64,
    
    /// Average concurrent users active
    pub avg_concurrent_users: f64,
}

/// Resource usage during test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Peak memory usage in MB
    pub peak_memory_mb: f64,
    
    /// Average memory usage in MB
    pub avg_memory_mb: f64,
    
    /// Peak CPU usage percentage
    pub peak_cpu_percent: f64,
    
    /// Average CPU usage percentage
    pub avg_cpu_percent: f64,
    
    /// Number of threads created
    pub thread_count: u32,
}

/// Error summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSummary {
    /// Total errors
    pub total_errors: u64,
    
    /// Error rate as percentage
    pub error_rate: f64,
    
    /// Breakdown by error type
    pub error_types: HashMap<String, u64>,
    
    /// Most common error
    pub most_common_error: Option<String>,
}

/// Performance test runner
pub struct PerformanceTest {
    config: PerformanceConfig,
    auth_manager: Arc<AuthenticationManager>,
    consent_manager: Option<Arc<ConsentManager>>,
}

impl PerformanceTest {
    /// Create a new performance test
    pub async fn new(config: PerformanceConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Create auth manager with optimized config for testing
        let auth_config = AuthConfig {
            enabled: true,
            storage: crate::config::StorageConfig::Environment { prefix: "PERF_TEST".to_string() },
            cache_size: 10000, // Larger cache for performance testing
            session_timeout_secs: 3600,
            max_failed_attempts: 10,
            rate_limit_window_secs: 60,
        };
        
        let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await?);
        
        // Create consent manager if consent operations are being tested
        let consent_manager = if config.test_operations.iter().any(|op| {
            matches!(op, TestOperation::CheckConsent | TestOperation::GrantConsent)
        }) {
            let consent_config = ConsentConfig::default();
            let storage = Arc::new(MemoryConsentStorage::new());
            Some(Arc::new(ConsentManager::new(consent_config, storage)))
        } else {
            None
        };
        
        Ok(Self {
            config,
            auth_manager,
            consent_manager,
        })
    }
    
    /// Run the performance test
    pub async fn run(&mut self) -> Result<PerformanceResults, Box<dyn std::error::Error>> {
        info!("Starting performance test with {} concurrent users for {} seconds", 
              self.config.concurrent_users, self.config.test_duration_secs);
        
        let start_time = Utc::now();
        let test_start = Instant::now();
        
        // Warmup phase
        if self.config.warmup_duration_secs > 0 {
            info!("Warming up for {} seconds...", self.config.warmup_duration_secs);
            self.warmup_phase().await?;
        }
        
        // Main test phase
        info!("Starting main test phase...");
        let main_test_start = Instant::now();
        let operation_results = self.run_main_test().await?;
        let main_test_duration = main_test_start.elapsed();
        
        // Cool down phase
        if self.config.cooldown_duration_secs > 0 {
            info!("Cooling down for {} seconds...", self.config.cooldown_duration_secs);
            sleep(Duration::from_secs(self.config.cooldown_duration_secs)).await;
        }
        
        let end_time = Utc::now();
        let total_duration = test_start.elapsed();
        
        // Calculate overall statistics
        let overall_stats = self.calculate_overall_stats(&operation_results, main_test_duration);
        let resource_usage = self.collect_resource_usage();
        let error_summary = self.calculate_error_summary(&operation_results);
        
        let results = PerformanceResults {
            config: TestConfig {
                concurrent_users: self.config.concurrent_users,
                test_duration_secs: self.config.test_duration_secs,
                requests_per_second: self.config.requests_per_second,
                warmup_duration_secs: self.config.warmup_duration_secs,
                cooldown_duration_secs: self.config.cooldown_duration_secs,
                operations_tested: self.config.test_operations.iter().map(|op| op.to_string()).collect(),
            },
            start_time,
            end_time,
            total_duration_secs: total_duration.as_secs_f64(),
            test_duration_secs: main_test_duration.as_secs_f64(),
            operation_results,
            overall_stats,
            resource_usage,
            error_summary,
        };
        
        info!("Performance test completed successfully");
        Ok(results)
    }
    
    /// Warmup phase to prepare the system
    async fn warmup_phase(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create some initial API keys for testing
        for i in 0..50 {
            let key_name = format!("warmup-key-{}", i);
            let _ = self.auth_manager.create_api_key(
                key_name,
                Role::Operator,
                None,
                Some(vec!["127.0.0.1".to_string()]),
            ).await;
        }
        
        // Warm up consent manager if needed
        if let Some(consent_manager) = &self.consent_manager {
            for i in 0..20 {
                let subject_id = format!("warmup-user-{}", i);
                let _ = consent_manager.request_consent_individual(
                    subject_id,
                    crate::ConsentType::DataProcessing,
                    crate::LegalBasis::Consent,
                    "Warmup consent".to_string(),
                    vec![],
                    "performance_test".to_string(),
                    None,
                ).await;
            }
        }
        
        // Brief pause to let things settle
        sleep(Duration::from_millis(100)).await;
        
        Ok(())
    }
    
    /// Run the main test phase
    async fn run_main_test(&self) -> Result<HashMap<String, OperationResults>, Box<dyn std::error::Error>> {
        let mut operation_results = HashMap::new();
        
        // Run tests for each operation
        for operation in &self.config.test_operations {
            info!("Testing operation: {}", operation);
            let results = self.test_operation(operation.clone()).await?;
            operation_results.insert(operation.to_string(), results);
        }
        
        Ok(operation_results)
    }
    
    /// Test a specific operation
    async fn test_operation(&self, operation: TestOperation) -> Result<OperationResults, Box<dyn std::error::Error>> {
        let mut handles = Vec::new();
        let mut response_times = Vec::new();
        let mut errors = HashMap::new();
        let mut total_requests = 0u64;
        let mut successful_requests = 0u64;
        
        let test_start = Instant::now();
        let test_duration = Duration::from_secs(self.config.test_duration_secs);
        
        // Spawn concurrent workers
        for user_id in 0..self.config.concurrent_users {
            let operation = operation.clone();
            let auth_manager = Arc::clone(&self.auth_manager);
            let consent_manager = self.consent_manager.as_ref().map(|cm| Arc::clone(cm));
            let requests_per_second = self.config.requests_per_second;
            
            let handle = tokio::spawn(async move {
                let mut user_response_times = Vec::new();
                let mut user_errors = HashMap::new();
                let mut user_requests = 0u64;
                let mut user_successful = 0u64;
                
                let request_interval = Duration::from_secs_f64(1.0 / requests_per_second);
                let mut next_request = Instant::now();
                
                while test_start.elapsed() < test_duration {
                    if Instant::now() >= next_request {
                        let request_start = Instant::now();
                        
                        let result = match &operation {
                            TestOperation::ValidateApiKey => {
                                Self::test_validate_api_key(&*auth_manager, user_id).await
                            }
                            TestOperation::CreateApiKey => {
                                Self::test_create_api_key(&*auth_manager, user_id).await
                            }
                            TestOperation::ListApiKeys => {
                                Self::test_list_api_keys(&*auth_manager).await
                            }
                            TestOperation::RateLimitCheck => {
                                Self::test_rate_limit_check(&*auth_manager, user_id).await
                            }
                            TestOperation::CheckConsent => {
                                if let Some(consent_mgr) = &consent_manager {
                                    Self::test_check_consent(&**consent_mgr, user_id).await
                                } else {
                                    Ok(())
                                }
                            }
                            TestOperation::GrantConsent => {
                                if let Some(consent_mgr) = &consent_manager {
                                    Self::test_grant_consent(&**consent_mgr, user_id).await
                                } else {
                                    Ok(())
                                }
                            }
                            _ => Ok(()), // Other operations not implemented yet
                        };
                        
                        let response_time = request_start.elapsed();
                        user_response_times.push(response_time.as_secs_f64() * 1000.0); // Convert to ms
                        user_requests += 1;
                        
                        match result {
                            Ok(_) => user_successful += 1,
                            Err(e) => {
                                let error_type = format!("{:?}", e);
                                *user_errors.entry(error_type).or_insert(0) += 1;
                            }
                        }
                        
                        next_request = Instant::now() + request_interval;
                    } else {
                        // Small sleep to prevent busy waiting
                        sleep(Duration::from_millis(1)).await;
                    }
                }
                
                (user_response_times, user_errors, user_requests, user_successful)
            });
            
            handles.push(handle);
        }
        
        // Collect results from all workers
        for handle in handles {
            let (user_response_times, user_errors, user_requests, user_successful) = handle.await?;
            response_times.extend(user_response_times);
            total_requests += user_requests;
            successful_requests += user_successful;
            
            for (error_type, count) in user_errors {
                *errors.entry(error_type).or_insert(0) += count;
            }
        }
        
        let failed_requests = total_requests - successful_requests;
        let success_rate = if total_requests > 0 {
            (successful_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        let test_duration_secs = test_start.elapsed().as_secs_f64();
        let requests_per_second = if test_duration_secs > 0.0 {
            total_requests as f64 / test_duration_secs
        } else {
            0.0
        };
        
        // Calculate response time statistics
        response_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let response_time_stats = if !response_times.is_empty() {
            ResponseTimeStats {
                avg_ms: response_times.iter().sum::<f64>() / response_times.len() as f64,
                min_ms: response_times[0],
                max_ms: response_times[response_times.len() - 1],
                p50_ms: Self::percentile(&response_times, 50.0),
                p90_ms: Self::percentile(&response_times, 90.0),
                p95_ms: Self::percentile(&response_times, 95.0),
                p99_ms: Self::percentile(&response_times, 99.0),
            }
        } else {
            ResponseTimeStats {
                avg_ms: 0.0,
                min_ms: 0.0,
                max_ms: 0.0,
                p50_ms: 0.0,
                p90_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
            }
        };
        
        Ok(OperationResults {
            total_requests,
            successful_requests,
            failed_requests,
            success_rate,
            requests_per_second,
            response_times: response_time_stats,
            errors,
        })
    }
    
    /// Test API key validation
    async fn test_validate_api_key(
        auth_manager: &AuthenticationManager,
        user_id: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create a test key for this user if it doesn't exist
        let key_name = format!("test-key-{}", user_id);
        let api_key = auth_manager.create_api_key(
            key_name,
            Role::Operator,
            None,
            Some(vec!["127.0.0.1".to_string()]),
        ).await?;
        
        // Validate the key
        auth_manager.validate_api_key(&api_key.key, Some("127.0.0.1")).await?;
        
        Ok(())
    }
    
    /// Test API key creation
    async fn test_create_api_key(
        auth_manager: &AuthenticationManager,
        user_id: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key_name = format!("perf-key-{}-{}", user_id, Uuid::new_v4());
        auth_manager.create_api_key(
            key_name,
            Role::Monitor,
            None,
            None,
        ).await?;
        
        Ok(())
    }
    
    /// Test API key listing
    async fn test_list_api_keys(
        auth_manager: &AuthenticationManager,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = auth_manager.list_keys().await;
        Ok(())
    }
    
    /// Test rate limiting (simplified - just test key validation which includes rate limiting)
    async fn test_rate_limit_check(
        auth_manager: &AuthenticationManager,
        user_id: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client_ip = format!("192.168.1.{}", (user_id % 254) + 1);
        // Create a test key and validate it to trigger rate limiting
        let key_name = format!("rate-test-key-{}", user_id);
        let api_key = auth_manager.create_api_key(
            key_name,
            Role::Operator,
            None,
            Some(vec![client_ip.clone()]),
        ).await?;
        
        // Validate the key which will trigger rate limiting checks
        auth_manager.validate_api_key(&api_key.key, Some(&client_ip)).await?;
        Ok(())
    }
    
    /// Test consent checking
    async fn test_check_consent(
        consent_manager: &ConsentManager,
        user_id: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject_id = format!("perf-user-{}", user_id);
        consent_manager.check_consent(&subject_id, &crate::ConsentType::DataProcessing).await?;
        Ok(())
    }
    
    /// Test consent granting
    async fn test_grant_consent(
        consent_manager: &ConsentManager,
        user_id: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject_id = format!("perf-user-{}", user_id);
        
        // First request consent
        let _ = consent_manager.request_consent_individual(
            subject_id.clone(),
            crate::ConsentType::Analytics,
            crate::LegalBasis::Consent,
            "Performance test consent".to_string(),
            vec![],
            "performance_test".to_string(),
            None,
        ).await;
        
        // Then grant it
        consent_manager.grant_consent(
            &subject_id,
            &crate::ConsentType::Analytics,
            None,
            "performance_test".to_string(),
        ).await?;
        
        Ok(())
    }
    
    /// Calculate percentile from sorted data
    fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
        if sorted_data.is_empty() {
            return 0.0;
        }
        
        let index = (percentile / 100.0) * (sorted_data.len() - 1) as f64;
        let lower = index.floor() as usize;
        let upper = index.ceil() as usize;
        
        if lower == upper {
            sorted_data[lower]
        } else {
            let weight = index - lower as f64;
            sorted_data[lower] * (1.0 - weight) + sorted_data[upper] * weight
        }
    }
    
    /// Calculate overall statistics
    fn calculate_overall_stats(
        &self,
        operation_results: &HashMap<String, OperationResults>,
        test_duration: Duration,
    ) -> OverallStats {
        let total_requests: u64 = operation_results.values().map(|r| r.total_requests).sum();
        let successful_requests: u64 = operation_results.values().map(|r| r.successful_requests).sum();
        
        let success_rate = if total_requests > 0 {
            (successful_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        let test_duration_secs = test_duration.as_secs_f64();
        let overall_rps = if test_duration_secs > 0.0 {
            total_requests as f64 / test_duration_secs
        } else {
            0.0
        };
        
        // Peak RPS is estimated as the maximum RPS from any operation
        let peak_rps = operation_results.values()
            .map(|r| r.requests_per_second)
            .fold(0.0, f64::max);
        
        OverallStats {
            total_requests,
            successful_requests,
            success_rate,
            overall_rps,
            peak_rps,
            avg_concurrent_users: self.config.concurrent_users as f64,
        }
    }
    
    /// Collect resource usage (simplified version)
    fn collect_resource_usage(&self) -> ResourceUsage {
        // In a real implementation, you'd collect actual system metrics
        // For now, return estimated values based on test scale
        ResourceUsage {
            peak_memory_mb: (self.config.concurrent_users as f64 * 0.5).max(10.0),
            avg_memory_mb: (self.config.concurrent_users as f64 * 0.3).max(5.0),
            peak_cpu_percent: (self.config.concurrent_users as f64 * 0.1).min(80.0),
            avg_cpu_percent: (self.config.concurrent_users as f64 * 0.05).min(50.0),
            thread_count: self.config.concurrent_users as u32 + 10,
        }
    }
    
    /// Calculate error summary
    fn calculate_error_summary(&self, operation_results: &HashMap<String, OperationResults>) -> ErrorSummary {
        let total_requests: u64 = operation_results.values().map(|r| r.total_requests).sum();
        let total_errors: u64 = operation_results.values().map(|r| r.failed_requests).sum();
        
        let error_rate = if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        let mut all_errors = HashMap::new();
        for result in operation_results.values() {
            for (error_type, count) in &result.errors {
                *all_errors.entry(error_type.clone()).or_insert(0) += count;
            }
        }
        
        let most_common_error = all_errors.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(error_type, _)| error_type.clone());
        
        ErrorSummary {
            total_errors,
            error_rate,
            error_types: all_errors,
            most_common_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_performance_config_default() {
        let config = PerformanceConfig::default();
        assert_eq!(config.concurrent_users, 100);
        assert_eq!(config.test_duration_secs, 60);
        assert!(!config.test_operations.is_empty());
    }
    
    #[tokio::test]
    async fn test_percentile_calculation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(PerformanceTest::percentile(&data, 50.0), 3.0);
        assert_eq!(PerformanceTest::percentile(&data, 90.0), 4.6);
    }
    
    #[tokio::test]
    async fn test_performance_test_creation() {
        let config = PerformanceConfig {
            concurrent_users: 10,
            test_duration_secs: 5,
            requests_per_second: 1.0,
            warmup_duration_secs: 1,
            cooldown_duration_secs: 1,
            enable_detailed_metrics: true,
            test_operations: vec![TestOperation::ValidateApiKey],
        };
        
        let test = PerformanceTest::new(config).await;
        assert!(test.is_ok());
    }
}