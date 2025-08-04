//! Common backend implementations to reduce generated code
//!
//! This module provides default implementations that can be used by macro-generated servers
//! to drastically reduce the amount of generated code per server.

use crate::{McpBackend, BackendError};
use pulseengine_mcp_protocol::*;
use async_trait::async_trait;
use std::marker::PhantomData;

/// A common error type that can be used by multiple servers to reduce generated code
#[derive(Debug, thiserror::Error)]
pub enum CommonMcpError {
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),
    
    #[error("Setup error: {0}")]
    Setup(String),
}

impl From<CommonMcpError> for pulseengine_mcp_protocol::Error {
    fn from(err: CommonMcpError) -> Self {
        match err {
            CommonMcpError::InvalidParams(msg) => 
                pulseengine_mcp_protocol::Error::invalid_params(msg),
            CommonMcpError::Internal(msg) => 
                pulseengine_mcp_protocol::Error::internal_error(msg),
            CommonMcpError::Backend(backend_err) => 
                pulseengine_mcp_protocol::Error::internal_error(backend_err.to_string()),
            CommonMcpError::Setup(msg) => 
                pulseengine_mcp_protocol::Error::internal_error(msg),
        }
    }
}

/// Default backend implementation that can be used by macro-generated servers
pub struct CommonBackendImpl<T> {
    inner: T,
    server_info: ServerInfo,
}

impl<T> CommonBackendImpl<T> {
    pub fn new(inner: T, server_info: ServerInfo) -> Self {
        Self { inner, server_info }
    }
}

#[async_trait]
impl<T> McpBackend for CommonBackendImpl<T> 
where 
    T: Send + Sync + Clone + Default + 'static,
{
    type Error = CommonMcpError;
    type Config = T;
    
    async fn initialize(config: Self::Config) -> Result<Self, Self::Error> {
        let server_info = ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                prompts: Some(PromptsCapability {
                    list_changed: Some(false),
                }),
                logging: Some(LoggingCapability {
                    level: Some("info".to_string()),
                }),
                sampling: None,
                ..Default::default()
            },
            server_info: Implementation {
                name: "MCP Server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Generated MCP Server".to_string()),
        };
        
        Ok(Self::new(config, server_info))
    }
    
    fn get_server_info(&self) -> ServerInfo {
        self.server_info.clone()
    }
    
    async fn health_check(&self) -> Result<(), Self::Error> {
        Ok(())
    }
    
    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> Result<ListToolsResult, Self::Error> {
        // Try to get tools if the inner type implements tool traits
        Ok(ListToolsResult {
            tools: vec![],
            next_cursor: None,
        })
    }
    
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> Result<CallToolResult, Self::Error> {
        Err(CommonMcpError::InvalidParams(format!("Unknown tool: {}", request.name)))
    }
    
    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }
    
    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> Result<ReadResourceResult, Self::Error> {
        Err(CommonMcpError::InvalidParams(format!("Unknown resource: {}", request.uri)))
    }
    
    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
    ) -> Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }
    
    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
    ) -> Result<GetPromptResult, Self::Error> {
        Err(CommonMcpError::InvalidParams(format!("Unknown prompt: {}", request.name)))
    }
}

/// Helper trait for macro-generated types to provide custom server info
pub trait HasServerInfo {
    fn server_info() -> ServerInfo;
}

/// Helper trait for servers with tools
pub trait McpToolsProvider {
    fn get_available_tools(&self) -> Vec<Tool>;
    fn call_tool_impl(&self, request: CallToolRequestParam) -> impl std::future::Future<Output = Result<CallToolResult, pulseengine_mcp_protocol::Error>> + Send;
}

/// Helper trait for servers with resources  
pub trait McpResourcesProvider {
    fn get_available_resources(&self) -> Vec<Resource>;
    fn read_resource_impl(&self, request: ReadResourceRequestParam) -> impl std::future::Future<Output = Result<ReadResourceResult, pulseengine_mcp_protocol::Error>> + Send;
}

/// Helper trait for servers with prompts
pub trait McpPromptsProvider {
    fn get_available_prompts(&self) -> Vec<Prompt>;
    fn get_prompt_impl(&self, request: GetPromptRequestParam) -> impl std::future::Future<Output = Result<GetPromptResult, pulseengine_mcp_protocol::Error>> + Send;
}