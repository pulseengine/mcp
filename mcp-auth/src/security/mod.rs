//! Security features for MCP request/response processing
//!
//! This module provides comprehensive security validation, sanitization,
//! and protection features for MCP protocol messages.

pub mod request_security;

pub use request_security::{
    InputSanitizer, RequestLimitsConfig, RequestSecurityConfig, RequestSecurityValidator,
    SecuritySeverity, SecurityValidationError, SecurityViolation, SecurityViolationType,
};
