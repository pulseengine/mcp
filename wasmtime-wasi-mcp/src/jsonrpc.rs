//! JSON-RPC message handling for MCP
//!
//! This module provides types and utilities for handling JSON-RPC 2.0 messages
//! in the MCP protocol over stdio transport.

use pulseengine_mcp_protocol::model;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC response message (success)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

/// JSON-RPC error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: Value,
    pub error: ErrorObject,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC error codes
#[allow(dead_code)]
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

impl JsonRpcRequest {
    /// Parse a JSON-RPC request from a string
    pub fn parse(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl JsonRpcError {
    /// Create an error response
    pub fn error(id: Value, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error: ErrorObject {
                code,
                message,
                data,
            },
        }
    }

    /// Create a method not found error
    pub fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(
            id,
            error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", method),
            None,
        )
    }

    /// Create an internal error
    pub fn internal_error(id: Value, message: String) -> Self {
        Self::error(
            id,
            error_codes::INTERNAL_ERROR,
            message,
            None,
        )
    }

    /// Create a parse error
    pub fn parse_error(id: Value) -> Self {
        Self::error(
            id,
            error_codes::PARSE_ERROR,
            "Parse error".to_string(),
            None,
        )
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// MCP message router
///
/// Routes JSON-RPC requests to appropriate handlers based on method name.
#[derive(Debug)]
pub struct MessageRouter {
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {}
    }

    /// Route a request and return a response
    pub fn route(&self, req: &JsonRpcRequest, ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req, ctx),
            "tools/list" => self.handle_tools_list(req, ctx),
            "tools/call" => self.handle_tools_call(req, ctx),
            "resources/list" => self.handle_resources_list(req, ctx),
            "resources/read" => self.handle_resources_read(req, ctx),
            "prompts/list" => self.handle_prompts_list(req, ctx),
            "prompts/get" => self.handle_prompts_get(req, ctx),
            _ => Err((error_codes::METHOD_NOT_FOUND, format!("Method not found: {}", req.method))),
        }
    }

    fn handle_initialize(&self, _req: &JsonRpcRequest, ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        // Build initialize response
        let result = model::InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ctx.capabilities.clone().unwrap_or_default(),
            server_info: ctx.server_info.clone().unwrap_or_else(|| model::Implementation {
                name: "unknown".to_string(),
                version: "0.0.0".to_string(),
            }),
            instructions: ctx.instructions.clone(),
        };

        serde_json::to_value(result)
            .map_err(|e| (error_codes::INTERNAL_ERROR, format!("Serialization error: {}", e)))
    }

    fn handle_tools_list(&self, _req: &JsonRpcRequest, ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        let tools = ctx.registry.list_tools();
        let result = model::ListToolsResult {
            tools,
            next_cursor: None,
        };

        serde_json::to_value(result)
            .map_err(|e| (error_codes::INTERNAL_ERROR, format!("Serialization error: {}", e)))
    }

    fn handle_resources_list(&self, _req: &JsonRpcRequest, ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        let resources = ctx.registry.list_resources();
        let result = model::ListResourcesResult {
            resources,
            next_cursor: None,
        };

        serde_json::to_value(result)
            .map_err(|e| (error_codes::INTERNAL_ERROR, format!("Serialization error: {}", e)))
    }

    fn handle_prompts_list(&self, _req: &JsonRpcRequest, ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        let prompts = ctx.registry.list_prompts();
        let result = model::ListPromptsResult {
            prompts,
            next_cursor: None,
        };

        serde_json::to_value(result)
            .map_err(|e| (error_codes::INTERNAL_ERROR, format!("Serialization error: {}", e)))
    }

    fn handle_tools_call(&self, req: &JsonRpcRequest, _ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        // Parse parameters
        let params: model::CallToolRequestParam = serde_json::from_value(
            req.params.as_ref()
                .ok_or_else(|| (error_codes::INVALID_PARAMS, "Missing parameters".to_string()))?
                .clone()
        ).map_err(|e| (error_codes::INVALID_PARAMS, format!("Invalid parameters: {}", e)))?;

        // TODO: Actually invoke the component's call-tool handler
        // For now, return a stub response
        eprintln!("[ROUTER] tools/call: name={}", params.name);

        // Return tool not found for now since we can't invoke components yet
        Err((error_codes::METHOD_NOT_FOUND, format!("Tool not found (component not loaded): {}", params.name)))
    }

    fn handle_resources_read(&self, req: &JsonRpcRequest, _ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        // Parse parameters
        let params: model::ReadResourceRequestParam = serde_json::from_value(
            req.params.as_ref()
                .ok_or_else(|| (error_codes::INVALID_PARAMS, "Missing parameters".to_string()))?
                .clone()
        ).map_err(|e| (error_codes::INVALID_PARAMS, format!("Invalid parameters: {}", e)))?;

        // TODO: Actually invoke the component's read-resource handler
        // For now, return a stub response
        eprintln!("[ROUTER] resources/read: uri={}", params.uri);

        // Return resource not found for now
        Err((error_codes::METHOD_NOT_FOUND, format!("Resource not found (component not loaded): {}", params.uri)))
    }

    fn handle_prompts_get(&self, req: &JsonRpcRequest, _ctx: &crate::ctx::WasiMcpCtx) -> Result<Value, (i32, String)> {
        // Parse parameters
        let params: model::GetPromptRequestParam = serde_json::from_value(
            req.params.as_ref()
                .ok_or_else(|| (error_codes::INVALID_PARAMS, "Missing parameters".to_string()))?
                .clone()
        ).map_err(|e| (error_codes::INVALID_PARAMS, format!("Invalid parameters: {}", e)))?;

        // TODO: Actually invoke the component's get-prompt handler
        // For now, return a stub response
        eprintln!("[ROUTER] prompts/get: name={}", params.name);

        // Return prompt not found for now
        Err((error_codes::METHOD_NOT_FOUND, format!("Prompt not found (component not loaded): {}", params.name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({"protocolVersion": "2024-11-05"})),
        };

        let serialized = serde_json::to_value(&request).unwrap();
        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["method"], "initialize");
        assert_eq!(serialized["id"], 1);
    }

    #[test]
    fn test_request_deserialization() {
        let json_str = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }"#;

        let request: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "tools/list");
        assert!(request.id.is_some());
        assert!(request.params.is_some());
    }

    #[test]
    fn test_request_without_params() {
        let json_str = r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }"#;

        let request: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(request.method, "tools/list");
        assert!(request.params.is_none());
    }

    #[test]
    fn test_success_response_creation() {
        let response = JsonRpcResponse::success(json!(1), json!({"status": "ok"}));

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert_eq!(response.result.unwrap()["status"], "ok");
    }

    #[test]
    fn test_success_response_serialization() {
        let response = JsonRpcResponse::success(json!(1), json!({"tools": []}));
        let serialized = serde_json::to_value(&response).unwrap();

        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["id"], 1);
        assert!(serialized["result"].is_object());
    }

    #[test]
    fn test_error_response_creation() {
        let error = JsonRpcError::error(
            json!(1),
            error_codes::METHOD_NOT_FOUND,
            "Method not found".to_string(),
            None,
        );

        assert_eq!(error.jsonrpc, "2.0");
        assert_eq!(error.id, 1);
        assert_eq!(error.error.code, error_codes::METHOD_NOT_FOUND);
        assert_eq!(error.error.message, "Method not found");
    }

    #[test]
    fn test_error_response_with_data() {
        let error = JsonRpcError::error(
            json!(1),
            error_codes::INVALID_PARAMS,
            "Invalid params".to_string(),
            Some(json!({"field": "missing"})),
        );

        assert!(error.error.data.is_some());
        assert_eq!(error.error.data.unwrap()["field"], "missing");
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(error_codes::PARSE_ERROR, -32700);
        assert_eq!(error_codes::INVALID_REQUEST, -32600);
        assert_eq!(error_codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(error_codes::INVALID_PARAMS, -32602);
        assert_eq!(error_codes::INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_router_creation() {
        let router = MessageRouter::new();
        // Router should be created successfully
        assert!(format!("{:?}", router).contains("MessageRouter"));
    }

    #[test]
    fn test_method_not_found() {
        let router = MessageRouter::new();
        let ctx = crate::ctx::WasiMcpCtx::new_with_stdio();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "invalid/method".to_string(),
            params: None,
        };

        let result = router.route(&request, &ctx);
        assert!(result.is_err());
        let (code, message) = result.unwrap_err();
        assert_eq!(code, error_codes::METHOD_NOT_FOUND);
        assert!(message.contains("Method not found"));
    }

    #[test]
    fn test_initialize_without_params() {
        // Note: Current implementation accepts initialize without params
        // TODO: Should validate protocolVersion in params
        let router = MessageRouter::new();
        let ctx = crate::ctx::WasiMcpCtx::new_with_stdio();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let result = router.route(&request, &ctx);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_object());
        assert_eq!(response["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn test_tools_list_success() {
        let router = MessageRouter::new();
        let ctx = crate::ctx::WasiMcpCtx::new_with_stdio();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        let result = router.route(&request, &ctx);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_object());
        assert!(response.get("tools").is_some());
    }

    #[test]
    fn test_resources_list_success() {
        let router = MessageRouter::new();
        let ctx = crate::ctx::WasiMcpCtx::new_with_stdio();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "resources/list".to_string(),
            params: None,
        };

        let result = router.route(&request, &ctx);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_object());
        assert!(response.get("resources").is_some());
    }

    #[test]
    fn test_prompts_list_success() {
        let router = MessageRouter::new();
        let ctx = crate::ctx::WasiMcpCtx::new_with_stdio();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "prompts/list".to_string(),
            params: None,
        };

        let result = router.route(&request, &ctx);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_object());
        assert!(response.get("prompts").is_some());
    }

    #[test]
    fn test_parse_error_response() {
        let error = JsonRpcError::parse_error(json!(null));

        assert_eq!(error.error.code, error_codes::PARSE_ERROR);
        assert!(error.error.message.contains("Parse error"));
    }
}
