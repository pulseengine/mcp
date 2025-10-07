//! MCP Authentication Middleware
//!
//! This middleware provides comprehensive authentication and authorization
//! for MCP requests, integrating with the AuthenticationManager and
//! permission system.

use crate::{AuthContext, AuthenticationManager, models::Role, security::RequestSecurityValidator};
use async_trait::async_trait;
use pulseengine_mcp_protocol::{Error as McpError, Request, Response};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, warn};

/// Errors that can occur during authentication extraction
#[derive(Debug, Error)]
pub enum AuthExtractionError {
    #[error("No authentication provided")]
    NoAuth,

    #[error("Invalid authentication format: {0}")]
    InvalidFormat(String),

    #[error("Authentication method not supported: {0}")]
    UnsupportedMethod(String),

    #[error("Missing required header: {0}")]
    MissingHeader(String),
}

/// Configuration for MCP authentication middleware
#[derive(Debug, Clone)]
pub struct McpAuthConfig {
    /// Require authentication for all requests
    pub require_auth: bool,

    /// Allow anonymous access to specific methods
    pub anonymous_methods: Vec<String>,

    /// Methods that require specific roles
    pub method_role_requirements: HashMap<String, Vec<Role>>,

    /// Enable permission checking for tools and resources
    pub enable_permission_checking: bool,

    /// Custom authentication header name (default: "Authorization")
    pub auth_header_name: String,

    /// Enable audit logging for authentication events
    pub enable_audit_logging: bool,

    /// Client IP header name for proxy environments
    pub client_ip_header: Option<String>,
}

impl Default for McpAuthConfig {
    fn default() -> Self {
        Self {
            require_auth: true,
            anonymous_methods: vec!["initialize".to_string(), "ping".to_string()],
            method_role_requirements: HashMap::new(),
            enable_permission_checking: true,
            auth_header_name: "Authorization".to_string(),
            enable_audit_logging: true,
            client_ip_header: Some("X-Forwarded-For".to_string()),
        }
    }
}

/// Authentication context extracted from request
#[derive(Debug, Clone)]
pub struct McpAuthContext {
    /// Authenticated API key context
    pub auth_context: Option<AuthContext>,

    /// Client IP address
    pub client_ip: Option<String>,

    /// Authentication method used
    pub auth_method: Option<String>,

    /// Whether the request is anonymous
    pub is_anonymous: bool,
}

/// Request context that includes authentication and metadata
#[derive(Debug, Clone)]
pub struct McpRequestContext {
    /// Unique request identifier
    pub request_id: String,

    /// Authentication context
    pub auth: McpAuthContext,

    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl McpRequestContext {
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            auth: McpAuthContext {
                auth_context: None,
                client_ip: None,
                auth_method: None,
                is_anonymous: true,
            },
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_auth(mut self, auth_context: AuthContext, auth_method: String) -> Self {
        self.auth.auth_context = Some(auth_context);
        self.auth.auth_method = Some(auth_method);
        self.auth.is_anonymous = false;
        self
    }

    pub fn with_client_ip(mut self, client_ip: String) -> Self {
        self.auth.client_ip = Some(client_ip);
        self
    }
}

/// MCP Authentication Middleware
pub struct McpAuthMiddleware {
    /// Authentication manager for key validation
    auth_manager: Arc<AuthenticationManager>,

    /// Middleware configuration
    config: McpAuthConfig,

    /// Request security validator
    security_validator: Arc<RequestSecurityValidator>,
}

impl McpAuthMiddleware {
    /// Create a new MCP authentication middleware
    pub fn new(auth_manager: Arc<AuthenticationManager>, config: McpAuthConfig) -> Self {
        Self {
            auth_manager,
            config,
            security_validator: Arc::new(RequestSecurityValidator::default()),
        }
    }

    /// Create with custom security validator
    pub fn with_security_validator(
        auth_manager: Arc<AuthenticationManager>,
        config: McpAuthConfig,
        security_validator: Arc<RequestSecurityValidator>,
    ) -> Self {
        Self {
            auth_manager,
            config,
            security_validator,
        }
    }

    /// Create middleware with default configuration
    pub fn with_default_config(auth_manager: Arc<AuthenticationManager>) -> Self {
        Self::new(auth_manager, McpAuthConfig::default())
    }

    /// Get access to the security validator for monitoring violations
    pub fn security_validator(&self) -> &RequestSecurityValidator {
        &self.security_validator
    }

    /// Process an incoming MCP request
    pub async fn process_request(
        &self,
        request: Request,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<(Request, McpRequestContext), McpError> {
        // Step 1: Validate request security first
        if let Err(security_error) = self
            .security_validator
            .validate_request(&request, None)
            .await
        {
            error!("Request security validation failed: {}", security_error);
            return Err(McpError::invalid_request(&format!(
                "Security validation failed: {}",
                security_error
            )));
        }

        // Step 2: Sanitize request if needed
        let sanitized_request = self.security_validator.sanitize_request(request).await;

        let request_id = match &sanitized_request.id {
            Some(id) => id.to_string(),
            None => uuid::Uuid::new_v4().to_string(),
        };
        let mut context = McpRequestContext::new(request_id);

        // Extract client IP if available
        if let Some(headers) = headers {
            if let Some(ip_header) = &self.config.client_ip_header {
                if let Some(client_ip) = headers.get(ip_header) {
                    context = context.with_client_ip(client_ip.clone());
                }
            }
        }

        // Check if authentication is required for this method
        if self.should_skip_auth(&sanitized_request.method) {
            debug!(
                "Skipping authentication for method: {}",
                sanitized_request.method
            );
            return Ok((sanitized_request, context));
        }

        // Extract authentication from headers
        let auth_result = if let Some(headers) = headers {
            self.extract_authentication(headers).await
        } else {
            Err(AuthExtractionError::NoAuth)
        };

        match auth_result {
            Ok((auth_context, auth_method)) => {
                // Authentication successful
                context = context.with_auth(auth_context, auth_method);

                // Check method-specific role requirements
                if let Err(e) = self
                    .check_method_permissions(&sanitized_request.method, &context)
                    .await
                {
                    error!("Method permission check failed: {}", e);
                    return Err(McpError::invalid_request(&format!("Access denied: {}", e)));
                }

                debug!("Request authenticated successfully");
                Ok((sanitized_request, context))
            }
            Err(e) => {
                if self.config.require_auth {
                    warn!("Authentication failed: {}", e);
                    Err(McpError::invalid_request(&format!(
                        "Authentication required: {}",
                        e
                    )))
                } else {
                    debug!("Authentication failed but not required: {}", e);
                    Ok((sanitized_request, context))
                }
            }
        }
    }

    /// Process an outgoing MCP response
    pub async fn process_response(
        &self,
        response: Response,
        _context: &McpRequestContext,
    ) -> Result<Response, McpError> {
        // Add security headers or process response as needed
        // For now, just pass through
        Ok(response)
    }

    /// Extract authentication from request headers
    async fn extract_authentication(
        &self,
        headers: &HashMap<String, String>,
    ) -> Result<(AuthContext, String), AuthExtractionError> {
        // Try to extract from Authorization header
        if let Some(auth_header) = headers.get(&self.config.auth_header_name) {
            return self.parse_auth_header(auth_header).await;
        }

        // Try to extract from X-API-Key header
        if let Some(api_key) = headers.get("X-API-Key") {
            return self.validate_api_key(api_key, "X-API-Key").await;
        }

        Err(AuthExtractionError::NoAuth)
    }

    /// Parse the Authorization header
    async fn parse_auth_header(
        &self,
        auth_header: &str,
    ) -> Result<(AuthContext, String), AuthExtractionError> {
        let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(AuthExtractionError::InvalidFormat(
                "Authorization header must be in format 'Type Token'".to_string(),
            ));
        }

        let auth_type = parts[0].to_lowercase();
        let token = parts[1];

        match auth_type.as_str() {
            "bearer" => self.validate_api_key(token, "Bearer").await,
            "apikey" => self.validate_api_key(token, "ApiKey").await,
            _ => Err(AuthExtractionError::UnsupportedMethod(auth_type)),
        }
    }

    /// Validate an API key
    async fn validate_api_key(
        &self,
        api_key: &str,
        method: &str,
    ) -> Result<(AuthContext, String), AuthExtractionError> {
        match self.auth_manager.validate_api_key(api_key, None).await {
            Ok(Some(auth_context)) => Ok((auth_context, method.to_string())),
            Ok(None) => Err(AuthExtractionError::InvalidFormat(
                "Invalid API key".to_string(),
            )),
            Err(e) => {
                error!("API key validation failed: {}", e);
                Err(AuthExtractionError::InvalidFormat(
                    "Authentication failed".to_string(),
                ))
            }
        }
    }

    /// Check if authentication should be skipped for a method
    fn should_skip_auth(&self, method: &str) -> bool {
        if !self.config.require_auth {
            return true;
        }

        self.config.anonymous_methods.contains(&method.to_string())
    }

    /// Check method-specific role requirements
    async fn check_method_permissions(
        &self,
        method: &str,
        context: &McpRequestContext,
    ) -> Result<(), String> {
        // If no specific requirements, allow
        if let Some(required_roles) = self.config.method_role_requirements.get(method) {
            if let Some(auth_context) = &context.auth.auth_context {
                // Check if user has one of the required roles
                let has_required_role = auth_context
                    .roles
                    .iter()
                    .any(|role| required_roles.contains(role));
                if !has_required_role {
                    return Err(format!(
                        "Method '{}' requires one of these roles: {:?}, but user has roles: {:?}",
                        method, required_roles, auth_context.roles
                    ));
                }
            } else {
                return Err(format!("Method '{}' requires authentication", method));
            }
        }

        Ok(())
    }
}

/// Trait for middleware that can process MCP requests and responses
#[async_trait]
pub trait McpMiddleware: Send + Sync {
    /// Process an incoming request
    async fn process_request(
        &self,
        request: Request,
        context: &McpRequestContext,
    ) -> Result<Request, McpError>;

    /// Process an outgoing response
    async fn process_response(
        &self,
        response: Response,
        context: &McpRequestContext,
    ) -> Result<Response, McpError>;
}

#[async_trait]
impl McpMiddleware for McpAuthMiddleware {
    async fn process_request(
        &self,
        request: Request,
        _context: &McpRequestContext,
    ) -> Result<Request, McpError> {
        // This implementation assumes context has already been created
        // by the initial process_request call
        Ok(request)
    }

    async fn process_response(
        &self,
        response: Response,
        context: &McpRequestContext,
    ) -> Result<Response, McpError> {
        self.process_response(response, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthConfig;

    #[tokio::test]
    async fn test_auth_middleware_creation() {
        let config = AuthConfig::memory();
        let auth_manager = Arc::new(AuthenticationManager::new(config).await.unwrap());
        let middleware = McpAuthMiddleware::with_default_config(auth_manager);

        assert!(!middleware.config.anonymous_methods.is_empty());
        assert!(middleware.config.require_auth);
    }

    #[tokio::test]
    async fn test_anonymous_method_detection() {
        let config = AuthConfig::memory();
        let auth_manager = Arc::new(AuthenticationManager::new(config).await.unwrap());
        let middleware = McpAuthMiddleware::with_default_config(auth_manager);

        assert!(middleware.should_skip_auth("initialize"));
        assert!(middleware.should_skip_auth("ping"));
        assert!(!middleware.should_skip_auth("tools/call"));
    }

    #[tokio::test]
    async fn test_auth_header_parsing() {
        let config = AuthConfig::memory();
        let auth_manager = Arc::new(AuthenticationManager::new(config).await.unwrap());
        let middleware = McpAuthMiddleware::with_default_config(auth_manager);

        // Test invalid format
        let result = middleware.parse_auth_header("invalid").await;
        assert!(result.is_err());

        // Test unsupported method
        let result = middleware.parse_auth_header("Basic token123").await;
        assert!(matches!(
            result,
            Err(AuthExtractionError::UnsupportedMethod(_))
        ));
    }
}
