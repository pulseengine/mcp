//! Log sanitization for production builds
//!
//! This module provides utilities for sanitizing sensitive information
//! from log messages in production builds while preserving debugging
//! capabilities in development.

use regex::Regex;
use std::sync::OnceLock;

/// Regex patterns for detecting sensitive information
static PASSWORD_REGEX: OnceLock<Regex> = OnceLock::new();
static TOKEN_REGEX: OnceLock<Regex> = OnceLock::new();
static API_KEY_REGEX: OnceLock<Regex> = OnceLock::new();
static CREDENTIAL_REGEX: OnceLock<Regex> = OnceLock::new();
static IP_REGEX: OnceLock<Regex> = OnceLock::new();
static UUID_REGEX: OnceLock<Regex> = OnceLock::new();

/// Initialize sanitization regex patterns
fn init_sanitization_patterns() {
    PASSWORD_REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)(["']?)(password|passwd|pwd|pass)(["']?)[\s]*[=:][\s]*["`']?([^'"`\s,}]+)"#,
        )
        .expect("Invalid password regex")
    });

    TOKEN_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)(?:(["']?)(token)(["']?)[\s]*[=:][\s]*['"]?([a-zA-Z0-9._-]+)|(bearer)[\s]+([a-zA-Z0-9._-]+))"#)
            .expect("Invalid token regex")
    });

    API_KEY_REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)(["']?)(api[_-]?key|apikey|key)(["']?)[\s]*[=:][\s]*['"]?([a-zA-Z0-9._-]+)"#,
        )
        .expect("Invalid API key regex")
    });

    CREDENTIAL_REGEX.get_or_init(|| {
        Regex::new(r#"(?i)(["']?)(credential|credentials|secret|auth)(["']?)[\s]*[=:][\s]*['"]?([^'"\s,}]+)"#)
            .expect("Invalid credential regex")
    });

    IP_REGEX.get_or_init(|| {
        Regex::new(r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b")
            .expect("Invalid IP regex")
    });

    UUID_REGEX.get_or_init(|| {
        Regex::new(
            r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b",
        )
        .expect("Invalid UUID regex")
    });
}

/// Sanitization configuration
#[derive(Debug, Clone)]
pub struct SanitizationConfig {
    /// Enable sanitization (typically true in production)
    pub enabled: bool,

    /// Preserve IP addresses in logs (might be needed for debugging)
    pub preserve_ips: bool,

    /// Preserve UUIDs in logs (needed for Loxone device identification)
    pub preserve_uuids: bool,

    /// Replacement string for sensitive data
    pub replacement: String,
}

impl Default for SanitizationConfig {
    fn default() -> Self {
        Self {
            enabled: cfg!(not(debug_assertions)), // Enable in release builds
            preserve_ips: false,                  // Hide IPs in production
            preserve_uuids: true,                 // Keep UUIDs for Loxone debugging
            replacement: "[REDACTED]".to_string(),
        }
    }
}

/// Log sanitizer for removing sensitive information
pub struct LogSanitizer {
    config: SanitizationConfig,
}

impl LogSanitizer {
    /// Create a new log sanitizer with default configuration
    pub fn new() -> Self {
        Self::with_config(SanitizationConfig::default())
    }

    /// Create a new log sanitizer with custom configuration
    pub fn with_config(config: SanitizationConfig) -> Self {
        init_sanitization_patterns();
        Self { config }
    }

    /// Sanitize a log message by removing or redacting sensitive information
    pub fn sanitize(&self, message: &str) -> String {
        if !self.config.enabled {
            return message.to_string();
        }

        let mut sanitized = message.to_string();

        // Replace passwords
        if let Some(regex) = PASSWORD_REGEX.get() {
            sanitized = regex
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    let full_match = &caps[0];
                    let value = &caps[4];

                    // Replace the value part while preserving the rest of the match
                    full_match.replace(value, &self.config.replacement)
                })
                .to_string();
        }

        // Replace tokens
        if let Some(regex) = TOKEN_REGEX.get() {
            sanitized = regex
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    let full_match = &caps[0];
                    // Check which alternative matched
                    if caps.get(4).is_some() {
                        // token=value pattern
                        let value = &caps[4];
                        full_match.replace(value, &self.config.replacement)
                    } else {
                        // bearer value pattern
                        let value = &caps[6];
                        full_match.replace(value, &self.config.replacement)
                    }
                })
                .to_string();
        }

        // Replace API keys
        if let Some(regex) = API_KEY_REGEX.get() {
            sanitized = regex
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    let full_match = &caps[0];
                    let value = &caps[4];
                    full_match.replace(value, &self.config.replacement)
                })
                .to_string();
        }

        // Replace credentials
        if let Some(regex) = CREDENTIAL_REGEX.get() {
            sanitized = regex
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    let full_match = &caps[0];
                    let value = &caps[4];
                    full_match.replace(value, &self.config.replacement)
                })
                .to_string();
        }

        // Replace IP addresses if not preserved
        if !self.config.preserve_ips {
            if let Some(regex) = IP_REGEX.get() {
                sanitized = regex.replace_all(&sanitized, "[IP_REDACTED]").to_string();
            }
        }

        // Replace UUIDs if not preserved
        if !self.config.preserve_uuids {
            if let Some(regex) = UUID_REGEX.get() {
                sanitized = regex.replace_all(&sanitized, "[UUID_REDACTED]").to_string();
            }
        }

        sanitized
    }

    /// Sanitize error messages for production logging
    pub fn sanitize_error(&self, error: &dyn std::error::Error) -> String {
        let error_msg = error.to_string();

        if !self.config.enabled {
            return error_msg;
        }

        // Always sanitize the error message first
        self.sanitize(&error_msg)
    }

    /// Create a sanitized version of structured logging context
    pub fn sanitize_context(&self, context: &serde_json::Value) -> serde_json::Value {
        if !self.config.enabled {
            return context.clone();
        }

        match context {
            serde_json::Value::Object(map) => {
                let mut sanitized_map = serde_json::Map::new();

                for (key, value) in map {
                    // Don't sanitize field names in JSON contexts, only values
                    let sanitized_value = if Self::is_sensitive_field(key) {
                        serde_json::Value::String(self.config.replacement.clone())
                    } else {
                        self.sanitize_context(value)
                    };
                    sanitized_map.insert(key.clone(), sanitized_value);
                }

                serde_json::Value::Object(sanitized_map)
            }
            serde_json::Value::Array(arr) => {
                let sanitized_arr: Vec<_> = arr.iter().map(|v| self.sanitize_context(v)).collect();
                serde_json::Value::Array(sanitized_arr)
            }
            serde_json::Value::String(s) => serde_json::Value::String(self.sanitize(s)),
            other => other.clone(),
        }
    }

    /// Check if a field name indicates sensitive data
    fn is_sensitive_field(field_name: &str) -> bool {
        let lower_name = field_name.to_lowercase();
        // Check for exact matches first
        if matches!(
            lower_name.as_str(),
            "password"
                | "passwd"
                | "pwd"
                | "pass"
                | "token"
                | "secret"
                | "api_key"
                | "apikey"
                | "key"
                | "credential"
                | "credentials"
                | "auth"
                | "authorization"
                | "client_secret"
                | "private_key"
                | "bearer"
                | "access_token"
                | "refresh_token"
                | "auth_token"
        ) {
            return true;
        }

        // Also check if field name contains sensitive keywords
        lower_name.contains("password")
            || lower_name.contains("passwd")
            || lower_name.contains("token")
            || lower_name.contains("secret")
            || lower_name.contains("api_key")
            || lower_name.contains("apikey")
            || lower_name.contains("credential")
            || lower_name.contains("auth")
            || lower_name.contains("bearer")
    }

    /// Sanitize field names themselves if needed
    fn sanitize_field_name(field_name: &str) -> String {
        // If the field name is sensitive and longer than 2 chars, partially redact it
        if Self::is_sensitive_field(field_name) && field_name.len() > 2 {
            let chars: Vec<char> = field_name.chars().collect();
            let first_char = chars[0];
            let last_char = chars[chars.len() - 1];
            let middle_len = chars.len() - 2;
            format!("{}{}{}", first_char, "*".repeat(middle_len), last_char)
        } else {
            field_name.to_string()
        }
    }
}

impl Default for LogSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global sanitizer instance
static GLOBAL_SANITIZER: OnceLock<LogSanitizer> = OnceLock::new();

/// Get the global sanitizer instance
pub fn get_sanitizer() -> &'static LogSanitizer {
    GLOBAL_SANITIZER.get_or_init(LogSanitizer::new)
}

/// Initialize the global sanitizer with custom configuration
pub fn init_sanitizer(config: SanitizationConfig) {
    let _ = GLOBAL_SANITIZER.set(LogSanitizer::with_config(config));
}

/// Convenient macro for sanitized logging
#[macro_export]
macro_rules! sanitized_log {
    ($level:ident, $($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            let sanitized = $crate::logging::sanitization::get_sanitizer().sanitize(&message);
            tracing::$level!("{}", sanitized);
        }
    };
}

/// Convenient macros for different log levels
#[macro_export]
macro_rules! sanitized_error {
    ($($arg:tt)*) => { sanitized_log!(error, $($arg)*) };
}

#[macro_export]
macro_rules! sanitized_warn {
    ($($arg:tt)*) => { sanitized_log!(warn, $($arg)*) };
}

#[macro_export]
macro_rules! sanitized_info {
    ($($arg:tt)*) => { sanitized_log!(info, $($arg)*) };
}

#[macro_export]
macro_rules! sanitized_debug {
    ($($arg:tt)*) => { sanitized_log!(debug, $($arg)*) };
}

#[cfg(test)]
#[path = "sanitization_tests.rs"]
mod sanitization_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let message = "Connecting with password=secret123 to server";
        let result = sanitizer.sanitize(message);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_api_key_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let message = "API request with api_key=abc123def456 failed";
        let result = sanitizer.sanitize(message);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("abc123def456"));
    }

    #[test]
    fn test_ip_preservation() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            preserve_ips: true,
            ..Default::default()
        });

        let message = "Connecting to 192.168.1.100:8080";
        let result = sanitizer.sanitize(message);
        assert!(result.contains("192.168.1.100"));
    }

    #[test]
    fn test_ip_redaction() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            preserve_ips: false,
            ..Default::default()
        });

        let message = "Connecting to 192.168.1.100:8080";
        let result = sanitizer.sanitize(message);
        assert!(!result.contains("192.168.1.100"));
        assert!(result.contains("[IP_REDACTED]"));
    }

    #[test]
    fn test_uuid_preservation() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            preserve_uuids: true,
            ..Default::default()
        });

        let message = "Device 550e8400-e29b-41d4-a716-446655440000 state changed";
        let result = sanitizer.sanitize(message);
        assert!(result.contains("550e8400-e29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_disabled_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: false,
            ..Default::default()
        });

        let message = "password=secret123 api_key=abc123";
        let result = sanitizer.sanitize(message);
        assert_eq!(message, result);
    }

    #[test]
    fn test_error_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let error = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "password=secret123 authentication failed",
        );
        let result = sanitizer.sanitize_error(&error);
        assert_eq!("password=[REDACTED] authentication failed", result);
    }

    #[test]
    fn test_context_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let context = serde_json::json!({
            "user": "admin",
            "password": "secret123",
            "host": "192.168.1.100",
            "device_count": 42
        });

        let result = sanitizer.sanitize_context(&context);
        assert!(!result.to_string().contains("secret123"));
        assert!(result.to_string().contains("[REDACTED]"));
        assert!(result.to_string().contains("admin")); // Non-sensitive fields preserved
    }
}
