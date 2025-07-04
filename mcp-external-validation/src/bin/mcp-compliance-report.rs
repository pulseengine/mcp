//! Generate comprehensive compliance reports for MCP servers

use clap::Parser;
use pulseengine_mcp_external_validation::{ExternalValidator, ValidationConfig};
use std::fs;
use std::process;
use tracing::{error, info, Level};

#[derive(Parser)]
#[command(name = "mcp-compliance-report")]
#[command(about = "Generate comprehensive compliance reports for MCP servers")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Server URL to validate
    #[arg(long, short)]
    server_url: String,

    /// Output file path
    #[arg(long, short)]
    output: String,

    /// Report format (json, yaml, html, markdown)
    #[arg(long, default_value = "json")]
    format: String,

    /// Configuration file path
    #[arg(long, short)]
    config: Option<String>,

    /// Include benchmark results in report
    #[arg(long)]
    include_benchmark: bool,

    /// Verbose output
    #[arg(long, short)]
    verbose: bool,

    /// Protocol versions to test (comma-separated)
    #[arg(long)]
    protocol_versions: Option<String>,

    /// Generate comparison report with multiple servers
    #[arg(long)]
    compare_servers: Option<String>,

    /// Include detailed test traces
    #[arg(long)]
    detailed: bool,
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

    let exit_code = if let Some(ref server_list) = cli.compare_servers {
        run_comparison_report(&cli, server_list).await
    } else {
        run_single_server_report(&cli).await
    };

    process::exit(exit_code);
}

async fn run_single_server_report(cli: &Cli) -> i32 {
    info!("Generating compliance report for {}", cli.server_url);

    // Load configuration
    let config = match load_config(cli).await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return 1;
        }
    };

    // Create validator
    let mut validator = match ExternalValidator::with_config(config).await {
        Ok(validator) => validator,
        Err(e) => {
            error!("Failed to create validator: {}", e);
            return 1;
        }
    };

    // Run validation
    let mut report = match validator.validate_compliance(&cli.server_url).await {
        Ok(report) => report,
        Err(e) => {
            error!("Validation failed: {}", e);
            return 1;
        }
    };

    // Add benchmark results if requested
    if cli.include_benchmark {
        info!("Running benchmark tests...");
        match validator.benchmark_server(&cli.server_url).await {
            Ok(benchmark_results) => {
                // Add benchmark results to performance metrics
                report.performance.avg_response_time_ms = benchmark_results.avg_response_time_ms;
                report.performance.max_response_time_ms = benchmark_results.max_response_time_ms;
                report.performance.throughput_rps = benchmark_results.throughput_rps;
                report.performance.total_requests = benchmark_results.successful_iterations;
            }
            Err(e) => {
                error!("Benchmark failed: {}", e);
                // Continue with report generation
            }
        }
    }

    // Generate and save report
    match generate_report(&report, &cli.format, cli.detailed).await {
        Ok(content) => {
            if let Err(e) = fs::write(&cli.output, content) {
                error!("Failed to write report to {}: {}", cli.output, e);
                return 1;
            }
            info!("Report saved to {}", cli.output);
            0
        }
        Err(e) => {
            error!("Failed to generate report: {}", e);
            1
        }
    }
}

async fn run_comparison_report(cli: &Cli, server_list: &str) -> i32 {
    let servers: Vec<String> = server_list
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    info!("Generating comparison report for {} servers", servers.len());

    // Load configuration
    let config = match load_config(cli).await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return 1;
        }
    };

    // Create validator
    let validator = match ExternalValidator::with_config(config).await {
        Ok(validator) => validator,
        Err(e) => {
            error!("Failed to create validator: {}", e);
            return 1;
        }
    };

    // Validate all servers
    let reports = match validator.validate_multiple_servers(&servers).await {
        Ok(reports) => reports,
        Err(e) => {
            error!("Multi-server validation failed: {}", e);
            return 1;
        }
    };

    // Generate comparison report
    match generate_comparison_report(&reports, &cli.format).await {
        Ok(content) => {
            if let Err(e) = fs::write(&cli.output, content) {
                error!("Failed to write comparison report to {}: {}", cli.output, e);
                return 1;
            }
            info!("Comparison report saved to {}", cli.output);
            0
        }
        Err(e) => {
            error!("Failed to generate comparison report: {}", e);
            1
        }
    }
}

async fn load_config(cli: &Cli) -> Result<ValidationConfig, Box<dyn std::error::Error>> {
    let mut config = if let Some(ref config_path) = cli.config {
        ValidationConfig::from_file(config_path)?
    } else {
        ValidationConfig::from_env()?
    };

    // Override protocol versions if specified
    if let Some(ref versions) = cli.protocol_versions {
        let requested_versions: Vec<String> =
            versions.split(',').map(|s| s.trim().to_string()).collect();

        config.protocols.versions = requested_versions;
    }

    Ok(config)
}

async fn generate_report(
    report: &pulseengine_mcp_external_validation::ComplianceReport,
    format: &str,
    detailed: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        "json" => Ok(serde_json::to_string_pretty(report)?),
        "yaml" => Ok(serde_yaml::to_string(report)?),
        "html" => generate_html_report(report, detailed).await,
        "markdown" => generate_markdown_report(report, detailed).await,
        _ => Err(format!("Unsupported format: {}", format).into()),
    }
}

async fn generate_html_report(
    report: &pulseengine_mcp_external_validation::ComplianceReport,
    detailed: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str("<title>MCP Compliance Report</title>\n");
    html.push_str("<style>\n");
    html.push_str(get_report_css());
    html.push_str("</style>\n");
    html.push_str("</head>\n<body>\n");

    html.push_str(&format!("<h1>MCP Compliance Report</h1>\n"));
    html.push_str(&format!("<h2>Server: {}</h2>\n", report.server_url));
    html.push_str(&format!(
        "<p><strong>Status:</strong> <span class=\"status-{}\">{}</span></p>\n",
        report.status_string().to_lowercase(),
        report.status_string()
    ));
    html.push_str(&format!(
        "<p><strong>Protocol Version:</strong> {}</p>\n",
        report.protocol_version
    ));
    html.push_str(&format!(
        "<p><strong>Duration:</strong> {:.2}s</p>\n",
        report.duration.as_secs_f64()
    ));

    let (passed, failed, skipped) = report.test_statistics();
    html.push_str("<h3>Test Summary</h3>\n");
    html.push_str(&format!("<ul>\n"));
    html.push_str(&format!(
        "<li>Total Tests: {}</li>\n",
        passed + failed + skipped
    ));
    html.push_str(&format!(
        "<li>Passed: {} ({:.1}%)</li>\n",
        passed,
        if passed + failed > 0 {
            passed as f32 / (passed + failed) as f32 * 100.0
        } else {
            0.0
        }
    ));
    html.push_str(&format!("<li>Failed: {}</li>\n", failed));
    html.push_str(&format!("<li>Skipped: {}</li>\n", skipped));
    html.push_str("</ul>\n");

    if !report.issues().is_empty() {
        html.push_str("<h3>Issues</h3>\n");
        html.push_str("<ul>\n");
        for issue in report.issues() {
            html.push_str(&format!("<li class=\"issue-{:?}\">", issue.severity));
            html.push_str(&format!(
                "[{:?}] {}: {}",
                issue.severity, issue.category, issue.description
            ));
            if let Some(ref suggestion) = issue.suggestion {
                html.push_str(&format!("<br><em>Suggestion: {}</em>", suggestion));
            }
            html.push_str("</li>\n");
        }
        html.push_str("</ul>\n");
    }

    if detailed {
        // Add detailed test results, performance metrics, etc.
        if report.performance.total_requests > 0 {
            html.push_str("<h3>Performance Metrics</h3>\n");
            html.push_str("<table>\n");
            html.push_str("<tr><th>Metric</th><th>Value</th></tr>\n");
            html.push_str(&format!(
                "<tr><td>Average Response Time</td><td>{:.2}ms</td></tr>\n",
                report.performance.avg_response_time_ms
            ));
            html.push_str(&format!(
                "<tr><td>95th Percentile</td><td>{:.2}ms</td></tr>\n",
                report.performance.p95_response_time_ms
            ));
            html.push_str(&format!(
                "<tr><td>99th Percentile</td><td>{:.2}ms</td></tr>\n",
                report.performance.p99_response_time_ms
            ));
            html.push_str(&format!(
                "<tr><td>Throughput</td><td>{:.2} RPS</td></tr>\n",
                report.performance.throughput_rps
            ));
            html.push_str("</table>\n");
        }
    }

    html.push_str("</body>\n</html>\n");

    Ok(html)
}

async fn generate_markdown_report(
    report: &pulseengine_mcp_external_validation::ComplianceReport,
    detailed: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut md = String::new();

    md.push_str("# MCP Compliance Report\n\n");
    md.push_str(&format!("**Server:** {}\n", report.server_url));
    md.push_str(&format!("**Status:** {}\n", report.status_string()));
    md.push_str(&format!(
        "**Protocol Version:** {}\n",
        report.protocol_version
    ));
    md.push_str(&format!(
        "**Duration:** {:.2}s\n\n",
        report.duration.as_secs_f64()
    ));

    let (passed, failed, skipped) = report.test_statistics();
    md.push_str("## Test Summary\n\n");
    md.push_str(&format!(
        "- **Total Tests:** {}\n",
        passed + failed + skipped
    ));
    md.push_str(&format!(
        "- **Passed:** {} ({:.1}%)\n",
        passed,
        if passed + failed > 0 {
            passed as f32 / (passed + failed) as f32 * 100.0
        } else {
            0.0
        }
    ));
    md.push_str(&format!("- **Failed:** {}\n", failed));
    md.push_str(&format!("- **Skipped:** {}\n\n", skipped));

    if !report.issues().is_empty() {
        md.push_str("## Issues Found\n\n");
        for (i, issue) in report.issues().iter().enumerate() {
            md.push_str(&format!(
                "{}. **[{:?}]** {}: {}\n",
                i + 1,
                issue.severity,
                issue.category,
                issue.description
            ));
            if let Some(ref suggestion) = issue.suggestion {
                md.push_str(&format!("   - *Suggestion:* {}\n", suggestion));
            }
        }
        md.push_str("\n");
    }

    if detailed && report.performance.total_requests > 0 {
        md.push_str("## Performance Metrics\n\n");
        md.push_str("| Metric | Value |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!(
            "| Average Response Time | {:.2}ms |\n",
            report.performance.avg_response_time_ms
        ));
        md.push_str(&format!(
            "| 95th Percentile | {:.2}ms |\n",
            report.performance.p95_response_time_ms
        ));
        md.push_str(&format!(
            "| 99th Percentile | {:.2}ms |\n",
            report.performance.p99_response_time_ms
        ));
        md.push_str(&format!(
            "| Throughput | {:.2} RPS |\n",
            report.performance.throughput_rps
        ));
        md.push_str("\n");
    }

    md.push_str(&format!(
        "---\n*Generated by PulseEngine MCP External Validation v{}*\n",
        env!("CARGO_PKG_VERSION")
    ));

    Ok(md)
}

async fn generate_comparison_report(
    reports: &[pulseengine_mcp_external_validation::ComplianceReport],
    format: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match format {
        "json" => {
            let comparison = serde_json::json!({
                "comparison_report": {
                    "servers": reports.len(),
                    "reports": reports
                }
            });
            Ok(serde_json::to_string_pretty(&comparison)?)
        }
        "markdown" => {
            let mut md = String::new();
            md.push_str("# MCP Server Comparison Report\n\n");
            md.push_str(&format!("**Servers Tested:** {}\n\n", reports.len()));

            md.push_str("## Summary\n\n");
            md.push_str("| Server | Status | Tests Passed | Issues |\n");
            md.push_str("|--------|--------|--------------|--------|\n");

            for report in reports {
                let (passed, failed, _) = report.test_statistics();
                md.push_str(&format!(
                    "| {} | {} | {}/{} | {} |\n",
                    report.server_url,
                    report.status_string(),
                    passed,
                    passed + failed,
                    report.issues().len()
                ));
            }

            md.push_str("\n## Detailed Reports\n\n");
            for report in reports {
                md.push_str(&format!("### {}\n\n", report.server_url));
                let single_report = generate_markdown_report(report, false).await?;
                md.push_str(&single_report);
                md.push_str("\n---\n\n");
            }

            Ok(md)
        }
        _ => Err(format!("Unsupported comparison format: {}", format).into()),
    }
}

fn get_report_css() -> &'static str {
    r#"
body {
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    line-height: 1.6;
    color: #333;
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    background-color: #f5f5f5;
}

h1, h2, h3 {
    color: #2c3e50;
    border-bottom: 2px solid #3498db;
    padding-bottom: 10px;
}

.status-compliant { color: #27ae60; font-weight: bold; }
.status-warning { color: #f39c12; font-weight: bold; }
.status-non-compliant { color: #e74c3c; font-weight: bold; }
.status-error { color: #8e44ad; font-weight: bold; }

ul {
    background-color: white;
    padding: 20px;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.issue-critical {
    background-color: #fdf2f2;
    border-left: 4px solid #e74c3c;
    padding: 10px;
    margin: 5px 0;
    border-radius: 4px;
}

.issue-error {
    background-color: #fef5e7;
    border-left: 4px solid #f39c12;
    padding: 10px;
    margin: 5px 0;
    border-radius: 4px;
}

.issue-warning {
    background-color: #fff7ed;
    border-left: 4px solid #f59e0b;
    padding: 10px;
    margin: 5px 0;
    border-radius: 4px;
}

table {
    width: 100%;
    border-collapse: collapse;
    background-color: white;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    margin: 20px 0;
}

th, td {
    padding: 12px 15px;
    text-align: left;
    border-bottom: 1px solid #ddd;
}

th {
    background-color: #3498db;
    color: white;
    font-weight: bold;
}

tr:nth-child(even) {
    background-color: #f2f2f2;
}
    "#
}
