//! External validation and compliance testing for MCP servers
//!
//! This crate provides comprehensive external validation to ensure your MCP server
//! implementations work correctly in real-world scenarios. It avoids "testing ourselves
//! for correctness" by using external tools and validators.
//!
#![allow(unknown_lints)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(dead_code)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::len_zero)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::single_match)]
#![allow(clippy::new_without_default)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::legacy_numeric_constants)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::useless_vec)]
#![allow(non_local_definitions)]
//! # Features
//!
//! - **MCP Validator Integration**: Official MCP protocol validator
//! - **JSON-RPC 2.0 Compliance**: External JSON-RPC specification validation
//! - **MCP Inspector Integration**: Automated testing with official tools
//! - **Python SDK Compatibility**: Cross-framework compatibility testing
//! - **Property-Based Testing**: Randomized testing for protocol invariants
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use pulseengine_mcp_external_validation::ExternalValidator;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut validator = ExternalValidator::new().await?;
//!
//!     // Validate a running MCP server
//!     let report = validator.validate_compliance("http://localhost:3000").await?;
//!
//!     if report.is_compliant() {
//!         println!("✅ Server is fully MCP compliant!");
//!     } else {
//!         println!("❌ Compliance issues found:");
//!         for issue in report.issues() {
//!             println!("  - {:?}", issue);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod auth_integration;
pub mod config;
pub mod cross_language;
pub mod ecosystem;
pub mod error;
pub mod inspector;
pub mod jsonrpc;
pub mod mcp_semantic;
pub mod mcp_validator;
pub mod python_sdk;
pub mod report;
pub mod security;
pub mod validator;

#[cfg(feature = "proptest")]
pub mod proptest;

#[cfg(feature = "fuzzing")]
pub mod fuzzing;

// Re-export main types
pub use config::ValidationConfig;
pub use error::{ValidationError, ValidationResult};
pub use report::{ComplianceReport, IssueSeverity, ValidationIssue};
pub use validator::ExternalValidator;

// Re-export for convenience
pub use auth_integration::{
    AuthIntegrationResult, AuthIntegrationTester, AuthTestOutcome, AuthTestType,
};
pub use cross_language::CrossLanguageTester;
pub use ecosystem::EcosystemTester;
pub use inspector::InspectorClient;
pub use jsonrpc::JsonRpcValidator;
pub use mcp_semantic::McpSemanticValidator;
pub use mcp_validator::McpValidatorClient;
pub use security::SecurityTester;

#[cfg(feature = "fuzzing")]
pub use fuzzing::{FuzzResult, FuzzTarget, McpFuzzer};

/// Protocol version constants for testing
pub const SUPPORTED_MCP_VERSIONS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18"];

/// Default timeout for external validation requests
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

/// Default number of retries for failed validation attempts
pub const DEFAULT_RETRIES: u32 = 3;

/// Check if an MCP protocol version is supported for validation
pub fn is_version_supported(version: &str) -> bool {
    SUPPORTED_MCP_VERSIONS.contains(&version)
}

/// Validate a server URL format
pub fn validate_server_url(url: &str) -> ValidationResult<url::Url> {
    let parsed_url = url::Url::parse(url).map_err(|e| ValidationError::InvalidServerUrl {
        url: url.to_string(),
        reason: e.to_string(),
    })?;

    // Only allow HTTP and HTTPS schemes for MCP servers
    match parsed_url.scheme() {
        "http" | "https" => Ok(parsed_url),
        _ => Err(ValidationError::InvalidServerUrl {
            url: url.to_string(),
            reason: format!(
                "Unsupported scheme: {}. Only http and https are allowed.",
                parsed_url.scheme()
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_support() {
        assert!(is_version_supported("2025-06-18"));
        assert!(is_version_supported("2025-03-26"));
        assert!(is_version_supported("2024-11-05"));
        assert!(!is_version_supported("2023-01-01"));
        assert!(!is_version_supported("invalid"));
    }

    #[test]
    fn test_url_validation() {
        assert!(validate_server_url("http://localhost:3000").is_ok());
        assert!(validate_server_url("https://api.example.com").is_ok());
        assert!(validate_server_url("not-a-url").is_err());
        assert!(validate_server_url("ftp://invalid").is_err());
    }
}
