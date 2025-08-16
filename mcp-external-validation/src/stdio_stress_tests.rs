//! Stress and performance tests for stdio transport
//!
//! This module provides intensive stress tests and performance benchmarks
//! for the stdio transport to ensure it can handle demanding scenarios.

use crate::{
    ValidationError, ValidationResult,
    stdio_integration_tests::{StdioTestConfig, StdioTestFixture, check_or_skip},
};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Performance test configuration
pub struct StressTestConfig {
    /// Number of rapid sequential tests
    pub rapid_test_count: usize,
    /// Delay between rapid tests
    pub rapid_test_delay: Duration,
    /// Duration for endurance test
    pub endurance_duration: Duration,
    /// Number of server restarts to test
    pub restart_cycles: usize,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            rapid_test_count: 10,
            rapid_test_delay: Duration::from_millis(100),
            endurance_duration: Duration::from_secs(60),
            restart_cycles: 5,
        }
    }
}

/// Performance metrics collected during testing
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Average connection time
    pub avg_connection_time: Duration,
    /// Maximum connection time observed
    pub max_connection_time: Duration,
    /// Minimum connection time observed
    pub min_connection_time: Duration,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Total tests run
    pub total_tests: usize,
    /// Number of successful tests
    pub successful_tests: usize,
    /// Server uptime during test
    pub total_uptime: Duration,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            avg_connection_time: Duration::ZERO,
            max_connection_time: Duration::ZERO,
            min_connection_time: Duration::MAX,
            success_rate: 0.0,
            total_tests: 0,
            successful_tests: 0,
            total_uptime: Duration::ZERO,
        }
    }

    fn update_connection_time(&mut self, time: Duration) {
        self.max_connection_time = self.max_connection_time.max(time);
        self.min_connection_time = self.min_connection_time.min(time);
    }

    fn calculate_averages(&mut self, connection_times: &[Duration]) {
        if !connection_times.is_empty() {
            let sum: Duration = connection_times.iter().sum();
            self.avg_connection_time = sum / connection_times.len() as u32;
        }

        self.success_rate = if self.total_tests > 0 {
            self.successful_tests as f64 / self.total_tests as f64
        } else {
            0.0
        };
    }
}

// =============================================================================
// RAPID SEQUENTIAL TESTS
// =============================================================================

/// Test rapid sequential server connections
#[tokio::test]
async fn test_stdio_rapid_sequential_connections() -> ValidationResult<()> {
    let fixture = StdioTestFixture::new().await?;
    if !check_or_skip(&fixture, "stdio_rapid_sequential_connections").await? {
        return Ok(());
    }

    info!("🚀 Testing rapid sequential stdio connections");

    let config = StressTestConfig::default();
    let mut metrics = PerformanceMetrics::new();
    let mut connection_times = Vec::new();

    for i in 0..config.rapid_test_count {
        debug!("Rapid test {}/{}", i + 1, config.rapid_test_count);

        let start_time = Instant::now();
        let result = fixture.test_server_with_inspector().await;
        let connection_time = start_time.elapsed();

        metrics.total_tests += 1;
        connection_times.push(connection_time);
        metrics.update_connection_time(connection_time);

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    metrics.successful_tests += 1;
                    debug!(
                        "✅ Rapid test {} successful in {:?}",
                        i + 1,
                        connection_time
                    );
                } else {
                    warn!("❌ Rapid test {} connection failed", i + 1);
                }
            }
            Err(e) => {
                warn!("❌ Rapid test {} failed: {}", i + 1, e);
            }
        }

        // Small delay between tests
        sleep(config.rapid_test_delay).await;
    }

    metrics.calculate_averages(&connection_times);

    info!("📊 Rapid Sequential Test Results:");
    info!("  • Total tests: {}", metrics.total_tests);
    info!("  • Successful: {}", metrics.successful_tests);
    info!("  • Success rate: {:.1}%", metrics.success_rate * 100.0);
    info!("  • Avg connection time: {:?}", metrics.avg_connection_time);
    info!("  • Min connection time: {:?}", metrics.min_connection_time);
    info!("  • Max connection time: {:?}", metrics.max_connection_time);

    // Require at least 80% success rate for rapid tests
    if metrics.success_rate < 0.8 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Rapid sequential test success rate too low: {:.1}% (expected >= 80%)",
                metrics.success_rate * 100.0
            ),
        });
    }

    info!("✅ Rapid sequential connection test passed");
    Ok(())
}

/// Test server restart resilience
#[tokio::test]
async fn test_stdio_server_restart_resilience() -> ValidationResult<()> {
    let fixture = StdioTestFixture::new().await?;
    if !check_or_skip(&fixture, "stdio_server_restart_resilience").await? {
        return Ok(());
    }

    info!("♻️  Testing stdio server restart resilience");

    let config = StressTestConfig::default();
    let mut successful_cycles = 0;

    for cycle in 0..config.restart_cycles {
        debug!("Restart cycle {}/{}", cycle + 1, config.restart_cycles);

        // Test server functionality
        let result = fixture.test_server_with_inspector().await;

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    successful_cycles += 1;
                    debug!("✅ Restart cycle {} successful", cycle + 1);
                } else {
                    warn!("❌ Restart cycle {} connection failed", cycle + 1);
                }
            }
            Err(e) => {
                warn!("❌ Restart cycle {} failed: {}", cycle + 1, e);
            }
        }

        // Short delay between cycles
        sleep(Duration::from_millis(500)).await;
    }

    let success_rate = successful_cycles as f64 / config.restart_cycles as f64;

    info!("📊 Server Restart Resilience Results:");
    info!("  • Total cycles: {}", config.restart_cycles);
    info!("  • Successful: {}", successful_cycles);
    info!("  • Success rate: {:.1}%", success_rate * 100.0);

    // Require at least 90% success rate for restart resilience
    if success_rate < 0.9 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Server restart resilience too low: {:.1}% (expected >= 90%)",
                success_rate * 100.0
            ),
        });
    }

    info!("✅ Server restart resilience test passed");
    Ok(())
}

// =============================================================================
// PERFORMANCE BENCHMARKS
// =============================================================================

/// Benchmark server startup time
#[tokio::test]
async fn test_stdio_server_startup_performance() -> ValidationResult<()> {
    let fixture = StdioTestFixture::new().await?;
    if !check_or_skip(&fixture, "stdio_server_startup_performance").await? {
        return Ok(());
    }

    info!("⏱️  Benchmarking stdio server startup performance");

    let mut startup_times = Vec::new();
    let benchmark_runs = 5;

    for run in 0..benchmark_runs {
        debug!("Startup benchmark run {}/{}", run + 1, benchmark_runs);

        let mut harness = fixture.create_server_harness();

        let start_time = Instant::now();
        let result = harness.start_server().await;
        let startup_time = start_time.elapsed();

        let _ = harness.stop_server().await;

        match result {
            Ok(_) => {
                startup_times.push(startup_time);
                debug!("✅ Startup run {} completed in {:?}", run + 1, startup_time);
            }
            Err(e) => {
                warn!("❌ Startup run {} failed: {}", run + 1, e);
            }
        }

        // Brief cooldown between runs
        sleep(Duration::from_millis(200)).await;
    }

    if startup_times.is_empty() {
        return Err(ValidationError::ValidationFailed {
            message: "No successful startup time measurements".to_string(),
        });
    }

    let avg_startup: Duration = startup_times.iter().sum::<Duration>() / startup_times.len() as u32;
    let min_startup = startup_times.iter().min().unwrap();
    let max_startup = startup_times.iter().max().unwrap();

    info!("📊 Server Startup Performance Results:");
    info!("  • Benchmark runs: {}", benchmark_runs);
    info!("  • Successful runs: {}", startup_times.len());
    info!("  • Average startup time: {:?}", avg_startup);
    info!("  • Fastest startup: {:?}", min_startup);
    info!("  • Slowest startup: {:?}", max_startup);

    // Startup should be reasonably fast (under 5 seconds)
    if avg_startup > Duration::from_secs(5) {
        warn!(
            "Server startup time is slower than expected: {:?}",
            avg_startup
        );
    }

    info!("✅ Server startup performance benchmark completed");
    Ok(())
}

/// Test connection time under load
#[tokio::test]
async fn test_stdio_connection_performance_under_load() -> ValidationResult<()> {
    let fixture = StdioTestFixture::new().await?;
    if !check_or_skip(&fixture, "stdio_connection_performance_under_load").await? {
        return Ok(());
    }

    info!("📈 Testing stdio connection performance under load");

    // Simulate load by running tests with minimal delays
    let load_test_count = 15;
    let mut connection_times = Vec::new();
    let mut successful_connections = 0;

    for i in 0..load_test_count {
        debug!("Load test {}/{}", i + 1, load_test_count);

        let start_time = Instant::now();
        let result = fixture.test_server_with_inspector().await;
        let connection_time = start_time.elapsed();

        connection_times.push(connection_time);

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    successful_connections += 1;
                    debug!("✅ Load test {} successful in {:?}", i + 1, connection_time);
                } else {
                    warn!("❌ Load test {} connection failed", i + 1);
                }
            }
            Err(e) => {
                warn!("❌ Load test {} failed: {}", i + 1, e);
            }
        }

        // Minimal delay to simulate load
        sleep(Duration::from_millis(50)).await;
    }

    let avg_connection: Duration =
        connection_times.iter().sum::<Duration>() / connection_times.len() as u32;
    let min_connection = connection_times.iter().min().unwrap();
    let max_connection = connection_times.iter().max().unwrap();
    let success_rate = successful_connections as f64 / load_test_count as f64;

    info!("📊 Connection Performance Under Load Results:");
    info!("  • Load tests: {}", load_test_count);
    info!("  • Successful connections: {}", successful_connections);
    info!("  • Success rate: {:.1}%", success_rate * 100.0);
    info!("  • Average connection time: {:?}", avg_connection);
    info!("  • Fastest connection: {:?}", min_connection);
    info!("  • Slowest connection: {:?}", max_connection);

    // Require reasonable performance under load
    if success_rate < 0.7 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Connection performance under load too poor: {:.1}% success rate",
                success_rate * 100.0
            ),
        });
    }

    info!("✅ Connection performance under load test passed");
    Ok(())
}

// =============================================================================
// ENDURANCE TESTS
// =============================================================================

/// Long-running endurance test
#[tokio::test]
async fn test_stdio_endurance() -> ValidationResult<()> {
    let fixture = StdioTestFixture::new().await?;
    if !check_or_skip(&fixture, "stdio_endurance").await? {
        return Ok(());
    }

    info!("🏃 Starting stdio endurance test");

    let config = StressTestConfig::default();
    let test_duration = Duration::from_secs(30); // Shortened for CI
    let test_interval = Duration::from_secs(2);

    let start_time = Instant::now();
    let mut test_count = 0;
    let mut successful_tests = 0;

    while start_time.elapsed() < test_duration {
        test_count += 1;
        debug!(
            "Endurance test iteration {} (elapsed: {:?})",
            test_count,
            start_time.elapsed()
        );

        let result = fixture.test_server_with_inspector().await;

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    successful_tests += 1;
                    debug!("✅ Endurance iteration {} successful", test_count);
                } else {
                    warn!("❌ Endurance iteration {} connection failed", test_count);
                }
            }
            Err(e) => {
                warn!("❌ Endurance iteration {} failed: {}", test_count, e);
            }
        }

        sleep(test_interval).await;
    }

    let total_duration = start_time.elapsed();
    let success_rate = if test_count > 0 {
        successful_tests as f64 / test_count as f64
    } else {
        0.0
    };

    info!("📊 Stdio Endurance Test Results:");
    info!("  • Total duration: {:?}", total_duration);
    info!("  • Test iterations: {}", test_count);
    info!("  • Successful iterations: {}", successful_tests);
    info!("  • Success rate: {:.1}%", success_rate * 100.0);
    info!(
        "  • Average interval: {:?}",
        total_duration / test_count as u32
    );

    // Require reasonable endurance performance
    if success_rate < 0.8 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Endurance test success rate too low: {:.1}% (expected >= 80%)",
                success_rate * 100.0
            ),
        });
    }

    if test_count < 5 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Too few endurance test iterations: {} (expected >= 5)",
                test_count
            ),
        });
    }

    info!("✅ Stdio endurance test passed");
    Ok(())
}

// =============================================================================
// RESOURCE STRESS TESTS
// =============================================================================

/// Test parameterized resource access under stress
#[tokio::test]
async fn test_stdio_parameterized_resource_stress() -> ValidationResult<()> {
    let fixture = StdioTestFixture::new().await?;
    if !check_or_skip(&fixture, "stdio_parameterized_resource_stress").await? {
        return Ok(());
    }

    info!("🎯 Testing parameterized resource access under stress");

    let stress_iterations = 8;
    let mut successful_resource_tests = 0;

    for i in 0..stress_iterations {
        debug!("Resource stress test {}/{}", i + 1, stress_iterations);

        let result = fixture.test_server_with_inspector().await;

        match result {
            Ok(inspector_result) => {
                // Check if resources are accessible (parameterized resource functionality)
                if inspector_result.resources_accessible {
                    successful_resource_tests += 1;
                    debug!("✅ Resource stress test {} - resources accessible", i + 1);
                } else {
                    warn!(
                        "❌ Resource stress test {} - resources not accessible",
                        i + 1
                    );
                }
            }
            Err(e) => {
                warn!("❌ Resource stress test {} failed: {}", i + 1, e);
            }
        }

        // Brief delay between resource tests
        sleep(Duration::from_millis(200)).await;
    }

    let resource_success_rate = successful_resource_tests as f64 / stress_iterations as f64;

    info!("📊 Parameterized Resource Stress Test Results:");
    info!("  • Stress iterations: {}", stress_iterations);
    info!(
        "  • Successful resource tests: {}",
        successful_resource_tests
    );
    info!(
        "  • Resource success rate: {:.1}%",
        resource_success_rate * 100.0
    );

    // Require high success rate for parameterized resources
    if resource_success_rate < 0.9 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Parameterized resource stress test success rate too low: {:.1}%",
                resource_success_rate * 100.0
            ),
        });
    }

    info!("✅ Parameterized resource stress test passed");
    Ok(())
}

// =============================================================================
// STRESS TEST RUNNER
// =============================================================================

// =============================================================================
// PUBLIC STRESS TEST FUNCTIONS (for programmatic use)
// =============================================================================

/// Test rapid sequential server connections
pub async fn stdio_rapid_sequential_connections_test() -> ValidationResult<()> {
    let fixture = crate::stdio_integration_tests::StdioTestFixture::new().await?;
    fixture.check_environment().await?;

    info!("🚀 Testing rapid sequential stdio connections");

    let config = StressTestConfig::default();
    let mut metrics = PerformanceMetrics::new();
    let mut connection_times = Vec::new();

    for i in 0..config.rapid_test_count {
        debug!("Rapid test {}/{}", i + 1, config.rapid_test_count);

        let start_time = Instant::now();
        let result = fixture.test_server_with_inspector().await;
        let connection_time = start_time.elapsed();

        metrics.total_tests += 1;
        connection_times.push(connection_time);
        metrics.update_connection_time(connection_time);

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    metrics.successful_tests += 1;
                    debug!(
                        "✅ Rapid test {} successful in {:?}",
                        i + 1,
                        connection_time
                    );
                } else {
                    warn!("❌ Rapid test {} connection failed", i + 1);
                }
            }
            Err(e) => {
                warn!("❌ Rapid test {} failed: {}", i + 1, e);
            }
        }

        // Small delay between tests
        sleep(config.rapid_test_delay).await;
    }

    metrics.calculate_averages(&connection_times);

    info!("📊 Rapid Sequential Test Results:");
    info!("  • Total tests: {}", metrics.total_tests);
    info!("  • Successful: {}", metrics.successful_tests);
    info!("  • Success rate: {:.1}%", metrics.success_rate * 100.0);
    info!("  • Avg connection time: {:?}", metrics.avg_connection_time);
    info!("  • Min connection time: {:?}", metrics.min_connection_time);
    info!("  • Max connection time: {:?}", metrics.max_connection_time);

    // Require at least 80% success rate for rapid tests
    if metrics.success_rate < 0.8 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Rapid sequential test success rate too low: {:.1}% (expected >= 80%)",
                metrics.success_rate * 100.0
            ),
        });
    }

    info!("✅ Rapid sequential connection test passed");
    Ok(())
}

/// Test server restart resilience
pub async fn stdio_server_restart_resilience_test() -> ValidationResult<()> {
    let fixture = crate::stdio_integration_tests::StdioTestFixture::new().await?;
    fixture.check_environment().await?;

    info!("♻️  Testing stdio server restart resilience");

    let config = StressTestConfig::default();
    let mut successful_cycles = 0;

    for cycle in 0..config.restart_cycles {
        debug!("Restart cycle {}/{}", cycle + 1, config.restart_cycles);

        // Test server functionality
        let result = fixture.test_server_with_inspector().await;

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    successful_cycles += 1;
                    debug!("✅ Restart cycle {} successful", cycle + 1);
                } else {
                    warn!("❌ Restart cycle {} connection failed", cycle + 1);
                }
            }
            Err(e) => {
                warn!("❌ Restart cycle {} failed: {}", cycle + 1, e);
            }
        }

        // Short delay between cycles
        sleep(Duration::from_millis(500)).await;
    }

    let success_rate = successful_cycles as f64 / config.restart_cycles as f64;

    info!("📊 Server Restart Resilience Results:");
    info!("  • Total cycles: {}", config.restart_cycles);
    info!("  • Successful: {}", successful_cycles);
    info!("  • Success rate: {:.1}%", success_rate * 100.0);

    // Require at least 90% success rate for restart resilience
    if success_rate < 0.9 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Server restart resilience too low: {:.1}% (expected >= 90%)",
                success_rate * 100.0
            ),
        });
    }

    info!("✅ Server restart resilience test passed");
    Ok(())
}

/// Benchmark server startup time
pub async fn stdio_server_startup_performance_test() -> ValidationResult<()> {
    let fixture = crate::stdio_integration_tests::StdioTestFixture::new().await?;
    fixture.check_environment().await?;

    info!("⏱️  Benchmarking stdio server startup performance");

    let mut startup_times = Vec::new();
    let benchmark_runs = 5;

    for run in 0..benchmark_runs {
        debug!("Startup benchmark run {}/{}", run + 1, benchmark_runs);

        let mut harness = fixture.create_server_harness();

        let start_time = Instant::now();
        let result = harness.start_server().await;
        let startup_time = start_time.elapsed();

        let _ = harness.stop_server().await;

        match result {
            Ok(_) => {
                startup_times.push(startup_time);
                debug!("✅ Startup run {} completed in {:?}", run + 1, startup_time);
            }
            Err(e) => {
                warn!("❌ Startup run {} failed: {}", run + 1, e);
            }
        }

        // Brief cooldown between runs
        sleep(Duration::from_millis(200)).await;
    }

    if startup_times.is_empty() {
        return Err(ValidationError::ValidationFailed {
            message: "No successful startup time measurements".to_string(),
        });
    }

    let avg_startup: Duration = startup_times.iter().sum::<Duration>() / startup_times.len() as u32;
    let min_startup = startup_times.iter().min().unwrap();
    let max_startup = startup_times.iter().max().unwrap();

    info!("📊 Server Startup Performance Results:");
    info!("  • Benchmark runs: {}", benchmark_runs);
    info!("  • Successful runs: {}", startup_times.len());
    info!("  • Average startup time: {:?}", avg_startup);
    info!("  • Fastest startup: {:?}", min_startup);
    info!("  • Slowest startup: {:?}", max_startup);

    // Startup should be reasonably fast (under 5 seconds)
    if avg_startup > Duration::from_secs(5) {
        warn!(
            "Server startup time is slower than expected: {:?}",
            avg_startup
        );
    }

    info!("✅ Server startup performance benchmark completed");
    Ok(())
}

/// Test connection time under load
pub async fn stdio_connection_performance_under_load_test() -> ValidationResult<()> {
    let fixture = crate::stdio_integration_tests::StdioTestFixture::new().await?;
    fixture.check_environment().await?;

    info!("📈 Testing stdio connection performance under load");

    // Simulate load by running tests with minimal delays
    let load_test_count = 15;
    let mut connection_times = Vec::new();
    let mut successful_connections = 0;

    for i in 0..load_test_count {
        debug!("Load test {}/{}", i + 1, load_test_count);

        let start_time = Instant::now();
        let result = fixture.test_server_with_inspector().await;
        let connection_time = start_time.elapsed();

        connection_times.push(connection_time);

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    successful_connections += 1;
                    debug!("✅ Load test {} successful in {:?}", i + 1, connection_time);
                } else {
                    warn!("❌ Load test {} connection failed", i + 1);
                }
            }
            Err(e) => {
                warn!("❌ Load test {} failed: {}", i + 1, e);
            }
        }

        // Minimal delay to simulate load
        sleep(Duration::from_millis(50)).await;
    }

    let avg_connection: Duration =
        connection_times.iter().sum::<Duration>() / connection_times.len() as u32;
    let min_connection = connection_times.iter().min().unwrap();
    let max_connection = connection_times.iter().max().unwrap();
    let success_rate = successful_connections as f64 / load_test_count as f64;

    info!("📊 Connection Performance Under Load Results:");
    info!("  • Load tests: {}", load_test_count);
    info!("  • Successful connections: {}", successful_connections);
    info!("  • Success rate: {:.1}%", success_rate * 100.0);
    info!("  • Average connection time: {:?}", avg_connection);
    info!("  • Fastest connection: {:?}", min_connection);
    info!("  • Slowest connection: {:?}", max_connection);

    // Require reasonable performance under load
    if success_rate < 0.7 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Connection performance under load too poor: {:.1}% success rate",
                success_rate * 100.0
            ),
        });
    }

    info!("✅ Connection performance under load test passed");
    Ok(())
}

/// Long-running endurance test
pub async fn stdio_endurance_test() -> ValidationResult<()> {
    let fixture = crate::stdio_integration_tests::StdioTestFixture::new().await?;
    fixture.check_environment().await?;

    info!("🏃 Starting stdio endurance test");

    let test_duration = Duration::from_secs(30); // Shortened for CI
    let test_interval = Duration::from_secs(2);

    let start_time = Instant::now();
    let mut test_count = 0;
    let mut successful_tests = 0;

    while start_time.elapsed() < test_duration {
        test_count += 1;
        debug!(
            "Endurance test iteration {} (elapsed: {:?})",
            test_count,
            start_time.elapsed()
        );

        let result = fixture.test_server_with_inspector().await;

        match result {
            Ok(inspector_result) => {
                if inspector_result.connection_success {
                    successful_tests += 1;
                    debug!("✅ Endurance iteration {} successful", test_count);
                } else {
                    warn!("❌ Endurance iteration {} connection failed", test_count);
                }
            }
            Err(e) => {
                warn!("❌ Endurance iteration {} failed: {}", test_count, e);
            }
        }

        sleep(test_interval).await;
    }

    let total_duration = start_time.elapsed();
    let success_rate = if test_count > 0 {
        successful_tests as f64 / test_count as f64
    } else {
        0.0
    };

    info!("📊 Stdio Endurance Test Results:");
    info!("  • Total duration: {:?}", total_duration);
    info!("  • Test iterations: {}", test_count);
    info!("  • Successful iterations: {}", successful_tests);
    info!("  • Success rate: {:.1}%", success_rate * 100.0);
    info!(
        "  • Average interval: {:?}",
        total_duration / test_count as u32
    );

    // Require reasonable endurance performance
    if success_rate < 0.8 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Endurance test success rate too low: {:.1}% (expected >= 80%)",
                success_rate * 100.0
            ),
        });
    }

    if test_count < 5 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Too few endurance test iterations: {} (expected >= 5)",
                test_count
            ),
        });
    }

    info!("✅ Stdio endurance test passed");
    Ok(())
}

/// Test parameterized resource access under stress
pub async fn stdio_parameterized_resource_stress_test() -> ValidationResult<()> {
    let fixture = crate::stdio_integration_tests::StdioTestFixture::new().await?;
    fixture.check_environment().await?;

    info!("🎯 Testing parameterized resource access under stress");

    let stress_iterations = 8;
    let mut successful_resource_tests = 0;

    for i in 0..stress_iterations {
        debug!("Resource stress test {}/{}", i + 1, stress_iterations);

        let result = fixture.test_server_with_inspector().await;

        match result {
            Ok(inspector_result) => {
                // Check if resources are accessible (parameterized resource functionality)
                if inspector_result.resources_accessible {
                    successful_resource_tests += 1;
                    debug!("✅ Resource stress test {} - resources accessible", i + 1);
                } else {
                    warn!(
                        "❌ Resource stress test {} - resources not accessible",
                        i + 1
                    );
                }
            }
            Err(e) => {
                warn!("❌ Resource stress test {} failed: {}", i + 1, e);
            }
        }

        // Brief delay between resource tests
        sleep(Duration::from_millis(200)).await;
    }

    let resource_success_rate = successful_resource_tests as f64 / stress_iterations as f64;

    info!("📊 Parameterized Resource Stress Test Results:");
    info!("  • Stress iterations: {}", stress_iterations);
    info!(
        "  • Successful resource tests: {}",
        successful_resource_tests
    );
    info!(
        "  • Resource success rate: {:.1}%",
        resource_success_rate * 100.0
    );

    // Require high success rate for parameterized resources
    if resource_success_rate < 0.9 {
        return Err(ValidationError::ValidationFailed {
            message: format!(
                "Parameterized resource stress test success rate too low: {:.1}%",
                resource_success_rate * 100.0
            ),
        });
    }

    info!("✅ Parameterized resource stress test passed");
    Ok(())
}

/// Run all stdio stress and performance tests
pub async fn run_all_stress_tests() -> ValidationResult<()> {
    info!("🚀 Running comprehensive stdio stress and performance tests");

    let mut passed = 0;
    let mut failed = 0;

    // Run stress tests individually using match to avoid async future issues
    let tests = vec![
        "Rapid Sequential Connections",
        "Server Restart Resilience",
        "Server Startup Performance",
        "Connection Performance Under Load",
        "Endurance Test",
        "Parameterized Resource Stress",
    ];

    for name in tests {
        info!("Stress Test: {}", name);

        let result = match name {
            "Rapid Sequential Connections" => stdio_rapid_sequential_connections_test().await,
            "Server Restart Resilience" => stdio_server_restart_resilience_test().await,
            "Server Startup Performance" => stdio_server_startup_performance_test().await,
            "Connection Performance Under Load" => {
                stdio_connection_performance_under_load_test().await
            }
            "Endurance Test" => stdio_endurance_test().await,
            "Parameterized Resource Stress" => stdio_parameterized_resource_stress_test().await,
            _ => Ok(()),
        };

        match result {
            Ok(_) => {
                info!("✅ {} - PASSED", name);
                passed += 1;
            }
            Err(e) => {
                warn!("❌ {} - FAILED: {}", name, e);
                failed += 1;
            }
        }
    }

    info!(
        "📊 Stress Test Results: {} passed, {} failed",
        passed, failed
    );

    if failed == 0 {
        info!("🎉 All stdio stress tests passed!");
        Ok(())
    } else {
        Err(ValidationError::ValidationFailed {
            message: format!("{} stdio stress tests failed", failed),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();

        metrics.total_tests = 10;
        metrics.successful_tests = 8;

        let times = vec![
            Duration::from_millis(100),
            Duration::from_millis(150),
            Duration::from_millis(120),
        ];

        for &time in &times {
            metrics.update_connection_time(time);
        }

        metrics.calculate_averages(&times);

        assert_eq!(metrics.success_rate, 0.8);
        assert_eq!(metrics.min_connection_time, Duration::from_millis(100));
        assert_eq!(metrics.max_connection_time, Duration::from_millis(150));
    }

    #[test]
    fn test_stress_test_config() {
        let config = StressTestConfig::default();
        assert!(config.rapid_test_count > 0);
        assert!(config.restart_cycles > 0);
        assert!(config.endurance_duration > Duration::ZERO);
    }
}
