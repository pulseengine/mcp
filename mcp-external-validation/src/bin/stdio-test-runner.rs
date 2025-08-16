//! Stdio Integration Test Runner
//!
//! This binary runs the comprehensive stdio + MCP Inspector integration test suite.
//! It's designed to be used in CI/CD pipelines and for manual testing.

use clap::{Arg, Command};
use pulseengine_mcp_external_validation::{
    stdio_integration_tests::run_all_stdio_tests, stdio_stress_tests::run_all_stress_tests,
};
use tracing::{Level, error, info};
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("stdio-test-runner")
        .version("0.10.0")
        .about("Comprehensive stdio transport + MCP Inspector integration test runner")
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LEVEL")
                .help("Set the logging level")
                .value_parser(["error", "warn", "info", "debug", "trace"])
                .default_value("info"),
        )
        .arg(
            Arg::new("integration-only")
                .long("integration-only")
                .help("Run only integration tests, skip stress tests")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("stress-only")
                .long("stress-only")
                .help("Run only stress tests, skip integration tests")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("server-binary")
                .long("server-binary")
                .value_name("PATH")
                .help("Path to the timedate-mcp-server binary")
                .default_value("./target/release/timedate-mcp-server"),
        )
        .get_matches();

    // Setup logging
    let log_level = match matches.get_one::<String>("log-level").unwrap().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    fmt().with_max_level(log_level).with_target(false).init();

    info!("🚀 Starting Stdio Integration Test Runner v0.10.0");

    let integration_only = matches.get_flag("integration-only");
    let stress_only = matches.get_flag("stress-only");
    let server_binary = matches.get_one::<String>("server-binary").unwrap();

    info!("Configuration:");
    info!("  • Log level: {:?}", log_level);
    info!("  • Server binary: {}", server_binary);
    info!("  • Integration only: {}", integration_only);
    info!("  • Stress only: {}", stress_only);

    // Validate server binary exists
    let server_path = std::path::Path::new(server_binary);
    if !server_path.exists() {
        error!("❌ Server binary not found: {}", server_binary);
        error!(
            "Please build timedate-mcp-server first or specify the correct path with --server-binary"
        );
        std::process::exit(1);
    }

    info!("✅ Server binary found: {}", server_binary);

    let mut total_passed = 0;
    let mut total_failed = 0;

    // Run integration tests
    if !stress_only {
        info!("\n{}", "=".repeat(60));
        info!("🧪 RUNNING INTEGRATION TESTS");
        info!("{}", "=".repeat(60));

        match run_all_stdio_tests().await {
            Ok(_) => {
                info!("✅ Integration tests completed successfully");
                total_passed += 1;
            }
            Err(e) => {
                error!("❌ Integration tests failed: {}", e);
                total_failed += 1;
            }
        }
    }

    // Run stress tests
    if !integration_only {
        info!("\n{}", "=".repeat(60));
        info!("💪 RUNNING STRESS TESTS");
        info!("{}", "=".repeat(60));

        match run_all_stress_tests().await {
            Ok(_) => {
                info!("✅ Stress tests completed successfully");
                total_passed += 1;
            }
            Err(e) => {
                error!("❌ Stress tests failed: {}", e);
                total_failed += 1;
            }
        }
    }

    // Final summary
    info!("\n{}", "=".repeat(60));
    info!("📊 FINAL TEST SUMMARY");
    info!("{}", "=".repeat(60));
    info!("Test Suites Passed: {}", total_passed);
    info!("Test Suites Failed: {}", total_failed);

    if total_failed == 0 {
        info!("🎉 All stdio test suites passed!");
        info!("✅ Stdio transport + MCP Inspector integration is working correctly");
        Ok(())
    } else {
        error!("❌ {} test suite(s) failed", total_failed);
        std::process::exit(1);
    }
}
