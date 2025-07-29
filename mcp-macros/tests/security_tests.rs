//! Security-focused tests for macro-generated code

use pulseengine_mcp_macros::{mcp_prompt, mcp_resource, mcp_server, mcp_tool};

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
        async fn sanitize_input(&self, input: String) -> Result<String, std::io::Error> {
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
                .filter(|c| c.is_alphanumeric() || " .-_@#".contains(*c))
                .collect::<String>()
                .trim()
                .to_string();

            if sanitized.len() > 1000 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Input too long",
                ));
            }

            Ok(sanitized)
        }

        /// Validate email addresses with security checks
        async fn validate_email(&self, email: String) -> Result<String, std::io::Error> {
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
        async fn rate_limited_operation(
            &self,
            operation_id: String,
        ) -> Result<String, std::io::Error> {
            // Simulate rate limiting check
            if operation_id.len() > 100 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Operation ID too long",
                ));
            }

            // Simulate some processing time to prevent rapid-fire requests
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            Ok(format!("Operation {} completed", operation_id))
        }

        /// Secure file path validation
        async fn validate_file_path(&self, path: String) -> Result<String, std::io::Error> {
            // Prevent directory traversal
            if path.contains("..") || path.contains("~") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Directory traversal not allowed",
                ));
            }

            // Prevent access to system directories
            let forbidden_paths = [
                "/etc/",
                "/proc/",
                "/sys/",
                "/dev/",
                "/root/",
                "C:\\Windows\\",
                "C:\\Users\\",
            ];
            for forbidden in &forbidden_paths {
                if path.starts_with(forbidden) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Access to system directories forbidden",
                    ));
                }
            }

            // Only allow specific file extensions
            let allowed_extensions = [".txt", ".json", ".yaml", ".yml", ".toml", ".md"];
            if !allowed_extensions.iter().any(|ext| path.ends_with(ext)) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "File extension not allowed",
                ));
            }

            Ok(path)
        }

        /// Password strength validation
        async fn validate_password(&self, password: String) -> Result<String, std::io::Error> {
            if password.len() < 8 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Password too short (minimum 8 characters)",
                ));
            }

            if password.len() > 128 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Password too long (maximum 128 characters)",
                ));
            }

            let has_uppercase = password.chars().any(|c| c.is_uppercase());
            let has_lowercase = password.chars().any(|c| c.is_lowercase());
            let has_digit = password.chars().any(|c| c.is_ascii_digit());
            let has_special = password
                .chars()
                .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

            let strength = [has_uppercase, has_lowercase, has_digit, has_special]
                .iter()
                .filter(|&&x| x)
                .count();

            if strength < 3 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Password must contain at least 3 of: uppercase, lowercase, digit, special character",
                ));
            }

            // Check against common passwords
            let common_passwords = ["password", "123456", "qwerty", "admin", "letmein"];
            for common in &common_passwords {
                if password.to_lowercase().contains(common) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Password contains common weak patterns",
                    ));
                }
            }

            Ok("Password meets security requirements".to_string())
        }
    }

    #[mcp_resource(uri_template = "secure://{resource_type}/{resource_id}")]
    impl SecurityServer {
        /// Secure resource access with validation
        async fn secure_resource(
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
    }

    #[mcp_prompt(name = "secure_prompt")]
    impl SecurityServer {
        /// Generate secure prompts with content filtering
        async fn secure_prompt(
            &self,
            topic: String,
            context: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
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
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("Topic contains forbidden content: {}", forbidden),
                    ));
                }
            }

            // Length validation
            if topic.len() > 100 || context.len() > 500 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Input too long",
                ));
            }

            // Generate safe prompt
            let safe_text = format!(
                "Please provide information about {} in the context of {}. Keep the response educational and appropriate.",
                topic
                    .chars()
                    .filter(|c| c.is_alphanumeric() || " .-_".contains(*c))
                    .collect::<String>(),
                context
                    .chars()
                    .filter(|c| c.is_alphanumeric() || " .-_".contains(*c))
                    .collect::<String>()
            );

            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptMessageContent::Text { text: safe_text },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use security_server::*;

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

        // Test script injection
        let script_result = server
            .sanitize_input("<script>alert('xss')</script>".to_string())
            .await;
        assert!(script_result.is_err());
        assert!(
            script_result
                .unwrap_err()
                .to_string()
                .contains("dangerous input")
        );

        // Test SQL injection
        let sql_result = server
            .sanitize_input("'; DROP TABLE users; --".to_string())
            .await;
        assert!(sql_result.is_err());

        // Test directory traversal
        let traversal_result = server
            .sanitize_input("../../../etc/passwd".to_string())
            .await;
        assert!(traversal_result.is_err());

        // Test sanitization of special characters
        let special_result = server
            .sanitize_input("Hello<>World&nbsp;".to_string())
            .await;
        assert!(special_result.is_ok());
        let sanitized = special_result.unwrap();
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(!sanitized.contains('&'));
    }

    #[tokio::test]
    async fn test_email_validation() {
        let server = SecurityServer::with_defaults();

        // Test valid email
        let valid_result = server.validate_email("user@example.com".to_string()).await;
        assert!(valid_result.is_ok());
        assert_eq!(valid_result.unwrap(), "user@example.com");

        // Test invalid format
        let invalid_result = server.validate_email("not-an-email".to_string()).await;
        assert!(invalid_result.is_err());

        // Test empty parts
        let empty_result = server.validate_email("@example.com".to_string()).await;
        assert!(empty_result.is_err());

        // Test restricted emails
        let admin_result = server.validate_email("admin@example.com".to_string()).await;
        assert!(admin_result.is_err());
        assert!(admin_result.unwrap_err().to_string().contains("Restricted"));

        let root_result = server.validate_email("root@example.com".to_string()).await;
        assert!(root_result.is_err());

        // Test too long email
        let long_local = "a".repeat(70);
        let long_result = server
            .validate_email(format!("{}@example.com", long_local))
            .await;
        assert!(long_result.is_err());
        assert!(long_result.unwrap_err().to_string().contains("too long"));
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let server = SecurityServer::with_defaults();

        let start = std::time::Instant::now();

        // Test normal operation
        let result = server.rate_limited_operation("test_op_1".to_string()).await;
        assert!(result.is_ok());

        let duration = start.elapsed();
        // Should take at least 100ms due to rate limiting
        assert!(duration >= std::time::Duration::from_millis(100));

        // Test too long operation ID
        let long_id = "a".repeat(150);
        let long_result = server.rate_limited_operation(long_id).await;
        assert!(long_result.is_err());
        assert!(long_result.unwrap_err().to_string().contains("too long"));
    }

    #[tokio::test]
    async fn test_file_path_validation() {
        let server = SecurityServer::with_defaults();

        // Test safe path
        let safe_result = server
            .validate_file_path("data/config.json".to_string())
            .await;
        assert!(safe_result.is_ok());
        assert_eq!(safe_result.unwrap(), "data/config.json");

        // Test directory traversal
        let traversal_result = server
            .validate_file_path("../../../etc/passwd".to_string())
            .await;
        assert!(traversal_result.is_err());
        assert!(
            traversal_result
                .unwrap_err()
                .to_string()
                .contains("traversal")
        );

        let home_result = server.validate_file_path("~/secret.txt".to_string()).await;
        assert!(home_result.is_err());

        // Test system directories
        let etc_result = server.validate_file_path("/etc/passwd".to_string()).await;
        assert!(etc_result.is_err());
        assert!(
            etc_result
                .unwrap_err()
                .to_string()
                .contains("system directories")
        );

        let windows_result = server
            .validate_file_path("C:\\Windows\\System32\\config".to_string())
            .await;
        assert!(windows_result.is_err());

        // Test disallowed extensions
        let exe_result = server.validate_file_path("malware.exe".to_string()).await;
        assert!(exe_result.is_err());
        assert!(
            exe_result
                .unwrap_err()
                .to_string()
                .contains("extension not allowed")
        );

        let script_result = server.validate_file_path("script.sh".to_string()).await;
        assert!(script_result.is_err());
    }

    #[tokio::test]
    async fn test_password_validation() {
        let server = SecurityServer::with_defaults();

        // Test strong password
        let strong_result = server
            .validate_password("StrongP@ssw0rd!".to_string())
            .await;
        assert!(strong_result.is_ok());
        assert!(
            strong_result
                .unwrap()
                .contains("meets security requirements")
        );

        // Test too short
        let short_result = server.validate_password("weak".to_string()).await;
        assert!(short_result.is_err());
        assert!(short_result.unwrap_err().to_string().contains("too short"));

        // Test too long
        let long_password = "a".repeat(150);
        let long_result = server.validate_password(long_password).await;
        assert!(long_result.is_err());
        assert!(long_result.unwrap_err().to_string().contains("too long"));

        // Test weak password (only lowercase)
        let weak_result = server.validate_password("weakpassword".to_string()).await;
        assert!(weak_result.is_err());
        assert!(
            weak_result
                .unwrap_err()
                .to_string()
                .contains("at least 3 of")
        );

        // Test common password patterns
        let common_result = server.validate_password("password123".to_string()).await;
        assert!(common_result.is_err());
        assert!(
            common_result
                .unwrap_err()
                .to_string()
                .contains("common weak patterns")
        );

        let qwerty_result = server.validate_password("Qwerty123!".to_string()).await;
        assert!(qwerty_result.is_err());
    }

    #[tokio::test]
    async fn test_secure_resource_access() {
        let server = SecurityServer::with_defaults();

        // Test allowed resource type
        let user_result = server
            .secure_resource("user".to_string(), "john_doe".to_string())
            .await;
        assert!(user_result.is_ok());
        assert_eq!(
            user_result.unwrap(),
            "Secure access to user resource: john_doe"
        );

        // Test disallowed resource type
        let invalid_type_result = server
            .secure_resource("secrets".to_string(), "key1".to_string())
            .await;
        assert!(invalid_type_result.is_err());
        assert!(
            invalid_type_result
                .unwrap_err()
                .to_string()
                .contains("not allowed")
        );

        // Test privileged resource access
        let admin_result = server
            .secure_resource("user".to_string(), "admin_user".to_string())
            .await;
        assert!(admin_result.is_err());
        assert!(
            admin_result
                .unwrap_err()
                .to_string()
                .contains("privileged resource")
        );

        let system_result = server
            .secure_resource("user".to_string(), "system_account".to_string())
            .await;
        assert!(system_result.is_err());

        // Test config access (should be denied)
        let config_result = server
            .secure_resource("config".to_string(), "app_settings".to_string())
            .await;
        assert!(config_result.is_err());
        assert!(
            config_result
                .unwrap_err()
                .to_string()
                .contains("elevated privileges")
        );

        // Test invalid resource ID format
        let invalid_id_result = server
            .secure_resource("user".to_string(), "user@domain.com".to_string())
            .await;
        assert!(invalid_id_result.is_err());
        assert!(
            invalid_id_result
                .unwrap_err()
                .to_string()
                .contains("Invalid resource ID")
        );

        // Test too long resource ID
        let long_id = "a".repeat(60);
        let long_id_result = server.secure_resource("user".to_string(), long_id).await;
        assert!(long_id_result.is_err());
        assert!(long_id_result.unwrap_err().to_string().contains("too long"));
    }

    #[tokio::test]
    async fn test_secure_prompt_generation() {
        let server = SecurityServer::with_defaults();

        // Test safe prompt
        let safe_result = server
            .secure_prompt("cooking".to_string(), "healthy recipes".to_string())
            .await;
        assert!(safe_result.is_ok());
        let message = safe_result.unwrap();
        if let pulseengine_mcp_protocol::PromptMessageContent::Text { text } = message.content {
            assert!(text.contains("cooking"));
            assert!(text.contains("healthy recipes"));
            assert!(text.contains("educational"));
        }

        // Test forbidden topics
        let hack_result = server
            .secure_prompt("hacking".to_string(), "network security".to_string())
            .await;
        assert!(hack_result.is_err());
        assert!(
            hack_result
                .unwrap_err()
                .to_string()
                .contains("forbidden content")
        );

        let password_result = server
            .secure_prompt(
                "password cracking".to_string(),
                "security testing".to_string(),
            )
            .await;
        assert!(password_result.is_err());

        let malware_result = server
            .secure_prompt("programming".to_string(), "malware development".to_string())
            .await;
        assert!(malware_result.is_err());

        // Test input length validation
        let long_topic = "a".repeat(150);
        let long_result = server
            .secure_prompt(long_topic, "context".to_string())
            .await;
        assert!(long_result.is_err());
        assert!(long_result.unwrap_err().to_string().contains("too long"));

        let long_context = "b".repeat(600);
        let long_context_result = server
            .secure_prompt("topic".to_string(), long_context)
            .await;
        assert!(long_context_result.is_err());
    }

    #[test]
    fn test_app_specific_security_config() {
        // Test that the server is configured with app-specific authentication
        let server = SecurityServer::with_defaults();
        let info = server.get_server_info();

        // Server should be properly configured
        assert_eq!(info.server_info.name, "Security Test Server");

        // Should have security-relevant capabilities
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.prompts.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_security_operations() {
        let server = SecurityServer::with_defaults();

        // Test that security validations work correctly under concurrent load
        let mut handles = Vec::new();

        for i in 0..50 {
            let server_clone = server.clone();
            handles.push(tokio::spawn(async move {
                match i % 3 {
                    0 => server_clone
                        .sanitize_input(format!("safe_input_{}", i))
                        .await
                        .is_ok(),
                    1 => server_clone
                        .validate_email(format!("user{}@example.com", i))
                        .await
                        .is_ok(),
                    _ => server_clone
                        .validate_file_path(format!("data/file_{}.txt", i))
                        .await
                        .is_ok(),
                }
            }));
        }

        let results: Vec<bool> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All safe operations should succeed
        assert_eq!(results.len(), 50);
        assert!(results.iter().all(|&r| r));
    }

    #[tokio::test]
    async fn test_security_error_messages() {
        let server = SecurityServer::with_defaults();

        // Test that error messages don't reveal sensitive information
        let script_error = server
            .sanitize_input("<script>alert('xss')</script>".to_string())
            .await;
        assert!(script_error.is_err());
        let error_msg = script_error.unwrap_err().to_string();
        // Should indicate the pattern but not reveal system details
        assert!(error_msg.contains("dangerous input"));
        assert!(!error_msg.contains("internal"));
        assert!(!error_msg.contains("system"));

        let path_error = server
            .validate_file_path("../../../etc/passwd".to_string())
            .await;
        assert!(path_error.is_err());
        let error_msg = path_error.unwrap_err().to_string();
        assert!(error_msg.contains("traversal"));
        assert!(!error_msg.contains("passwd"));
    }
}
