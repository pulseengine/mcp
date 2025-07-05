//! Basic validation example

use pulseengine_mcp_external_validation::ExternalValidator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create validator with default configuration
    let validator = ExternalValidator::new().await?;

    // Check validator status
    let status = validator.get_validator_status().await?;
    println!("Validator Status:");
    println!(
        "  MCP Validator: {}",
        if status.mcp_validator_available {
            "Available"
        } else {
            "Unavailable"
        }
    );
    println!(
        "  JSON-RPC Validator: {}",
        if status.jsonrpc_validator_available {
            "Available"
        } else {
            "Unavailable"
        }
    );
    println!(
        "  Inspector: {}",
        if status.inspector_available {
            "Available"
        } else {
            "Unavailable"
        }
    );

    // Example server URL (replace with actual server)
    let server_url = "http://localhost:3000";

    // Quick validation check
    println!("\nRunning quick validation for {server_url}...");
    match validator.quick_validate(server_url).await {
        Ok(compliance_status) => {
            println!("Quick validation result: {compliance_status:?}");
        }
        Err(e) => {
            println!("Quick validation failed: {e}");
        }
    }

    // Full compliance validation (uncomment to run)
    /*
    println!("\nRunning full compliance validation...");
    match validator.validate_compliance(server_url).await {
        Ok(report) => {
            println!("Validation completed!");
            println!("Status: {}", report.status_string());
            println!("Issues found: {}", report.issues().len());

            for issue in report.issues() {
                println!("  - [{:?}] {}: {}", issue.severity, issue.category, issue.description);
            }
        }
        Err(e) => {
            println!("Validation failed: {}", e);
        }
    }
    */

    Ok(())
}
