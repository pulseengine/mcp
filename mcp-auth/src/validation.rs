//! Authentication validation utilities
//!
//! This module provides helper functions for extracting authentication
//! information from requests, validating permissions, and handling
//! session management.

use crate::models::{AuthContext, Role};
use std::collections::HashMap;

/// Permission constants for common operations
pub mod permissions {
    pub const ADMIN_CREATE_KEY: &str = "admin.create_key";
    pub const ADMIN_DELETE_KEY: &str = "admin.delete_key";
    pub const ADMIN_LIST_KEYS: &str = "admin.list_keys";
    pub const ADMIN_VIEW_AUDIT: &str = "admin.view_audit";

    pub const DEVICE_READ: &str = "device.read";
    pub const DEVICE_CONTROL: &str = "device.control";

    pub const SYSTEM_STATUS: &str = "system.status";
    pub const SYSTEM_HEALTH: &str = "system.health";

    pub const MCP_TOOLS_LIST: &str = "mcp.tools.list";
    pub const MCP_TOOLS_EXECUTE: &str = "mcp.tools.execute";
    pub const MCP_RESOURCES_LIST: &str = "mcp.resources.list";
    pub const MCP_RESOURCES_READ: &str = "mcp.resources.read";
}

/// Helper function to extract client IP from various sources
/// This works with axum HTTP headers
pub fn extract_client_ip(headers: &HashMap<String, String>) -> String {
    // Try various headers in order of preference
    for header_name in ["x-forwarded-for", "x-real-ip", "x-client-ip"] {
        if let Some(ip_str) = headers.get(header_name) {
            // Take the first IP if there are multiple (comma-separated)
            let ip = ip_str.split(',').next().unwrap_or(ip_str).trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    "unknown".to_string()
}

/// Helper function to extract API key from request headers or query parameters
pub fn extract_api_key(headers: &HashMap<String, String>, query: Option<&str>) -> Option<String> {
    // Try Authorization header with Bearer token
    if let Some(auth_header) = headers.get("authorization") {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    // Try X-API-Key header
    if let Some(api_key_header) = headers.get("x-api-key") {
        return Some(api_key_header.clone());
    }

    // Try query parameter
    if let Some(query_string) = query {
        for param in query_string.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                if key == "api_key" {
                    return Some(urlencoding::decode(value).unwrap_or_default().to_string());
                }
            }
        }
    }

    None
}

/// Check if a session has the required permission
pub fn check_permission(
    context: &AuthContext,
    permission: &str,
    session_timeout_minutes: u64,
) -> bool {
    // Check if session is still valid
    if !is_session_valid(context, session_timeout_minutes) {
        return false;
    }

    // Check role-based permission
    context.has_permission(permission)
}

/// Check if a session is still valid based on timeout
pub fn is_session_valid(_context: &AuthContext, _session_timeout_minutes: u64) -> bool {
    // For now, sessions don't have explicit timestamps in AuthContext
    // This can be enhanced when we add session tracking
    true
}

/// Validate that a string is a valid UUID
pub fn is_valid_uuid(uuid_str: &str) -> bool {
    uuid::Uuid::parse_str(uuid_str).is_ok()
}

/// Validate that a string is a valid IP address
pub fn is_valid_ip_address(ip_str: &str) -> bool {
    ip_str.parse::<std::net::IpAddr>().is_ok()
}

/// Validate that a role has permission for a specific device
pub fn validate_device_permission(role: &Role, device_id: &str) -> bool {
    match role {
        Role::Admin => true,    // Admin has access to all devices
        Role::Operator => true, // Operator has access to all devices
        Role::Monitor => true,  // Monitor can read all devices
        Role::Device { allowed_devices } => allowed_devices.contains(&device_id.to_string()),
        Role::Custom { permissions } => {
            // Check if custom role has device-specific permission
            permissions
                .iter()
                .any(|perm| perm == "device.*" || perm == &format!("device.{}", device_id))
        }
    }
}

/// Generate a secure random key for API keys
pub fn generate_secure_key(prefix: &str) -> String {
    let random_part = uuid::Uuid::new_v4().to_string().replace('-', "");
    format!("{}_{}", prefix, random_part)
}

/// Sanitize input to prevent injection attacks
pub fn sanitize_input(input: &str) -> String {
    // Remove potentially dangerous characters
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || "-_.".contains(*c))
        .collect()
}

/// Validate input length and format
pub fn validate_input_format(
    input: &str,
    max_length: usize,
    allow_special: bool,
) -> Result<(), String> {
    if input.is_empty() {
        return Err("Input cannot be empty".to_string());
    }

    if input.len() > max_length {
        return Err(format!("Input too long (max: {})", max_length));
    }

    if !allow_special {
        for ch in input.chars() {
            if !ch.is_alphanumeric() && !"-_.".contains(ch) {
                return Err(format!("Invalid character: '{}'", ch));
            }
        }
    }

    Ok(())
}

/// Extract and validate rate limiting headers
pub fn extract_rate_limit_info(headers: &HashMap<String, String>) -> Option<(u32, u32)> {
    let limit = headers.get("x-ratelimit-limit")?.parse().ok()?;
    let remaining = headers.get("x-ratelimit-remaining")?.parse().ok()?;
    Some((limit, remaining))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Role;

    #[test]
    fn test_extract_client_ip() {
        let mut headers = HashMap::new();
        headers.insert(
            "x-forwarded-for".to_string(),
            "192.168.1.1, 10.0.0.1".to_string(),
        );

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, "192.168.1.1");
    }

    #[test]
    fn test_extract_api_key_from_bearer() {
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            "Bearer test_key_123".to_string(),
        );

        let key = extract_api_key(&headers, None);
        assert_eq!(key, Some("test_key_123".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_header() {
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), "test_key_123".to_string());

        let key = extract_api_key(&headers, None);
        assert_eq!(key, Some("test_key_123".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_query() {
        let headers = HashMap::new();
        let query = "param1=value1&api_key=test_key_123&param2=value2";

        let key = extract_api_key(&headers, Some(query));
        assert_eq!(key, Some("test_key_123".to_string()));
    }

    #[test]
    fn test_validate_device_permission() {
        let admin_role = Role::Admin;
        let device_role = Role::Device {
            allowed_devices: vec!["device1".to_string(), "device2".to_string()],
        };

        assert!(validate_device_permission(&admin_role, "any_device"));
        assert!(validate_device_permission(&device_role, "device1"));
        assert!(!validate_device_permission(&device_role, "device3"));
    }

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("invalid-uuid"));
        assert!(!is_valid_uuid(""));
    }

    #[test]
    fn test_is_valid_ip_address() {
        assert!(is_valid_ip_address("192.168.1.1"));
        assert!(is_valid_ip_address("::1"));
        assert!(!is_valid_ip_address("invalid-ip"));
        assert!(!is_valid_ip_address("999.999.999.999"));
    }

    #[test]
    fn test_sanitize_input() {
        assert_eq!(sanitize_input("hello_world-123.txt"), "hello_world-123.txt");
        assert_eq!(sanitize_input("hello<script>"), "helloscript");
        assert_eq!(sanitize_input("test;DROP TABLE"), "testDROPTABLE");
    }

    #[test]
    fn test_validate_input_format() {
        assert!(validate_input_format("valid_input", 20, false).is_ok());
        assert!(validate_input_format("", 20, false).is_err());
        assert!(validate_input_format("very_long_input_exceeding_limit", 10, false).is_err());
        assert!(validate_input_format("invalid@char", 20, false).is_err());
        assert!(validate_input_format("invalid@char", 20, true).is_ok());
    }
}
