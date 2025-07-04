//! WebSocket Transport Authentication
//!
//! This module provides authentication for WebSocket-based MCP servers,
//! handling both connection-time and per-message authentication.

use super::auth_extractors::{
    AuthExtractionResult, AuthExtractor, AuthUtils, TransportAuthContext, TransportAuthError,
    TransportRequest, TransportType,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Configuration for WebSocket authentication
#[derive(Debug, Clone)]
pub struct WebSocketAuthConfig {
    /// Require authentication during WebSocket handshake
    pub require_handshake_auth: bool,

    /// Allow authentication after connection (first message)
    pub allow_post_connect_auth: bool,

    /// Supported authentication methods
    pub supported_methods: Vec<WebSocketAuthMethod>,

    /// Enable per-message authentication
    pub enable_per_message_auth: bool,

    /// WebSocket subprotocol for authentication
    pub auth_subprotocol: Option<String>,

    /// Connection timeout for authentication (seconds)
    pub auth_timeout_secs: u64,
}

impl Default for WebSocketAuthConfig {
    fn default() -> Self {
        Self {
            require_handshake_auth: true,
            allow_post_connect_auth: true,
            supported_methods: vec![
                WebSocketAuthMethod::HandshakeHeaders,
                WebSocketAuthMethod::QueryParams,
                WebSocketAuthMethod::FirstMessage,
            ],
            enable_per_message_auth: false,
            auth_subprotocol: Some("mcp-auth".to_string()),
            auth_timeout_secs: 30,
        }
    }
}

/// WebSocket authentication methods
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebSocketAuthMethod {
    /// Authentication via handshake headers
    HandshakeHeaders,

    /// Authentication via query parameters
    QueryParams,

    /// Authentication via first message
    FirstMessage,

    /// Authentication via subprotocol
    Subprotocol,

    /// Per-message authentication
    PerMessage,
}

/// WebSocket authentication extractor
pub struct WebSocketAuthExtractor {
    config: WebSocketAuthConfig,
}

impl WebSocketAuthExtractor {
    /// Create a new WebSocket authentication extractor
    pub fn new(config: WebSocketAuthConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(WebSocketAuthConfig::default())
    }

    /// Extract authentication from WebSocket handshake headers
    fn extract_handshake_headers(&self, headers: &HashMap<String, String>) -> AuthExtractionResult {
        if !self
            .config
            .supported_methods
            .contains(&WebSocketAuthMethod::HandshakeHeaders)
        {
            return Ok(None);
        }

        // Try Authorization header
        if let Some(auth_header) = headers
            .get("Authorization")
            .or_else(|| headers.get("authorization"))
        {
            if auth_header.starts_with("Bearer ") {
                match AuthUtils::extract_bearer_token(auth_header) {
                    Ok(token) => {
                        AuthUtils::validate_api_key_format(&token)?;
                        let context = TransportAuthContext::new(
                            token,
                            "HandshakeHeaders".to_string(),
                            TransportType::WebSocket,
                        );
                        return Ok(Some(context));
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        // Try X-API-Key header
        if let Some(api_key) = AuthUtils::extract_api_key_header(headers) {
            AuthUtils::validate_api_key_format(&api_key)?;
            let context = TransportAuthContext::new(
                api_key,
                "HandshakeHeaders".to_string(),
                TransportType::WebSocket,
            );
            return Ok(Some(context));
        }

        // Try WebSocket-specific headers
        if let Some(api_key) = headers.get("Sec-WebSocket-Protocol") {
            if let Some(auth_token) = self.extract_from_subprotocol(api_key) {
                AuthUtils::validate_api_key_format(&auth_token)?;
                let context = TransportAuthContext::new(
                    auth_token,
                    "Subprotocol".to_string(),
                    TransportType::WebSocket,
                );
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Extract authentication from query parameters (during handshake)
    fn extract_query_params(&self, request: &TransportRequest) -> AuthExtractionResult {
        if !self
            .config
            .supported_methods
            .contains(&WebSocketAuthMethod::QueryParams)
        {
            return Ok(None);
        }

        // Try common query parameter names
        for param_name in &["api_key", "apikey", "key", "token", "access_token"] {
            if let Some(api_key) = request.get_query_param(param_name) {
                AuthUtils::validate_api_key_format(api_key)?;
                let context = TransportAuthContext::new(
                    api_key.clone(),
                    "QueryParams".to_string(),
                    TransportType::WebSocket,
                );
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Extract authentication from first WebSocket message
    fn extract_first_message(&self, request: &TransportRequest) -> AuthExtractionResult {
        if !self
            .config
            .supported_methods
            .contains(&WebSocketAuthMethod::FirstMessage)
        {
            return Ok(None);
        }

        if let Some(body) = &request.body {
            // Look for authentication in message
            if let Some(auth_data) = self.find_auth_in_message(body) {
                AuthUtils::validate_api_key_format(&auth_data)?;
                let context = TransportAuthContext::new(
                    auth_data,
                    "FirstMessage".to_string(),
                    TransportType::WebSocket,
                );
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Extract authentication token from WebSocket subprotocol
    fn extract_from_subprotocol(&self, subprotocol: &str) -> Option<String> {
        // Format: "mcp-auth.TOKEN" or "mcp-auth-TOKEN"
        if let Some(auth_protocol) = &self.config.auth_subprotocol {
            let prefix = format!("{}.", auth_protocol);
            if let Some(token) = subprotocol.strip_prefix(&prefix) {
                return Some(token.to_string());
            }

            let prefix_dash = format!("{}-", auth_protocol);
            if let Some(token) = subprotocol.strip_prefix(&prefix_dash) {
                return Some(token.to_string());
            }
        }

        None
    }

    /// Find authentication data in WebSocket message
    fn find_auth_in_message(&self, message: &Value) -> Option<String> {
        // Try direct auth field
        if let Some(auth) = message.get("auth") {
            if let Some(api_key) = auth.get("api_key").and_then(|v| v.as_str()) {
                return Some(api_key.to_string());
            }
            if let Some(token) = auth.get("token").and_then(|v| v.as_str()) {
                return Some(token.to_string());
            }
        }

        // Try in params (for MCP initialize)
        if let Some(params) = message.get("params") {
            if let Some(api_key) = params.get("api_key").and_then(|v| v.as_str()) {
                return Some(api_key.to_string());
            }

            // Try nested in clientInfo
            if let Some(client_info) = params.get("clientInfo") {
                if let Some(auth) = client_info.get("authentication") {
                    if let Some(api_key) = auth.get("api_key").and_then(|v| v.as_str()) {
                        return Some(api_key.to_string());
                    }
                }
            }
        }

        // Try root level for simple auth messages
        if let Some(api_key) = message.get("api_key").and_then(|v| v.as_str()) {
            return Some(api_key.to_string());
        }

        None
    }

    /// Add WebSocket-specific context information
    fn enrich_context(
        &self,
        mut context: TransportAuthContext,
        request: &TransportRequest,
    ) -> TransportAuthContext {
        // Add client IP
        if let Some(client_ip) = AuthUtils::extract_client_ip(&request.headers) {
            context = context.with_client_ip(client_ip);
        }

        // Add user agent
        if let Some(user_agent) = AuthUtils::extract_user_agent(&request.headers) {
            context = context.with_user_agent(user_agent);
        }

        // Add WebSocket-specific metadata
        if let Some(origin) = request.get_header("Origin") {
            context = context.with_metadata("origin".to_string(), origin.clone());
        }

        if let Some(protocols) = request.get_header("Sec-WebSocket-Protocol") {
            context = context.with_metadata("protocols".to_string(), protocols.clone());
        }

        if let Some(version) = request.get_header("Sec-WebSocket-Version") {
            context = context.with_metadata("ws_version".to_string(), version.clone());
        }

        context
    }

    /// Check if WebSocket handshake contains authentication
    pub fn has_handshake_auth(&self, request: &TransportRequest) -> bool {
        // Check headers for auth
        if request.headers.contains_key("Authorization")
            || AuthUtils::extract_api_key_header(&request.headers).is_some()
        {
            return true;
        }

        // Check query params for auth
        for param_name in &["api_key", "apikey", "key", "token", "access_token"] {
            if request.query_params.contains_key(*param_name) {
                return true;
            }
        }

        // Check subprotocol for auth
        if let Some(protocols) = request.get_header("Sec-WebSocket-Protocol") {
            if let Some(auth_protocol) = &self.config.auth_subprotocol {
                if protocols.contains(auth_protocol) {
                    return true;
                }
            }
        }

        false
    }
}

#[async_trait]
impl AuthExtractor for WebSocketAuthExtractor {
    async fn extract_auth(&self, request: &TransportRequest) -> AuthExtractionResult {
        // Try different authentication methods in order of preference

        // 1. Handshake headers
        if let Ok(Some(context)) = self.extract_handshake_headers(&request.headers) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 2. Query parameters
        if let Ok(Some(context)) = self.extract_query_params(request) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 3. First message (if body is present)
        if let Ok(Some(context)) = self.extract_first_message(request) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // No authentication found
        if self.config.require_handshake_auth && !self.config.allow_post_connect_auth {
            return Err(TransportAuthError::NoAuth);
        }

        Ok(None)
    }

    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket
    }

    fn can_handle(&self, request: &TransportRequest) -> bool {
        // Check for WebSocket-specific headers
        request.headers.contains_key("Sec-WebSocket-Key")
            || request.headers.contains_key("Upgrade")
            || request.metadata.contains_key("websocket")
    }

    async fn validate_auth(
        &self,
        context: &TransportAuthContext,
    ) -> Result<(), TransportAuthError> {
        // WebSocket-specific validation
        if context.credential.is_empty() {
            return Err(TransportAuthError::InvalidFormat(
                "Empty credential".to_string(),
            ));
        }

        // Warn about insecure authentication methods
        if context.method == "QueryParams" {
            tracing::warn!("WebSocket authentication via query parameters is less secure - consider using headers");
        }

        Ok(())
    }
}

/// Helper for creating WebSocket authentication configuration
impl WebSocketAuthConfig {
    /// Create a secure configuration
    pub fn secure() -> Self {
        Self {
            require_handshake_auth: true,
            allow_post_connect_auth: false,
            supported_methods: vec![WebSocketAuthMethod::HandshakeHeaders],
            enable_per_message_auth: false,
            auth_subprotocol: Some("mcp-auth".to_string()),
            auth_timeout_secs: 10,
        }
    }

    /// Create a flexible configuration
    pub fn flexible() -> Self {
        Self {
            require_handshake_auth: false,
            allow_post_connect_auth: true,
            supported_methods: vec![
                WebSocketAuthMethod::HandshakeHeaders,
                WebSocketAuthMethod::QueryParams,
                WebSocketAuthMethod::FirstMessage,
            ],
            enable_per_message_auth: false,
            auth_subprotocol: Some("mcp-auth".to_string()),
            auth_timeout_secs: 30,
        }
    }

    /// Create a development-friendly configuration
    pub fn development() -> Self {
        Self {
            require_handshake_auth: false,
            allow_post_connect_auth: true,
            supported_methods: vec![
                WebSocketAuthMethod::HandshakeHeaders,
                WebSocketAuthMethod::QueryParams,
                WebSocketAuthMethod::FirstMessage,
                WebSocketAuthMethod::Subprotocol,
            ],
            enable_per_message_auth: false,
            auth_subprotocol: Some("mcp-auth".to_string()),
            auth_timeout_secs: 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_handshake_header_extraction() {
        let extractor = WebSocketAuthExtractor::default();
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer lmcp_test_1234567890abcdef".to_string(),
        );
        headers.insert("Sec-WebSocket-Key".to_string(), "test-key".to_string());

        let request = TransportRequest::from_headers(headers);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "HandshakeHeaders");
        assert_eq!(context.transport_type, TransportType::WebSocket);
    }

    #[test]
    fn test_query_parameter_extraction() {
        let extractor = WebSocketAuthExtractor::default();
        let request = TransportRequest::new()
            .with_header("Sec-WebSocket-Key".to_string(), "test-key".to_string())
            .with_query_param(
                "api_key".to_string(),
                "lmcp_test_1234567890abcdef".to_string(),
            );

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "QueryParams");
    }

    #[test]
    fn test_first_message_extraction() {
        let extractor = WebSocketAuthExtractor::default();

        let auth_message = json!({
            "auth": {
                "api_key": "lmcp_test_1234567890abcdef"
            }
        });

        let request = TransportRequest::new()
            .with_header("Sec-WebSocket-Key".to_string(), "test-key".to_string())
            .with_body(auth_message);

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "FirstMessage");
    }

    #[test]
    fn test_subprotocol_extraction() {
        let extractor = WebSocketAuthExtractor::default();
        let mut headers = HashMap::new();
        headers.insert(
            "Sec-WebSocket-Protocol".to_string(),
            "mcp-auth.lmcp_test_1234567890abcdef".to_string(),
        );
        headers.insert("Sec-WebSocket-Key".to_string(), "test-key".to_string());

        let request = TransportRequest::from_headers(headers);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "Subprotocol");
    }

    #[test]
    fn test_mcp_initialize_message() {
        let extractor = WebSocketAuthExtractor::default();

        let init_message = json!({
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "test-client",
                    "authentication": {
                        "api_key": "lmcp_test_1234567890abcdef"
                    }
                }
            }
        });

        let request = TransportRequest::new()
            .with_header("Sec-WebSocket-Key".to_string(), "test-key".to_string())
            .with_body(init_message);

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "FirstMessage");
    }

    #[test]
    fn test_has_handshake_auth() {
        let extractor = WebSocketAuthExtractor::default();

        // Test with Authorization header
        let request1 = TransportRequest::new()
            .with_header("Authorization".to_string(), "Bearer token123".to_string());
        assert!(extractor.has_handshake_auth(&request1));

        // Test with query parameter
        let request2 =
            TransportRequest::new().with_query_param("api_key".to_string(), "token123".to_string());
        assert!(extractor.has_handshake_auth(&request2));

        // Test without auth
        let request3 = TransportRequest::new();
        assert!(!extractor.has_handshake_auth(&request3));
    }

    #[test]
    fn test_configuration_presets() {
        let secure_config = WebSocketAuthConfig::secure();
        assert!(secure_config.require_handshake_auth);
        assert!(!secure_config.allow_post_connect_auth);
        assert_eq!(secure_config.auth_timeout_secs, 10);

        let flexible_config = WebSocketAuthConfig::flexible();
        assert!(!flexible_config.require_handshake_auth);
        assert!(flexible_config.allow_post_connect_auth);

        let dev_config = WebSocketAuthConfig::development();
        assert!(!dev_config.require_handshake_auth);
        assert_eq!(dev_config.auth_timeout_secs, 60);
    }
}
