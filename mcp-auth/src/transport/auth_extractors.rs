//! Transport Authentication Extractors
//!
//! This module defines the common interface for extracting authentication
//! from different transport types.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during transport authentication
#[derive(Debug, Error)]
pub enum TransportAuthError {
    #[error("No authentication provided")]
    NoAuth,
    
    #[error("Invalid authentication format: {0}")]
    InvalidFormat(String),
    
    #[error("Transport not supported")]
    UnsupportedTransport,
    
    #[error("Missing required data: {0}")]
    MissingData(String),
    
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}

/// Result of authentication extraction
pub type AuthExtractionResult = Result<Option<TransportAuthContext>, TransportAuthError>;

/// Authentication context extracted from transport
#[derive(Debug, Clone)]
pub struct TransportAuthContext {
    /// API key or token
    pub credential: String,
    
    /// Authentication method used
    pub method: String,
    
    /// Client IP address (if available)
    pub client_ip: Option<String>,
    
    /// User agent (if available)
    pub user_agent: Option<String>,
    
    /// Additional metadata from transport
    pub metadata: HashMap<String, String>,
    
    /// Transport type
    pub transport_type: TransportType,
}

impl TransportAuthContext {
    /// Create a new transport auth context
    pub fn new(credential: String, method: String, transport_type: TransportType) -> Self {
        Self {
            credential,
            method,
            client_ip: None,
            user_agent: None,
            metadata: HashMap::new(),
            transport_type,
        }
    }
    
    /// Add client IP to the context
    pub fn with_client_ip(mut self, ip: String) -> Self {
        self.client_ip = Some(ip);
        self
    }
    
    /// Add user agent to the context
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }
    
    /// Add metadata to the context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Transport type enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportType {
    Http,
    WebSocket,
    Stdio,
    Custom(String),
}

/// Generic request data for transport authentication
#[derive(Debug, Clone)]
pub struct TransportRequest {
    /// HTTP-style headers
    pub headers: HashMap<String, String>,
    
    /// Query parameters (for HTTP/WebSocket)
    pub query_params: HashMap<String, String>,
    
    /// Request body or message content
    pub body: Option<Value>,
    
    /// Raw request data (for custom transports)
    pub raw_data: Option<Vec<u8>>,
    
    /// Transport-specific metadata
    pub metadata: HashMap<String, Value>,
}

impl TransportRequest {
    /// Create a new transport request
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            raw_data: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Create from HTTP-style headers
    pub fn from_headers(headers: HashMap<String, String>) -> Self {
        Self {
            headers,
            query_params: HashMap::new(),
            body: None,
            raw_data: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Add a header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
    
    /// Add a query parameter
    pub fn with_query_param(mut self, key: String, value: String) -> Self {
        self.query_params.insert(key, value);
        self
    }
    
    /// Add body content
    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }
    
    /// Get header value
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }
    
    /// Get query parameter
    pub fn get_query_param(&self, key: &str) -> Option<&String> {
        self.query_params.get(key)
    }
}

impl Default for TransportRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for extracting authentication from transport requests
#[async_trait]
pub trait AuthExtractor: Send + Sync {
    /// Extract authentication from a transport request
    async fn extract_auth(&self, request: &TransportRequest) -> AuthExtractionResult;
    
    /// Get the transport type this extractor handles
    fn transport_type(&self) -> TransportType;
    
    /// Check if this extractor can handle the given request
    fn can_handle(&self, _request: &TransportRequest) -> bool {
        // Default implementation - subclasses can override
        true
    }
    
    /// Validate the extracted authentication (optional hook)
    async fn validate_auth(&self, _context: &TransportAuthContext) -> Result<(), TransportAuthError> {
        // Default implementation does no validation
        Ok(())
    }
}

/// Utility functions for common authentication patterns
pub struct AuthUtils;

impl AuthUtils {
    /// Extract Bearer token from Authorization header
    pub fn extract_bearer_token(auth_header: &str) -> Result<String, TransportAuthError> {
        if !auth_header.starts_with("Bearer ") {
            return Err(TransportAuthError::InvalidFormat(
                "Authorization header must start with 'Bearer '".to_string(),
            ));
        }
        
        let token = &auth_header[7..]; // Skip "Bearer "
        if token.is_empty() {
            return Err(TransportAuthError::InvalidFormat(
                "Bearer token cannot be empty".to_string(),
            ));
        }
        
        Ok(token.to_string())
    }
    
    /// Extract API key from X-API-Key header
    pub fn extract_api_key_header(headers: &HashMap<String, String>) -> Option<String> {
        headers.get("X-API-Key")
            .or_else(|| headers.get("x-api-key"))
            .or_else(|| headers.get("X-Api-Key"))
            .cloned()
    }
    
    /// Extract client IP from headers (handling proxies)
    pub fn extract_client_ip(headers: &HashMap<String, String>) -> Option<String> {
        // Try common headers in order of preference
        headers.get("X-Forwarded-For")
            .or_else(|| headers.get("X-Real-IP"))
            .or_else(|| headers.get("X-Client-IP"))
            .or_else(|| headers.get("CF-Connecting-IP")) // Cloudflare
            .map(|ip| {
                // X-Forwarded-For can be a comma-separated list
                ip.split(',').next().unwrap_or(ip).trim().to_string()
            })
    }
    
    /// Extract user agent from headers
    pub fn extract_user_agent(headers: &HashMap<String, String>) -> Option<String> {
        headers.get("User-Agent")
            .or_else(|| headers.get("user-agent"))
            .cloned()
    }
    
    /// Validate API key format (basic checks)
    pub fn validate_api_key_format(api_key: &str) -> Result<(), TransportAuthError> {
        if api_key.is_empty() {
            return Err(TransportAuthError::InvalidFormat("API key cannot be empty".to_string()));
        }
        
        if api_key.len() < 16 {
            return Err(TransportAuthError::InvalidFormat("API key too short".to_string()));
        }
        
        if api_key.len() > 256 {
            return Err(TransportAuthError::InvalidFormat("API key too long".to_string()));
        }
        
        // Check for valid characters (alphanumeric, hyphens, underscores)
        if !api_key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(TransportAuthError::InvalidFormat(
                "API key contains invalid characters".to_string(),
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bearer_token_extraction() {
        let valid_header = "Bearer abc123def456";
        let token = AuthUtils::extract_bearer_token(valid_header).unwrap();
        assert_eq!(token, "abc123def456");
        
        let invalid_header = "Basic abc123";
        assert!(AuthUtils::extract_bearer_token(invalid_header).is_err());
        
        let empty_token = "Bearer ";
        assert!(AuthUtils::extract_bearer_token(empty_token).is_err());
    }
    
    #[test]
    fn test_api_key_header_extraction() {
        let mut headers = HashMap::new();
        headers.insert("X-API-Key".to_string(), "test-key-123".to_string());
        
        let key = AuthUtils::extract_api_key_header(&headers).unwrap();
        assert_eq!(key, "test-key-123");
        
        // Test case insensitive
        let mut headers2 = HashMap::new();
        headers2.insert("x-api-key".to_string(), "test-key-456".to_string());
        
        let key2 = AuthUtils::extract_api_key_header(&headers2).unwrap();
        assert_eq!(key2, "test-key-456");
    }
    
    #[test]
    fn test_client_ip_extraction() {
        let mut headers = HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "192.168.1.100, 10.0.0.1".to_string());
        
        let ip = AuthUtils::extract_client_ip(&headers).unwrap();
        assert_eq!(ip, "192.168.1.100");
        
        let mut headers2 = HashMap::new();
        headers2.insert("X-Real-IP".to_string(), "203.0.113.45".to_string());
        
        let ip2 = AuthUtils::extract_client_ip(&headers2).unwrap();
        assert_eq!(ip2, "203.0.113.45");
    }
    
    #[test]
    fn test_api_key_format_validation() {
        // Valid key
        assert!(AuthUtils::validate_api_key_format("lmcp_admin_1234567890abcdef").is_ok());
        
        // Too short
        assert!(AuthUtils::validate_api_key_format("short").is_err());
        
        // Invalid characters
        assert!(AuthUtils::validate_api_key_format("key with spaces").is_err());
        
        // Empty
        assert!(AuthUtils::validate_api_key_format("").is_err());
    }
    
    #[test]
    fn test_transport_request_builder() {
        let request = TransportRequest::new()
            .with_header("Authorization".to_string(), "Bearer token123".to_string())
            .with_query_param("format".to_string(), "json".to_string());
        
        assert_eq!(request.get_header("Authorization").unwrap(), "Bearer token123");
        assert_eq!(request.get_query_param("format").unwrap(), "json");
    }
}