//! Request validation utilities

use pulseengine_mcp_protocol::{Error, Request};

/// Request validator
pub struct RequestValidator;

impl RequestValidator {
    /// Validate an MCP request
    ///
    /// # Errors
    ///
    /// Returns an error if the request has invalid JSON-RPC version or empty method
    pub fn validate_request(request: &Request) -> Result<(), Error> {
        // Basic validation
        if request.jsonrpc != "2.0" {
            return Err(Error::invalid_request("Invalid JSON-RPC version"));
        }

        if request.method.is_empty() {
            return Err(Error::invalid_request("Method cannot be empty"));
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
