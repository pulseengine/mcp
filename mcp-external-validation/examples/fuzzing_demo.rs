//! Demonstration of fuzzing MCP servers for protocol compliance
//!
//! This example shows how to use the fuzzing framework to test
//! an MCP server's robustness against malformed inputs.

use pulseengine_mcp_external_validation::{
    FuzzTarget, McpFuzzer, ValidationConfig, fuzzing::fuzz_results_to_issues,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,pulseengine_mcp_external_validation=debug")
        .init();

    // Server to test (start your MCP server first)
    let server_url =
        std::env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    println!("🔥 Starting MCP Protocol Fuzzing");
    println!("Target server: {server_url}");
    println!();

    // Create fuzzer with configuration
    let config = ValidationConfig::default();
    let fuzzer = McpFuzzer::new(config)
        .with_seed(42) // Use seed for reproducible results
        .with_max_iterations(1000); // Limit iterations for demo

    // Test different fuzzing targets
    let targets = vec![
        FuzzTarget::JsonRpcStructure,
        FuzzTarget::MethodNames,
        FuzzTarget::ParameterValues,
        FuzzTarget::ProtocolVersions,
        FuzzTarget::ResourceUris,
        FuzzTarget::ToolArguments,
    ];

    let mut all_results = Vec::new();

    for target in targets {
        println!("Testing {target:?}...");

        match fuzzer.fuzz_server(&server_url, target).await {
            Ok(result) => {
                println!("  ✓ Completed {} iterations", result.iterations);
                println!("  - Crashes: {}", result.crashes);
                println!("  - Hangs: {}", result.hangs);
                println!("  - Invalid responses: {}", result.invalid_responses);
                println!("  - Unique issues: {}", result.issues.len());

                if !result.issues.is_empty() {
                    println!("  - Sample issues:");
                    for (i, issue) in result.issues.iter().take(3).enumerate() {
                        println!("    {}. {:?}: {}", i + 1, issue.issue_type, issue.error);
                    }
                }

                all_results.push(result);
            }
            Err(e) => {
                println!("  ✗ Error: {e}");
            }
        }
        println!();
    }

    // Convert to validation issues
    let issues = fuzz_results_to_issues(&all_results);

    // Summary
    println!("📊 Fuzzing Summary");
    println!("==================");

    let total_iterations: usize = all_results.iter().map(|r| r.iterations).sum();
    let total_crashes: usize = all_results.iter().map(|r| r.crashes).sum();
    let total_hangs: usize = all_results.iter().map(|r| r.hangs).sum();

    println!("Total iterations: {total_iterations}");
    println!("Total crashes: {total_crashes}");
    println!("Total hangs: {total_hangs}");
    println!("Total issues: {}", issues.len());

    if total_crashes > 0 || total_hangs > 0 {
        println!();
        println!("⚠️  Critical issues detected!");
        println!("The server crashed or hung when processing malformed inputs.");
        println!("This could indicate security vulnerabilities or stability issues.");
    } else if issues.is_empty() {
        println!();
        println!("✅ No issues found!");
        println!("The server handled all fuzzed inputs gracefully.");
    } else {
        println!();
        println!("⚠️  Some issues found, but no crashes.");
        println!("Review the issues to improve protocol compliance.");
    }

    Ok(())
}
