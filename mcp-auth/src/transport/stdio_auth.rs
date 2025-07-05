//! Stdio Transport Authentication
//!
//! This module provides authentication for stdio-based MCP servers,
//! typically used with Claude Desktop and CLI clients.

use super::auth_extractors::{
    AuthExtractionResult, AuthExtractor, AuthUtils, TransportAuthContext, TransportAuthError,
    TransportRequest, TransportType,
};
use async_trait::async_trait;
use serde_json::Value;

/// Configuration for stdio authentication
#[derive(Debug, Clone)]
pub struct StdioAuthConfig {
    /// Environment variable name for API key
    pub api_key_env_var: String,

    /// Allow authentication through MCP initialize params
    pub allow_init_params: bool,

    /// Allow authentication through process arguments
    pub allow_process_args: bool,

    /// Default API key for development
    pub default_api_key: Option<String>,

    /// Require authentication for stdio
    pub require_auth: bool,
}

impl Default for StdioAuthConfig {
    fn default() -> Self {
        Self {
            api_key_env_var: "MCP_API_KEY".to_string(),
            allow_init_params: true,
            allow_process_args: false, // Security risk in production
            default_api_key: None,
            require_auth: false, // Often used locally
        }
    }
}

/// Stdio authentication extractor
pub struct StdioAuthExtractor {
    config: StdioAuthConfig,
}

impl StdioAuthExtractor {
    /// Create a new stdio authentication extractor
    pub fn new(config: StdioAuthConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(StdioAuthConfig::default())
    }

    /// Extract authentication from environment variables
    fn extract_env_auth(&self) -> AuthExtractionResult {
        if let Ok(api_key) = std::env::var(&self.config.api_key_env_var) {
            if !api_key.is_empty() {
                AuthUtils::validate_api_key_format(&api_key)?;
                let context = TransportAuthContext::new(
                    api_key,
                    "Environment".to_string(),
                    TransportType::Stdio,
                );
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Extract authentication from MCP initialize parameters
    fn extract_init_params(&self, request: &TransportRequest) -> AuthExtractionResult {
        if !self.config.allow_init_params {
            return Ok(None);
        }

        if let Some(body) = &request.body {
            // Look for authentication in initialize request params
            if let Some(params) = body.get("params") {
                // Check for API key in various locations
                if let Some(api_key) = self.find_api_key_in_params(params) {
                    AuthUtils::validate_api_key_format(&api_key)?;
                    let context = TransportAuthContext::new(
                        api_key,
                        "InitParams".to_string(),
                        TransportType::Stdio,
                    );
                    return Ok(Some(context));
                }
            }
        }

        Ok(None)
    }

    /// Find API key in various parameter structures
    fn find_api_key_in_params(&self, params: &Value) -> Option<String> {
        // Try direct api_key field
        if let Some(api_key) = params.get("api_key").and_then(|v| v.as_str()) {
            return Some(api_key.to_string());
        }

        // Try nested clientInfo
        if let Some(client_info) = params.get("clientInfo") {
            if let Some(api_key) = client_info.get("api_key").and_then(|v| v.as_str()) {
                return Some(api_key.to_string());
            }

            // Try in capabilities
            if let Some(capabilities) = client_info.get("capabilities") {
                if let Some(auth) = capabilities.get("authentication") {
                    if let Some(api_key) = auth.get("api_key").and_then(|v| v.as_str()) {
                        return Some(api_key.to_string());
                    }
                }
            }
        }

        // Try in server capabilities/config
        if let Some(capabilities) = params.get("capabilities") {
            if let Some(auth) = capabilities.get("authentication") {
                if let Some(api_key) = auth.get("api_key").and_then(|v| v.as_str()) {
                    return Some(api_key.to_string());
                }
            }
        }

        None
    }

    /// Extract authentication from process arguments
    fn extract_process_args(&self) -> AuthExtractionResult {
        if !self.config.allow_process_args {
            return Ok(None);
        }

        let args: Vec<String> = std::env::args().collect();

        // Look for --api-key argument
        for i in 0..args.len() {
            if args[i] == "--api-key" && i + 1 < args.len() {
                let api_key = &args[i + 1];
                AuthUtils::validate_api_key_format(api_key)?;
                let context = TransportAuthContext::new(
                    api_key.clone(),
                    "ProcessArgs".to_string(),
                    TransportType::Stdio,
                );
                return Ok(Some(context));
            }

            // Look for --api-key=value format
            if let Some(key_value) = args[i].strip_prefix("--api-key=") {
                AuthUtils::validate_api_key_format(key_value)?;
                let context = TransportAuthContext::new(
                    key_value.to_string(),
                    "ProcessArgs".to_string(),
                    TransportType::Stdio,
                );
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Use default API key if configured
    fn extract_default_auth(&self) -> AuthExtractionResult {
        if let Some(ref api_key) = self.config.default_api_key {
            AuthUtils::validate_api_key_format(api_key)?;
            let context = TransportAuthContext::new(
                api_key.clone(),
                "Default".to_string(),
                TransportType::Stdio,
            );
            return Ok(Some(context));
        }

        Ok(None)
    }

    /// Add stdio-specific context information
    fn enrich_context(
        &self,
        mut context: TransportAuthContext,
        _request: &TransportRequest,
    ) -> TransportAuthContext {
        // Add process information
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_name) = current_exe.file_name().and_then(|n| n.to_str()) {
                context = context.with_metadata("process".to_string(), exe_name.to_string());
            }
        }

        // Add working directory
        if let Ok(cwd) = std::env::current_dir() {
            context =
                context.with_metadata("working_dir".to_string(), cwd.to_string_lossy().to_string());
        }

        // Add user information if available
        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            context = context.with_metadata("user".to_string(), user);
        }

        context
    }
}

#[async_trait]
impl AuthExtractor for StdioAuthExtractor {
    async fn extract_auth(&self, request: &TransportRequest) -> AuthExtractionResult {
        // Try different authentication sources in order of preference

        // 1. Environment variables
        if let Ok(Some(context)) = self.extract_env_auth() {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 2. MCP initialize parameters
        if let Ok(Some(context)) = self.extract_init_params(request) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 3. Process arguments (if allowed)
        if let Ok(Some(context)) = self.extract_process_args() {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 4. Default API key (if configured)
        if let Ok(Some(context)) = self.extract_default_auth() {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // No authentication found
        if self.config.require_auth {
            return Err(TransportAuthError::NoAuth);
        }

        Ok(None)
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Stdio
    }

    fn can_handle(&self, _request: &TransportRequest) -> bool {
        // Stdio extractor can always attempt extraction
        true
    }

    async fn validate_auth(
        &self,
        context: &TransportAuthContext,
    ) -> Result<(), TransportAuthError> {
        // Stdio-specific validation
        if context.credential.is_empty() {
            return Err(TransportAuthError::InvalidFormat(
                "Empty credential".to_string(),
            ));
        }

        // Additional validation for development environments
        if context.method == "Default" {
            tracing::warn!(
                "Using default API key for stdio authentication - not recommended for production"
            );
        }

        Ok(())
    }
}

/// Helper for creating stdio authentication configuration
impl StdioAuthConfig {
    /// Create a development-friendly configuration
    pub fn development() -> Self {
        Self {
            api_key_env_var: "MCP_API_KEY".to_string(),
            allow_init_params: true,
            allow_process_args: true,
            default_api_key: Some("lmcp_dev_1234567890abcdef".to_string()),
            require_auth: false,
        }
    }

    /// Create a production configuration
    pub fn production() -> Self {
        Self {
            api_key_env_var: "MCP_API_KEY".to_string(),
            allow_init_params: true,
            allow_process_args: false,
            default_api_key: None,
            require_auth: true,
        }
    }

    /// Create a secure configuration (minimal attack surface)
    pub fn secure() -> Self {
        Self {
            api_key_env_var: "MCP_API_KEY".to_string(),
            allow_init_params: false,
            allow_process_args: false,
            default_api_key: None,
            require_auth: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_environment_variable_extraction() {
        std::env::set_var("TEST_MCP_API_KEY", "lmcp_test_1234567890abcdef");

        let config = StdioAuthConfig {
            api_key_env_var: "TEST_MCP_API_KEY".to_string(),
            ..Default::default()
        };
        let extractor = StdioAuthExtractor::new(config);
        let request = TransportRequest::new();

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "Environment");
        assert_eq!(context.transport_type, TransportType::Stdio);

        std::env::remove_var("TEST_MCP_API_KEY");
    }

    #[test]
    fn test_init_params_extraction() {
        let extractor = StdioAuthExtractor::default();

        let init_request = json!({
            "params": {
                "api_key": "lmcp_test_1234567890abcdef",
                "clientInfo": {
                    "name": "test-client"
                }
            }
        });

        let request = TransportRequest::new().with_body(init_request);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "InitParams");
    }

    #[test]
    fn test_nested_init_params_extraction() {
        let extractor = StdioAuthExtractor::default();

        let init_request = json!({
            "params": {
                "clientInfo": {
                    "name": "test-client",
                    "capabilities": {
                        "authentication": {
                            "api_key": "lmcp_test_1234567890abcdef"
                        }
                    }
                }
            }
        });

        let request = TransportRequest::new().with_body(init_request);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "InitParams");
    }

    #[test]
    fn test_default_api_key() {
        let config = StdioAuthConfig {
            default_api_key: Some("lmcp_default_1234567890abcdef".to_string()),
            ..Default::default()
        };
        let extractor = StdioAuthExtractor::new(config);
        let request = TransportRequest::new();

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_default_1234567890abcdef");
        assert_eq!(context.method, "Default");
    }

    #[test]
    fn test_no_authentication_required() {
        let config = StdioAuthConfig {
            require_auth: false,
            ..Default::default()
        };
        let extractor = StdioAuthExtractor::new(config);
        let request = TransportRequest::new();

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_authentication_required_but_missing() {
        let config = StdioAuthConfig {
            require_auth: true,
            ..Default::default()
        };
        let extractor = StdioAuthExtractor::new(config);
        let request = TransportRequest::new();

        let result = tokio_test::block_on(extractor.extract_auth(&request));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportAuthError::NoAuth));
    }

    #[test]
    fn test_configuration_presets() {
        let dev_config = StdioAuthConfig::development();
        assert!(dev_config.allow_process_args);
        assert!(dev_config.default_api_key.is_some());
        assert!(!dev_config.require_auth);

        let prod_config = StdioAuthConfig::production();
        assert!(!prod_config.allow_process_args);
        assert!(prod_config.default_api_key.is_none());
        assert!(prod_config.require_auth);

        let secure_config = StdioAuthConfig::secure();
        assert!(!secure_config.allow_init_params);
        assert!(!secure_config.allow_process_args);
        assert!(secure_config.require_auth);
    }
}
