//! Integration tests for CLI and server interaction

use crate::test_utils::*;
use async_trait::async_trait;
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::backend::{BackendError, McpBackend};
use pulseengine_mcp_server::{CliError, create_server_info};
use pulseengine_mcp_transport::TransportConfig;
use std::error::Error as StdError;
use std::fmt;

// Test backend that integrates with CLI framework
#[derive(Clone)]
struct CliTestBackend {
    name: String,
    tools: Vec<String>,
    resources: Vec<String>,
}

#[derive(Debug)]
struct CliTestError(String);

impl fmt::Display for CliTestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CLI test error: {}", self.0)
    }
}

impl StdError for CliTestError {}

impl From<BackendError> for CliTestError {
    fn from(err: BackendError) -> Self {
        CliTestError(err.to_string())
    }
}

impl From<CliTestError> for Error {
    fn from(err: CliTestError) -> Self {
        Error::internal_error(err.to_string())
    }
}

#[async_trait]
impl McpBackend for CliTestBackend {
    type Error = CliTestError;
    type Config = (String, Vec<String>, Vec<String>); // name, tools, resources

    async fn initialize(
        (name, tools, resources): Self::Config,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name,
            tools,
            resources,
        })
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(true),
                }),
                prompts: Some(PromptsCapability {
                    list_changed: Some(true),
                }),
                logging: Some(LoggingCapability {
                    level: Some("info".to_string()),
                }),
                sampling: None,
                ..Default::default()
            },
            server_info: Implementation::new(self.name.clone(), "1.0.0"),
            instructions: Some("CLI integration test backend".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        let tools = self
            .tools
            .iter()
            .map(|name| Tool {
                name: name.clone(),
                description: format!("Tool: {name}"),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": {"type": "string"}
                    },
                    "required": ["input"]
                }),
                output_schema: None,
                title: None,
                annotations: None,
                icons: None,
                _meta: None,
            })
            .collect();

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        if self.tools.contains(&request.name) {
            let args = request.arguments.unwrap_or_default();
            let input = args
                .get("input")
                .and_then(|v| v.as_str())
                .unwrap_or("no input");

            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: format!(
                        "CLI backend '{}' executed tool '{}' with input: {}",
                        self.name, request.name, input
                    ),
                    _meta: None,
                }],
                is_error: Some(false),
                structured_content: None,
                _meta: None,
            })
        } else {
            Err(BackendError::not_supported(format!("Tool not found: {}", request.name)).into())
        }
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        let resources = self
            .resources
            .iter()
            .map(|name| Resource {
                uri: format!("cli://{name}"),
                name: name.clone(),
                description: Some(format!("Resource: {name}")),
                mime_type: Some("text/plain".to_string()),
                annotations: None,
                raw: None,
                title: None,
                icons: None,
                _meta: None,
            })
            .collect();

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        for resource_name in &self.resources {
            if request.uri == format!("cli://{resource_name}") {
                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents {
                        uri: request.uri.clone(),
                        mime_type: Some("text/plain".to_string()),
                        text: Some(format!(
                            "Content of CLI resource '{}' from backend '{}'",
                            resource_name, self.name
                        )),
                        blob: None,
                        _meta: None,
                    }],
                });
            }
        }

        Err(BackendError::not_supported(format!("Resource not found: {}", request.uri)).into())
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        Err(BackendError::not_supported(format!("Prompt not found: {}", request.name)).into())
    }
}

#[tokio::test]
async fn test_cli_server_builder_basic() {
    let server_info = create_server_info(
        Some("CLI Test Server".to_string()),
        Some("1.0.0".to_string()),
    );

    assert_eq!(server_info.server_info.name, "CLI Test Server");
    assert_eq!(server_info.server_info.version, "1.0.0");
    // Capabilities are None in default server info
    assert!(server_info.capabilities.tools.is_none());
    assert!(server_info.capabilities.resources.is_none());
    assert!(server_info.capabilities.prompts.is_none());
}

#[tokio::test]
async fn test_cli_server_info_creation() {
    let server_info = create_server_info(
        Some("Builder Test Server".to_string()),
        Some("2.0.0".to_string()),
    );

    assert_eq!(server_info.server_info.name, "Builder Test Server");
    assert_eq!(server_info.server_info.version, "2.0.0");
    // Capabilities are None in default server info
    assert!(server_info.capabilities.tools.is_none());
    assert!(server_info.capabilities.resources.is_none());
    assert!(server_info.capabilities.prompts.is_none());
}

#[tokio::test]
async fn test_cli_configuration_structs() {
    // Test that CLI configuration structs can be created
    let auth_config = test_auth_config();
    let monitoring_config = test_monitoring_config();
    let security_config = test_security_config();

    // Verify configurations are valid
    assert!(!auth_config.enabled); // We set this to false in test_auth_config
    assert!(monitoring_config.enabled);
    assert!(security_config.validate_requests);
}

#[tokio::test]
async fn test_cli_error_types() {
    // Test CLI error types
    let config_error = CliError::Configuration("Test config error".to_string());
    assert!(config_error.to_string().contains("Configuration error"));

    let parsing_error = CliError::Parsing("Test parsing error".to_string());
    assert!(parsing_error.to_string().contains("CLI parsing error"));

    let server_error = CliError::ServerSetup("Test server error".to_string());
    assert!(server_error.to_string().contains("Server setup error"));

    let logging_error = CliError::Logging("Test logging error".to_string());
    assert!(logging_error.to_string().contains("Logging setup error"));
}

#[tokio::test]
async fn test_cli_configuration_creation() {
    // Test basic CLI configuration functionality
    let auth_config = test_auth_config();
    let monitoring_config = test_monitoring_config();
    let security_config = test_security_config();

    // Verify configurations can be created
    assert!(!auth_config.enabled);
    assert!(monitoring_config.enabled);
    assert!(security_config.validate_requests);
}

#[tokio::test]
async fn test_cli_error_handling() {
    // Test CLI error types
    let config_error = CliError::Configuration("Test config error".to_string());
    assert!(config_error.to_string().contains("Configuration error"));

    let parsing_error = CliError::Parsing("Test parsing error".to_string());
    assert!(parsing_error.to_string().contains("CLI parsing error"));

    let server_error = CliError::ServerSetup("Test server error".to_string());
    assert!(server_error.to_string().contains("Server setup error"));

    let logging_error = CliError::Logging("Test logging error".to_string());
    assert!(logging_error.to_string().contains("Logging setup error"));
}

#[tokio::test]
async fn test_cli_server_integration_with_backend() {
    let backend = CliTestBackend::initialize((
        "CLI Integration Backend".to_string(),
        vec!["cli_tool1".to_string(), "cli_tool2".to_string()],
        vec!["cli_resource1".to_string()],
    ))
    .await
    .unwrap();

    // Verify backend configuration
    let server_info = backend.get_server_info();
    assert_eq!(server_info.server_info.name, "CLI Integration Backend");

    // Test health check
    assert!(backend.health_check().await.is_ok());

    // Test tools listing
    let tools_result = backend
        .list_tools(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert_eq!(tools_result.tools.len(), 2);
    assert_eq!(tools_result.tools[0].name, "cli_tool1");
    assert_eq!(tools_result.tools[1].name, "cli_tool2");

    // Test tool execution
    let call_result = backend
        .call_tool(CallToolRequestParam {
            name: "cli_tool1".to_string(),
            arguments: Some(serde_json::json!({"input": "test input"})),
        })
        .await
        .unwrap();

    assert_eq!(call_result.is_error, Some(false));
    match &call_result.content[0] {
        Content::Text { text, .. } => {
            assert!(text.contains("CLI Integration Backend"));
            assert!(text.contains("cli_tool1"));
            assert!(text.contains("test input"));
        }
        _ => panic!("Expected text content"),
    }

    // Test resources
    let resources_result = backend
        .list_resources(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert_eq!(resources_result.resources.len(), 1);
    assert_eq!(resources_result.resources[0].name, "cli_resource1");

    // Test resource reading
    let read_result = backend
        .read_resource(ReadResourceRequestParam {
            uri: "cli://cli_resource1".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(read_result.contents.len(), 1);
    assert!(
        read_result.contents[0]
            .text
            .as_ref()
            .unwrap()
            .contains("CLI Integration Backend")
    );
}

#[tokio::test]
async fn test_server_info_creation() {
    // Test with custom name and version
    let custom_info = create_server_info(
        Some("Custom CLI Server".to_string()),
        Some("3.1.4".to_string()),
    );

    assert_eq!(custom_info.server_info.name, "Custom CLI Server");
    assert_eq!(custom_info.server_info.version, "3.1.4");

    // Test with default values (should use Cargo.toml values)
    let default_info = create_server_info(None, None);

    assert!(!default_info.server_info.name.is_empty());
    assert!(!default_info.server_info.version.is_empty());
    assert!(default_info.server_info.version.contains('.'));
}

#[tokio::test]
async fn test_cli_transport_integration() {
    let transport_configs = vec![
        ("Stdio", TransportConfig::Stdio),
        (
            "HTTP",
            TransportConfig::Http {
                host: Some("127.0.0.1".to_string()),
                port: 8080,
            },
        ),
        (
            "WebSocket",
            TransportConfig::WebSocket {
                host: Some("127.0.0.1".to_string()),
                port: 8081,
            },
        ),
    ];

    for (name, _transport_config) in transport_configs {
        // Verify transport configurations can be created
        println!("Successfully created {} transport config", name);
    }
}

#[tokio::test]
async fn test_cli_full_integration_scenario() {
    // Create a comprehensive CLI + server integration test
    let backend = CliTestBackend::initialize((
        "Full Integration Backend".to_string(),
        vec!["integration_tool".to_string()],
        vec!["integration_resource".to_string()],
    ))
    .await
    .unwrap();

    let server_info = create_server_info(
        Some("Full Integration Server".to_string()),
        Some("1.0.0".to_string()),
    );

    // Verify server info creation
    assert_eq!(server_info.server_info.name, "Full Integration Server");
    assert_eq!(server_info.server_info.version, "1.0.0");

    // Test backend capabilities
    let tools = backend
        .list_tools(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert_eq!(tools.tools.len(), 1);
    assert_eq!(tools.tools[0].name, "integration_tool");

    let resources = backend
        .list_resources(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert_eq!(resources.resources.len(), 1);
    assert_eq!(resources.resources[0].name, "integration_resource");

    // Test tool execution in the integration context
    let call_result = backend
        .call_tool(CallToolRequestParam {
            name: "integration_tool".to_string(),
            arguments: Some(serde_json::json!({"input": "full integration test"})),
        })
        .await
        .unwrap();

    assert_eq!(call_result.is_error, Some(false));
    match &call_result.content[0] {
        Content::Text { text, .. } => {
            assert!(text.contains("Full Integration Backend"));
            assert!(text.contains("integration_tool"));
            assert!(text.contains("full integration test"));
        }
        _ => panic!("Expected text content"),
    }
}
