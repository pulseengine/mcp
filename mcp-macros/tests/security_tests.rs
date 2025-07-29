//! Security-focused tests for macro-generated code

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

mod security_server {
    use super::*;

    #[mcp_server(
        name = "Security Test Server",
        app_name = "security-test-security-tests"
    )]
    #[derive(Default, Clone)]
    pub struct SecurityServer;

    #[mcp_tools]
    impl SecurityServer {
        /// Validate and sanitize user input
        pub async fn sanitize_input(&self, input: String) -> Result<String, std::io::Error> {
            // Check for common injection patterns
            let dangerous_patterns = [
                "';",
                "script>",
                "<script",
                "javascript:",
                "vbscript:",
                "onload=",
                "onerror=",
                "../",
                "..\\",
                "%2e%2e",
                "eval(",
                "exec(",
                "system(",
                "cmd.exe",
                "DROP TABLE",
                "INSERT INTO",
                "DELETE FROM",
                "UPDATE SET",
            ];

            for pattern in &dangerous_patterns {
                if input.to_lowercase().contains(&pattern.to_lowercase()) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Potentially dangerous input detected: {}", pattern),
                    ));
                }
            }

            // Sanitize the input
            let sanitized = input
                .chars()
                .filter(|c| c.is_alphanumeric() || " .-_@".contains(*c))
                .collect::<String>();

            Ok(sanitized)
        }

        /// Validate email addresses with security checks
        pub async fn validate_email(&self, email: String) -> Result<String, std::io::Error> {
            // Basic email validation
            if !email.contains('@') || email.split('@').count() != 2 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid email format",
                ));
            }

            let parts: Vec<&str> = email.split('@').collect();
            let (local, domain) = (parts[0], parts[1]);

            // Security checks
            if local.is_empty() || domain.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Empty email parts",
                ));
            }

            if local.len() > 64 || domain.len() > 255 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Email parts too long",
                ));
            }

            // Check for suspicious patterns
            let suspicious_patterns = ["admin@", "root@", "system@", "postmaster@"];
            for pattern in &suspicious_patterns {
                if email.to_lowercase().starts_with(pattern) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Restricted email address",
                    ));
                }
            }

            Ok(email.to_lowercase())
        }

        /// Rate-limited operation
        pub async fn rate_limited_operation(
            &self,
            operation_id: String,
        ) -> Result<String, std::io::Error> {
            // Simulate rate limiting
            if operation_id.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Operation ID cannot be empty",
                ));
            }

            Ok(format!("Operation {} executed", operation_id))
        }

        /// Validate file paths to prevent directory traversal
        pub async fn validate_file_path(&self, path: String) -> Result<String, std::io::Error> {
            // Check for directory traversal attempts
            if path.contains("..") || path.contains("~") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Directory traversal detected",
                ));
            }

            // Check for absolute paths
            if path.starts_with('/') || path.contains(':') {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Absolute paths not allowed",
                ));
            }

            Ok(path)
        }

        /// Validate password strength
        pub async fn validate_password(&self, password: String) -> Result<String, std::io::Error> {
            if password.len() < 8 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Password too short",
                ));
            }

            let has_upper = password.chars().any(|c| c.is_uppercase());
            let has_lower = password.chars().any(|c| c.is_lowercase());
            let has_digit = password.chars().any(|c| c.is_numeric());
            let has_special = password.chars().any(|c| "!@#$%^&*()".contains(c));

            if !has_upper || !has_lower || !has_digit || !has_special {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Password does not meet complexity requirements",
                ));
            }

            Ok("Password meets security requirements".to_string())
        }

        /// Secure resource access with validation
        pub async fn secure_resource(
            &self,
            resource_type: String,
            resource_id: String,
        ) -> Result<String, std::io::Error> {
            // Validate resource type
            let allowed_types = ["user", "document", "config", "log"];
            if !allowed_types.contains(&resource_type.as_str()) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Resource type not allowed",
                ));
            }

            // Validate resource ID format
            if !resource_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid resource ID format",
                ));
            }

            if resource_id.len() > 50 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Resource ID too long",
                ));
            }

            // Simulate access control check
            match resource_type.as_str() {
                "user" => {
                    // Users can only access their own resources
                    if resource_id.starts_with("admin_") || resource_id.starts_with("system_") {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "Access denied to privileged resource",
                        ));
                    }
                }
                "config" => {
                    // Config access is restricted
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Configuration access requires elevated privileges",
                    ));
                }
                _ => {} // Other types allowed
            }

            Ok(format!(
                "Secure access to {} resource: {}",
                resource_type, resource_id
            ))
        }

        /// Generate secure prompts with content filtering
        pub async fn secure_prompt(
            &self,
            topic: String,
            context: String,
        ) -> String {
            // Content filtering
            let forbidden_topics = [
                "password",
                "security",
                "hack",
                "exploit",
                "vulnerability",
                "inject",
                "malware",
                "virus",
                "phishing",
                "social engineering",
            ];

            for forbidden in &forbidden_topics {
                if topic.to_lowercase().contains(forbidden)
                    || context.to_lowercase().contains(forbidden)
                {
                    return format!("Error: Topic contains forbidden content: {}", forbidden);
                }
            }

            // Length validation
            if topic.len() > 100 || context.len() > 500 {
                return "Error: Input too long".to_string();
            }

            // Generate safe prompt
            let safe_text = format!(
                "Discuss the topic '{}' in the context of '{}'. Please keep the discussion professional and constructive.",
                topic
                    .chars()
                    .filter(|c| c.is_alphanumeric() || " .-_".contains(*c))
                    .collect::<String>(),
                context
                    .chars()
                    .filter(|c| c.is_alphanumeric() || " .-_".contains(*c))
                    .collect::<String>()
            );

            safe_text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use security_server::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_security_server_compiles() {
        let _server = SecurityServer::with_defaults();
    }

    #[tokio::test]
    async fn test_input_sanitization() {
        let server = SecurityServer::with_defaults();

        // Test safe input
        let safe_result = server.sanitize_input("Hello World 123".to_string()).await;
        assert!(safe_result.is_ok());
        assert_eq!(safe_result.unwrap(), "Hello World 123");
    }

    #[tokio::test]
    async fn test_email_validation() {
        let server = SecurityServer::with_defaults();

        // Test valid email
        let valid_result = server.validate_email("user@example.com".to_string()).await;
        assert!(valid_result.is_ok());
        assert_eq!(valid_result.unwrap(), "user@example.com");

        // Test invalid email
        let invalid_result = server.validate_email("invalid-email".to_string()).await;
        assert!(invalid_result.is_err());
    }

    #[tokio::test]
    async fn test_password_validation() {
        let server = SecurityServer::with_defaults();

        // Test strong password
        let strong_result = server.validate_password("MyP@ssw0rd123".to_string()).await;
        assert!(strong_result.is_ok());

        // Test weak password
        let weak_result = server.validate_password("weak".to_string()).await;
        assert!(weak_result.is_err());
    }

    #[tokio::test]
    async fn test_server_info() {
        let server = SecurityServer::with_defaults();
        let info = server.get_server_info();
        assert_eq!(info.server_info.name, "Security Test Server");
    }
}