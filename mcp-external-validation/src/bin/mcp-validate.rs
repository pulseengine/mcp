//! Command-line tool for validating MCP servers

use clap::Parser;
use pulseengine_mcp_external_validation::{ExternalValidator, ValidationConfig};
use std::process;
use tracing::{error, info, warn, Level};

#[derive(Parser)]
#[command(name = "mcp-validate")]
#[command(about = "Validate MCP server compliance using external validators")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Server URL to validate
    #[arg(long, short)]
    server_url: String,

    /// Protocol version to test
    #[arg(long, default_value = "2025-03-26")]
    protocol_version: String,

    /// Configuration file path
    #[arg(long, short)]
    config: Option<String>,

    /// Output format (text, json, yaml)
    #[arg(long, default_value = "text")]
    output: String,

    /// Verbose output
    #[arg(long, short)]
    verbose: bool,

    /// Quick validation (subset of tests)
    #[arg(long, short)]
    quick: bool,

    /// Skip MCP validator tests
    #[arg(long)]
    skip_mcp_validator: bool,

    /// Skip JSON-RPC validation
    #[arg(long)]
    skip_jsonrpc: bool,

    /// Skip Inspector tests
    #[arg(long)]
    skip_inspector: bool,

    /// Run benchmark tests
    #[arg(long)]
    benchmark: bool,

    /// Number of benchmark iterations
    #[arg(long, default_value = "10")]
    benchmark_iterations: u32,

    /// Exit with error code if validation fails
    #[arg(long)]
    strict: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Load configuration
    let config = match load_config(&cli).await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    // Create validator
    let validator = match ExternalValidator::with_config(config).await {
        Ok(validator) => validator,
        Err(e) => {
            error!("Failed to create validator: {}", e);
            process::exit(1);
        }
    };

    // Run validation or benchmark
    let exit_code = if cli.benchmark {
        run_benchmark(&validator, &cli).await
    } else if cli.quick {
        run_quick_validation(&validator, &cli).await
    } else {
        run_full_validation(&validator, &cli).await
    };

    process::exit(exit_code);
}

async fn load_config(cli: &Cli) -> Result<ValidationConfig, Box<dyn std::error::Error>> {
    let mut config = if let Some(ref config_path) = cli.config {
        ValidationConfig::from_file(config_path)?
    } else {
        ValidationConfig::from_env()?
    };

    // Override with CLI arguments
    if cli.skip_mcp_validator {
        config.validator.api_url = "disabled".to_string();
    }

    if !cli.skip_jsonrpc {
        config.jsonrpc.validate_schema = true;
    }

    if cli.skip_inspector {
        config.inspector.auto_start = false;
    }

    // Filter protocol versions
    if config
        .protocols
        .versions
        .iter()
        .any(|v| v == &cli.protocol_version)
    {
        config.protocols.versions = vec![cli.protocol_version.clone()];
    } else {
        warn!(
            "Requested protocol version {} not in config, using default",
            cli.protocol_version
        );
    }

    Ok(config)
}

async fn run_full_validation(validator: &ExternalValidator, cli: &Cli) -> i32 {
    info!("Starting full MCP validation for {}", cli.server_url);

    // Create a new validator instance for mutable operations
    let config = match load_config(cli).await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return 1;
        }
    };

    let mut validator_mut = match ExternalValidator::with_config(config).await {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to create validator: {}", e);
            return 1;
        }
    };

    match validator_mut.validate_compliance(&cli.server_url).await {
        Ok(report) => {
            // Output results
            match cli.output.as_str() {
                "json" => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_default()
                ),
                "yaml" => println!("{}", serde_yaml::to_string(&report).unwrap_or_default()),
                _ => print_text_report(&report),
            }

            // Determine exit code
            if cli.strict && !report.is_compliant() {
                error!("Validation failed with strict checking enabled");
                1
            } else {
                0
            }
        }
        Err(e) => {
            error!("Validation failed: {}", e);
            1
        }
    }
}

async fn run_quick_validation(validator: &ExternalValidator, cli: &Cli) -> i32 {
    info!("Starting quick validation for {}", cli.server_url);

    match validator.quick_validate(&cli.server_url).await {
        Ok(status) => {
            match cli.output.as_str() {
                "json" => println!("{}", serde_json::json!({"status": format!("{:?}", status)})),
                _ => println!("Status: {:?}", status),
            }

            match status {
                pulseengine_mcp_external_validation::report::ComplianceStatus::Compliant => 0,
                pulseengine_mcp_external_validation::report::ComplianceStatus::Warning => {
                    if cli.strict {
                        1
                    } else {
                        0
                    }
                }
                _ => 1,
            }
        }
        Err(e) => {
            error!("Quick validation failed: {}", e);
            1
        }
    }
}

async fn run_benchmark(validator: &ExternalValidator, cli: &Cli) -> i32 {
    info!("Starting benchmark tests for {}", cli.server_url);

    match validator.benchmark_server(&cli.server_url).await {
        Ok(results) => {
            match cli.output.as_str() {
                "json" => {
                    let json_results = serde_json::json!({
                        "total_duration_ms": results.total_duration.as_millis(),
                        "iterations": results.iterations,
                        "successful_iterations": results.successful_iterations,
                        "avg_response_time_ms": results.avg_response_time_ms,
                        "min_response_time_ms": results.min_response_time_ms,
                        "max_response_time_ms": results.max_response_time_ms,
                        "throughput_rps": results.throughput_rps
                    });
                    println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
                }
                _ => {
                    println!("Benchmark Results:");
                    println!("==================");
                    println!(
                        "Total Duration: {:.2}s",
                        results.total_duration.as_secs_f64()
                    );
                    println!(
                        "Iterations: {} (successful: {})",
                        results.iterations, results.successful_iterations
                    );
                    println!(
                        "Average Response Time: {:.2}ms",
                        results.avg_response_time_ms
                    );
                    println!("Min Response Time: {:.2}ms", results.min_response_time_ms);
                    println!("Max Response Time: {:.2}ms", results.max_response_time_ms);
                    println!("Throughput: {:.2} requests/second", results.throughput_rps);
                }
            }

            if results.successful_iterations == 0 {
                error!("No successful benchmark iterations");
                1
            } else {
                0
            }
        }
        Err(e) => {
            error!("Benchmark failed: {}", e);
            1
        }
    }
}

fn print_text_report(report: &pulseengine_mcp_external_validation::ComplianceReport) {
    println!("MCP Compliance Report");
    println!("====================");
    println!("Server: {}", report.server_url);
    println!("Protocol Version: {}", report.protocol_version);
    println!("Status: {}", report.status_string());
    println!("Duration: {:.2}s", report.duration.as_secs_f64());
    println!();

    let (passed, failed, skipped) = report.test_statistics();
    let total = passed + failed + skipped;

    println!("Test Results:");
    println!("  Total Tests: {}", total);
    println!(
        "  Passed: {} ({:.1}%)",
        passed,
        if total > 0 {
            passed as f32 / total as f32 * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  Failed: {} ({:.1}%)",
        failed,
        if total > 0 {
            failed as f32 / total as f32 * 100.0
        } else {
            0.0
        }
    );
    println!("  Skipped: {}", skipped);
    println!();

    if !report.issues().is_empty() {
        println!("Issues Found ({}):", report.issues().len());
        for (i, issue) in report.issues().iter().enumerate() {
            println!(
                "  {}. [{:?}] {}: {}",
                i + 1,
                issue.severity,
                issue.category,
                issue.description
            );

            if let Some(ref suggestion) = issue.suggestion {
                println!("     Suggestion: {}", suggestion);
            }
        }
        println!();
    }

    // Performance metrics
    if report.performance.total_requests > 0 {
        println!("Performance Metrics:");
        println!(
            "  Average Response Time: {:.2}ms",
            report.performance.avg_response_time_ms
        );
        println!(
            "  95th Percentile: {:.2}ms",
            report.performance.p95_response_time_ms
        );
        println!(
            "  99th Percentile: {:.2}ms",
            report.performance.p99_response_time_ms
        );
        println!(
            "  Max Response Time: {:.2}ms",
            report.performance.max_response_time_ms
        );
        println!("  Throughput: {:.2} RPS", report.performance.throughput_rps);
        println!("  Timeouts: {}", report.performance.timeouts);
        println!("  Failures: {}", report.performance.failures);
        println!();
    }

    // External validator results
    if let Some(ref mcp_result) = report.external_results.mcp_validator {
        println!("MCP Validator Results:");
        println!(
            "  HTTP Compliance: {}/{} ({:.1}%)",
            mcp_result.http_compliance.passed,
            mcp_result.http_compliance.total,
            mcp_result.http_compliance.score * 100.0
        );
        println!(
            "  OAuth Framework: {}/{} ({:.1}%)",
            mcp_result.oauth_framework.passed,
            mcp_result.oauth_framework.total,
            mcp_result.oauth_framework.score * 100.0
        );
        println!(
            "  Protocol Features: {}/{} ({:.1}%)",
            mcp_result.protocol_features.passed,
            mcp_result.protocol_features.total,
            mcp_result.protocol_features.score * 100.0
        );
        println!();
    }

    if let Some(ref jsonrpc_result) = report.external_results.jsonrpc_validator {
        println!("JSON-RPC Validator Results:");
        println!(
            "  Schema Validation: {}/{} ({:.1}%)",
            jsonrpc_result.schema_validation.passed,
            jsonrpc_result.schema_validation.total,
            jsonrpc_result.schema_validation.score * 100.0
        );
        println!(
            "  Message Format: {}/{} ({:.1}%)",
            jsonrpc_result.message_format.passed,
            jsonrpc_result.message_format.total,
            jsonrpc_result.message_format.score * 100.0
        );
        println!();
    }

    if let Some(ref inspector_result) = report.external_results.inspector {
        println!("MCP Inspector Results:");
        println!(
            "  Connection: {}",
            if inspector_result.connection_success {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Authentication: {}",
            if inspector_result.auth_success {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Tool Discovery: {}",
            if inspector_result.tools_discoverable {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Resource Access: {}",
            if inspector_result.resources_accessible {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "  Export Functionality: {}",
            if inspector_result.export_success {
                "✅"
            } else {
                "❌"
            }
        );

        if !inspector_result.inspector_issues.is_empty() {
            println!("  Issues:");
            for issue in &inspector_result.inspector_issues {
                println!("    - {}", issue);
            }
        }
        println!();
    }

    println!("Summary: {}", report.summary());
}
