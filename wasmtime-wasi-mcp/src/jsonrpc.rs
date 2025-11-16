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

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// MCP message router
///
/// Routes JSON-RPC requests to appropriate handlers based on method name.
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
            "resources/list" => self.handle_resources_list(req, ctx),
            "prompts/list" => self.handle_prompts_list(req, ctx),
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
}
