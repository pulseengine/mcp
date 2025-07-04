//! Example of Python SDK compatibility testing

use pulseengine_mcp_external_validation::{python_sdk::PythonSdkTester, ValidationConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create configuration
    let mut config = ValidationConfig::default();
    config.testing.python_sdk_compatibility = true;

    // Create Python SDK tester
    let mut tester = PythonSdkTester::new(config)?;

    // Check Python availability
    println!("Setting up Python environment for MCP SDK testing...");
    match tester.setup_environment().await {
        Ok(_) => {
            println!("✅ Python environment ready");

            // Get SDK info
            match tester.get_sdk_info().await {
                Ok(info) => {
                    println!("\nPython SDK Information:");
                    println!("  SDK Version: {}", info.version);
                    println!("  Python Version: {}", info.python_version);
                    println!("  Installed: {}", info.installed);
                }
                Err(e) => {
                    println!("❌ Failed to get SDK info: {}", e);
                }
            }

            // Test compatibility with a server
            let server_url = "http://localhost:3000";

            println!("\nTesting compatibility with {}...", server_url);
            match tester.test_compatibility(server_url).await {
                Ok(results) => {
                    println!("\n✅ Compatibility Test Results:");
                    println!("  Connection Compatible: {}", results.connection_compatible);
                    println!("  Tools Compatible: {}", results.tools_compatible);
                    println!("  Resources Compatible: {}", results.resources_compatible);
                    println!("  Transport Compatible: {}", results.transport_compatible);
                    println!(
                        "  Error Handling Compatible: {}",
                        results.error_handling_compatible
                    );
                    println!(
                        "  Overall Compatibility: {:.1}%",
                        results.compatibility_score
                    );
                }
                Err(e) => {
                    println!("❌ Compatibility test failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to setup Python environment: {}", e);
            println!("\nMake sure Python 3.9+ is installed on your system.");
        }
    }

    Ok(())
}
