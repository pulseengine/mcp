//! Session-Aware MCP Authentication Middleware
//!
//! This middleware extends the basic MCP authentication to include session management,
//! JWT token validation, and enhanced security features.

use crate::{
    jwt::JwtError,
    middleware::mcp_auth::{AuthExtractionError, McpAuthConfig, McpRequestContext},
    security::RequestSecurityValidator,
    session::{Session, SessionError, SessionManager},
    AuthContext, AuthenticationManager,
};
use pulseengine_mcp_protocol::{Error as McpError, Request, Response};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors specific to session middleware
#[derive(Debug, Error)]
pub enum SessionMiddlewareError {
    #[error("Session error: {0}")]
    SessionError(#[from] SessionError),

    #[error("Authentication error: {0}")]
    AuthError(#[from] AuthExtractionError),

    #[error("JWT validation failed: {0}")]
    JwtError(#[from] JwtError),

    #[error("Invalid session token format")]
    InvalidTokenFormat,

    #[error("Session required but not provided")]
    SessionRequired,
}

/// Enhanced configuration for session-aware middleware
#[derive(Debug, Clone)]
pub struct SessionMiddlewareConfig {
    /// Base MCP auth configuration
    pub auth_config: McpAuthConfig,

    /// Enable session management
    pub enable_sessions: bool,

    /// Require sessions for authenticated requests
    pub require_sessions: bool,

    /// Enable JWT token authentication
    pub enable_jwt_auth: bool,

    /// JWT token header name
    pub jwt_header_name: String,

    /// Session ID header name
    pub session_header_name: String,

    /// Enable automatic session creation for API keys
    pub auto_create_sessions: bool,

    /// Session duration for auto-created sessions
    pub auto_session_duration: Option<chrono::Duration>,

    /// Enable session extension on access
    pub extend_sessions_on_access: bool,

    /// Methods that bypass session requirements
    pub session_exempt_methods: Vec<String>,
}

impl Default for SessionMiddlewareConfig {
    fn default() -> Self {
        Self {
            auth_config: McpAuthConfig::default(),
            enable_sessions: true,
            require_sessions: false, // Optional by default
            enable_jwt_auth: true,
            jwt_header_name: "Authorization".to_string(),
            session_header_name: "X-Session-ID".to_string(),
            auto_create_sessions: true,
            auto_session_duration: Some(chrono::Duration::hours(24)),
            extend_sessions_on_access: true,
            session_exempt_methods: vec!["initialize".to_string(), "ping".to_string()],
        }
    }
}

/// Enhanced request context with session information
#[derive(Debug, Clone)]
pub struct SessionRequestContext {
    /// Base request context
    pub base_context: McpRequestContext,

    /// Active session (if any)
    pub session: Option<Session>,

    /// Whether request used JWT authentication
    pub jwt_authenticated: bool,

    /// Session was created automatically
    pub auto_created_session: bool,
}

impl SessionRequestContext {
    pub fn new(base_context: McpRequestContext) -> Self {
        Self {
            base_context,
            session: None,
            jwt_authenticated: false,
            auto_created_session: false,
        }
    }

    pub fn with_session(mut self, session: Session, auto_created: bool) -> Self {
        self.session = Some(session);
        self.auto_created_session = auto_created;
        self
    }

    pub fn with_jwt_auth(mut self) -> Self {
        self.jwt_authenticated = true;
        self
    }

    /// Get the session ID if available
    pub fn session_id(&self) -> Option<&str> {
        self.session.as_ref().map(|s| s.session_id.as_str())
    }

    /// Get the user ID from session or auth context
    pub fn user_id(&self) -> Option<String> {
        if let Some(session) = &self.session {
            Some(session.user_id.clone())
        } else if let Some(auth_context) = &self.base_context.auth.auth_context {
            auth_context.api_key_id.clone()
        } else {
            None
        }
    }
}

/// Session-aware MCP authentication middleware
pub struct SessionMiddleware {
    /// Authentication manager
    auth_manager: Arc<AuthenticationManager>,

    /// Session manager
    session_manager: Arc<SessionManager>,

    /// Security validator
    security_validator: Arc<RequestSecurityValidator>,

    /// Middleware configuration
    config: SessionMiddlewareConfig,
}

impl SessionMiddleware {
    /// Create new session middleware
    pub fn new(
        auth_manager: Arc<AuthenticationManager>,
        session_manager: Arc<SessionManager>,
        security_validator: Arc<RequestSecurityValidator>,
        config: SessionMiddlewareConfig,
    ) -> Self {
        Self {
            auth_manager,
            session_manager,
            security_validator,
            config,
        }
    }

    /// Create with default configuration
    pub fn with_default_config(
        auth_manager: Arc<AuthenticationManager>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self::new(
            auth_manager,
            session_manager,
            Arc::new(RequestSecurityValidator::default()),
            SessionMiddlewareConfig::default(),
        )
    }

    /// Process an incoming MCP request with session awareness
    pub async fn process_request(
        &self,
        request: Request,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<(Request, SessionRequestContext), McpError> {
        // Step 1: Security validation (same as before)
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

        let sanitized_request = self.security_validator.sanitize_request(request).await;

        // Step 2: Extract request ID and create base context
        let request_id = match &sanitized_request.id {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Null => uuid::Uuid::new_v4().to_string(),
            _ => uuid::Uuid::new_v4().to_string(),
        };

        let mut base_context = McpRequestContext::new(request_id);
        let mut session_context = SessionRequestContext::new(base_context.clone());

        // Step 3: Extract client IP
        if let Some(headers) = headers {
            if let Some(ip_header) = &self.config.auth_config.client_ip_header {
                if let Some(client_ip) = headers.get(ip_header) {
                    base_context = base_context.with_client_ip(client_ip.clone());
                }
            }
        }

        // Step 4: Check if this method requires authentication/sessions
        if self.should_skip_auth(&sanitized_request.method) {
            debug!(
                "Skipping authentication for method: {}",
                sanitized_request.method
            );
            session_context.base_context = base_context;
            return Ok((sanitized_request, session_context));
        }

        // Step 5: Try different authentication methods
        let auth_result = self.authenticate_request(headers).await;

        match auth_result {
            Ok((auth_context, auth_method, session)) => {
                // Authentication successful
                base_context = base_context.with_auth(auth_context.clone(), auth_method.clone());

                if auth_method.starts_with("JWT") {
                    session_context = session_context.with_jwt_auth();
                }

                if let Some(session) = session {
                    session_context = session_context.with_session(session, false);
                } else if self.config.auto_create_sessions && !session_context.jwt_authenticated {
                    // Auto-create session for API key authentication
                    match self.create_auto_session(&auth_context, headers).await {
                        Ok(session) => {
                            session_context = session_context.with_session(session, true);
                            info!(
                                "Auto-created session for user: {:?}",
                                auth_context.api_key_id
                            );
                        }
                        Err(e) => {
                            warn!("Failed to auto-create session: {}", e);
                        }
                    }
                }

                // Check method permissions
                if let Err(e) = self
                    .check_method_permissions(&sanitized_request.method, &base_context)
                    .await
                {
                    error!("Method permission check failed: {}", e);
                    return Err(McpError::invalid_request(&format!("Access denied: {}", e)));
                }

                session_context.base_context = base_context;
                debug!("Request authenticated successfully");
                Ok((sanitized_request, session_context))
            }
            Err(e) => {
                if self.config.auth_config.require_auth {
                    warn!("Authentication failed: {}", e);
                    Err(McpError::invalid_request(&format!(
                        "Authentication required: {}",
                        e
                    )))
                } else {
                    debug!("Authentication failed but not required: {}", e);
                    session_context.base_context = base_context;
                    Ok((sanitized_request, session_context))
                }
            }
        }
    }

    /// Authenticate request using multiple methods
    async fn authenticate_request(
        &self,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<(AuthContext, String, Option<Session>), SessionMiddlewareError> {
        if let Some(headers) = headers {
            // Try JWT authentication first
            if self.config.enable_jwt_auth {
                if let Ok((auth_context, method)) = self.try_jwt_authentication(headers).await {
                    return Ok((auth_context, method, None));
                }
            }

            // Try session ID authentication
            if self.config.enable_sessions {
                if let Ok((auth_context, session)) = self.try_session_authentication(headers).await
                {
                    return Ok((auth_context, "Session".to_string(), Some(session)));
                }
            }

            // Fall back to traditional API key authentication
            if let Ok((auth_context, method)) = self.try_api_key_authentication(headers).await {
                return Ok((auth_context, method, None));
            }
        }

        Err(SessionMiddlewareError::AuthError(
            AuthExtractionError::NoAuth,
        ))
    }

    /// Try JWT token authentication
    async fn try_jwt_authentication(
        &self,
        headers: &HashMap<String, String>,
    ) -> Result<(AuthContext, String), SessionMiddlewareError> {
        if let Some(auth_header) = headers.get(&self.config.jwt_header_name) {
            if auth_header.starts_with("Bearer ") {
                let token = &auth_header[7..];
                let auth_context = self.session_manager.validate_jwt_token(token).await?;
                return Ok((auth_context, "JWT".to_string()));
            }
        }

        Err(SessionMiddlewareError::AuthError(
            AuthExtractionError::NoAuth,
        ))
    }

    /// Try session ID authentication
    async fn try_session_authentication(
        &self,
        headers: &HashMap<String, String>,
    ) -> Result<(AuthContext, Session), SessionMiddlewareError> {
        if let Some(session_id) = headers.get(&self.config.session_header_name) {
            let session = self.session_manager.validate_session(session_id).await?;
            return Ok((session.auth_context.clone(), session));
        }

        Err(SessionMiddlewareError::AuthError(
            AuthExtractionError::NoAuth,
        ))
    }

    /// Try API key authentication
    async fn try_api_key_authentication(
        &self,
        headers: &HashMap<String, String>,
    ) -> Result<(AuthContext, String), SessionMiddlewareError> {
        // Try Authorization header
        if let Some(auth_header) = headers.get(&self.config.auth_config.auth_header_name) {
            if let Ok((auth_context, method)) = self.parse_auth_header(auth_header).await {
                return Ok((auth_context, method));
            }
        }

        // Try X-API-Key header
        if let Some(api_key) = headers.get("X-API-Key") {
            if let Ok(auth_context) = self.validate_api_key(api_key).await {
                return Ok((auth_context, "X-API-Key".to_string()));
            }
        }

        Err(SessionMiddlewareError::AuthError(
            AuthExtractionError::NoAuth,
        ))
    }

    /// Parse Authorization header
    async fn parse_auth_header(
        &self,
        auth_header: &str,
    ) -> Result<(AuthContext, String), SessionMiddlewareError> {
        let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(SessionMiddlewareError::AuthError(
                AuthExtractionError::InvalidFormat(
                    "Invalid Authorization header format".to_string(),
                ),
            ));
        }

        match parts[0] {
            "Bearer" => {
                let auth_context = self.validate_api_key(parts[1]).await?;
                Ok((auth_context, "Bearer".to_string()))
            }
            "Basic" => {
                use base64::{engine::general_purpose, Engine as _};
                let decoded = general_purpose::STANDARD.decode(parts[1]).map_err(|_| {
                    SessionMiddlewareError::AuthError(AuthExtractionError::InvalidFormat(
                        "Invalid Base64 in Basic auth".to_string(),
                    ))
                })?;

                let decoded_str = String::from_utf8(decoded).map_err(|_| {
                    SessionMiddlewareError::AuthError(AuthExtractionError::InvalidFormat(
                        "Invalid UTF-8 in Basic auth".to_string(),
                    ))
                })?;

                let auth_parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
                if auth_parts.is_empty() {
                    return Err(SessionMiddlewareError::AuthError(
                        AuthExtractionError::InvalidFormat(
                            "Basic auth must contain username".to_string(),
                        ),
                    ));
                }

                let auth_context = self.validate_api_key(auth_parts[0]).await?;
                Ok((auth_context, "Basic".to_string()))
            }
            _ => Err(SessionMiddlewareError::AuthError(
                AuthExtractionError::UnsupportedMethod(parts[0].to_string()),
            )),
        }
    }

    /// Validate API key and return auth context
    async fn validate_api_key(&self, api_key: &str) -> Result<AuthContext, SessionMiddlewareError> {
        let auth_result = self
            .auth_manager
            .validate_api_key(api_key, None)
            .await
            .map_err(|e| {
                SessionMiddlewareError::AuthError(AuthExtractionError::InvalidFormat(format!(
                    "API key validation failed: {}",
                    e
                )))
            })?;

        auth_result.ok_or_else(|| {
            SessionMiddlewareError::AuthError(AuthExtractionError::InvalidFormat(
                "Invalid API key".to_string(),
            ))
        })
    }

    /// Create automatic session for API key authentication
    async fn create_auto_session(
        &self,
        auth_context: &AuthContext,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<Session, SessionError> {
        let client_ip = headers
            .and_then(|h| {
                self.config
                    .auth_config
                    .client_ip_header
                    .as_ref()
                    .and_then(|ip_header| h.get(ip_header))
            })
            .cloned();

        let user_agent = headers.and_then(|h| h.get("User-Agent")).cloned();

        let user_id = auth_context.api_key_id.clone().unwrap_or_else(|| {
            auth_context
                .user_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        });

        let (session, _) = self
            .session_manager
            .create_session(
                user_id,
                auth_context.clone(),
                self.config.auto_session_duration,
                client_ip,
                user_agent,
            )
            .await?;

        Ok(session)
    }

    /// Check if authentication should be skipped for this method
    fn should_skip_auth(&self, method: &str) -> bool {
        self.config
            .auth_config
            .anonymous_methods
            .contains(&method.to_string())
            || self
                .config
                .session_exempt_methods
                .contains(&method.to_string())
    }

    /// Check method-specific permissions (placeholder - would integrate with permission system)
    async fn check_method_permissions(
        &self,
        _method: &str,
        _context: &McpRequestContext,
    ) -> Result<(), String> {
        // This would integrate with the permission system
        // For now, just return Ok
        Ok(())
    }

    /// Process response (add session headers if needed)
    pub async fn process_response(
        &self,
        response: Response,
        context: &SessionRequestContext,
    ) -> Result<(Response, HashMap<String, String>), McpError> {
        let mut response_headers = HashMap::new();

        // Add session ID to response headers if session exists
        if let Some(session) = &context.session {
            response_headers.insert(
                self.config.session_header_name.clone(),
                session.session_id.clone(),
            );

            if context.auto_created_session {
                response_headers.insert("X-Session-Created".to_string(), "true".to_string());
            }
        }

        Ok((response, response_headers))
    }

    /// Get session manager for external access
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Get authentication manager
    pub fn auth_manager(&self) -> &AuthenticationManager {
        &self.auth_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        session::{MemorySessionStorage, SessionConfig},
        AuthConfig,
    };

    async fn create_test_middleware() -> SessionMiddleware {
        let auth_manager = Arc::new(
            crate::AuthenticationManager::new(AuthConfig::memory())
                .await
                .unwrap(),
        );
        let session_manager = Arc::new(SessionManager::new(
            SessionConfig::default(),
            Arc::new(MemorySessionStorage::new()),
        ));

        SessionMiddleware::with_default_config(auth_manager, session_manager)
    }

    #[tokio::test]
    async fn test_session_middleware_creation() {
        let middleware = create_test_middleware().await;

        // Just test that it was created successfully
        assert!(middleware.config.enable_sessions);
    }

    #[tokio::test]
    async fn test_anonymous_request_processing() {
        let middleware = create_test_middleware().await;

        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(), // Anonymous method
            params: serde_json::json!({}),
            id: serde_json::Value::Number(1.into()),
        };

        let result = middleware.process_request(request, None).await;
        assert!(result.is_ok());

        let (_, context) = result.unwrap();
        assert!(context.session.is_none());
        assert!(context.base_context.auth.is_anonymous);
    }
}
