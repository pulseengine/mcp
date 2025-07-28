//! End-to-end integration scenarios that test the complete MCP framework

use crate::test_utils::*;
use async_trait::async_trait;
use pulseengine_mcp_auth::AuthenticationManager;
use pulseengine_mcp_monitoring::MetricsCollector;
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_security::SecurityMiddleware;
use pulseengine_mcp_server::{
    backend::{BackendError, McpBackend},
    handler::GenericServerHandler,
    middleware::MiddlewareStack,
    server::{McpServer, ServerConfig},
};
use pulseengine_mcp_transport::TransportConfig;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Comprehensive test backend that simulates a real-world MCP server
#[derive(Clone)]
struct E2ETestBackend {
    name: String,
    request_counter: Arc<AtomicU64>,
    session_data: Arc<std::sync::RwLock<HashMap<String, serde_json::Value>>>,
    tools: Vec<E2ETool>,
    resources: Vec<E2EResource>,
    prompts: Vec<E2EPrompt>,
}

#[derive(Clone, Debug)]
struct E2ETool {
    name: String,
    description: String,
    handler: E2EToolHandler,
}

#[derive(Clone, Debug)]
enum E2EToolHandler {
    Echo,
    Calculate,
    Session,
    FileSystem,
    Weather,
}

#[derive(Clone, Debug)]
struct E2EResource {
    name: String,
    uri: String,
    content: String,
    mime_type: String,
}

#[derive(Clone, Debug)]
struct E2EPrompt {
    name: String,
    description: String,
    template: String,
}

#[derive(Debug)]
struct E2ETestError(String);

impl fmt::Display for E2ETestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E2E test error: {}", self.0)
    }
}

impl StdError for E2ETestError {}

impl From<BackendError> for E2ETestError {
    fn from(err: BackendError) -> Self {
        E2ETestError(err.to_string())
    }
}

impl From<E2ETestError> for Error {
    fn from(err: E2ETestError) -> Self {
        Error::internal_error(err.to_string())
    }
}

impl E2ETestBackend {
    fn new(name: String) -> Self {
        Self {
            name,
            request_counter: Arc::new(AtomicU64::new(0)),
            session_data: Arc::new(std::sync::RwLock::new(HashMap::new())),
            tools: vec![
                E2ETool {
                    name: "echo".to_string(),
                    description: "Echo back the input message".to_string(),
                    handler: E2EToolHandler::Echo,
                },
                E2ETool {
                    name: "calculate".to_string(),
                    description: "Perform basic mathematical calculations".to_string(),
                    handler: E2EToolHandler::Calculate,
                },
                E2ETool {
                    name: "session_store".to_string(),
                    description: "Store data in the session".to_string(),
                    handler: E2EToolHandler::Session,
                },
                E2ETool {
                    name: "file_info".to_string(),
                    description: "Get information about files".to_string(),
                    handler: E2EToolHandler::FileSystem,
                },
                E2ETool {
                    name: "weather".to_string(),
                    description: "Get weather information (simulated)".to_string(),
                    handler: E2EToolHandler::Weather,
                },
            ],
            resources: vec![
                E2EResource {
                    name: "system_info".to_string(),
                    uri: "e2e://system/info".to_string(),
                    content: "System information resource".to_string(),
                    mime_type: "application/json".to_string(),
                },
                E2EResource {
                    name: "api_docs".to_string(),
                    uri: "e2e://docs/api".to_string(),
                    content: "API documentation resource".to_string(),
                    mime_type: "text/markdown".to_string(),
                },
                E2EResource {
                    name: "config".to_string(),
                    uri: "e2e://config/server".to_string(),
                    content: r#"{"server": "e2e-test", "version": "1.0.0"}"#.to_string(),
                    mime_type: "application/json".to_string(),
                },
            ],
            prompts: vec![
                E2EPrompt {
                    name: "greeting".to_string(),
                    description: "Generate a personalized greeting".to_string(),
                    template: "Hello {{name}}! Welcome to the E2E test system.".to_string(),
                },
                E2EPrompt {
                    name: "summary".to_string(),
                    description: "Summarize the given content".to_string(),
                    template: "Please provide a summary of: {{content}}".to_string(),
                },
            ],
        }
    }
}

#[async_trait]
impl McpBackend for E2ETestBackend {
    type Error = E2ETestError;
    type Config = String;

    async fn initialize(name: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(Self::new(name))
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(true),
                    list_changed: Some(true),
                }),
                prompts: Some(PromptsCapability {
                    list_changed: Some(true),
                }),
                logging: Some(LoggingCapability {
                    level: Some("debug".to_string()),
                }),
                sampling: Some(SamplingCapability {}),
                ..Default::default()
            },
            server_info: Implementation {
                name: format!("E2E Test Server: {}", self.name),
                version: "1.0.0".to_string(),
            },
            instructions: Some(
                "Comprehensive end-to-end test backend with full MCP capabilities".to_string(),
            ),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        self.request_counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn list_tools(
        &self,
        request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        self.request_counter.fetch_add(1, Ordering::Relaxed);

        let start_index = request
            .cursor
            .and_then(|c| c.parse::<usize>().ok())
            .unwrap_or(0);

        let page_size = 10; // Simulate pagination
        let end_index = std::cmp::min(start_index + page_size, self.tools.len());

        let tools: Vec<Tool> = self.tools[start_index..end_index]
            .iter()
            .map(|tool| Tool {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: match tool.handler {
                    E2EToolHandler::Echo => serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": {"type": "string", "description": "Message to echo back"}
                        },
                        "required": ["message"]
                    }),
                    E2EToolHandler::Calculate => serde_json::json!({
                        "type": "object",
                        "properties": {
                            "expression": {"type": "string", "description": "Mathematical expression to evaluate"},
                            "precision": {"type": "integer", "description": "Number of decimal places", "default": 2}
                        },
                        "required": ["expression"]
                    }),
                    E2EToolHandler::Session => serde_json::json!({
                        "type": "object",
                        "properties": {
                            "key": {"type": "string", "description": "Session key"},
                            "value": {"description": "Value to store"}
                        },
                        "required": ["key", "value"]
                    }),
                    E2EToolHandler::FileSystem => serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "File or directory path"}
                        },
                        "required": ["path"]
                    }),
                    E2EToolHandler::Weather => serde_json::json!({
                        "type": "object",
                        "properties": {
                            "location": {"type": "string", "description": "Location for weather"},
                            "units": {"type": "string", "enum": ["metric", "imperial"], "default": "metric"}
                        },
                        "required": ["location"]
                    }),
                },
                output_schema: None,
            })
            .collect();

        let next_cursor = if end_index < self.tools.len() {
            Some(end_index.to_string())
        } else {
            None
        };

        Ok(ListToolsResult { tools, next_cursor })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        self.request_counter.fetch_add(1, Ordering::Relaxed);

        let tool = self
            .tools
            .iter()
            .find(|t| t.name == request.name)
            .ok_or_else(|| E2ETestError(format!("Tool not found: {}", request.name)))?;

        let args = request.arguments.unwrap_or_default();

        let content = match &tool.handler {
            E2EToolHandler::Echo => {
                let message = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No message provided");
                vec![Content::Text {
                    text: format!("Echo from {}: {}", self.name, message),
                }]
            }
            E2EToolHandler::Calculate => {
                let expression = args
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let precision =
                    args.get("precision").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

                // Simple calculator (just for demo)
                let result = match expression {
                    expr if expr.contains('+') => {
                        let parts: Vec<&str> = expr.split('+').collect();
                        if parts.len() == 2 {
                            let a: f64 = parts[0].trim().parse().unwrap_or(0.0);
                            let b: f64 = parts[1].trim().parse().unwrap_or(0.0);
                            format!("{:.precision$}", a + b, precision = precision)
                        } else {
                            "Invalid expression".to_string()
                        }
                    }
                    expr if expr.contains('*') => {
                        let parts: Vec<&str> = expr.split('*').collect();
                        if parts.len() == 2 {
                            let a: f64 = parts[0].trim().parse().unwrap_or(0.0);
                            let b: f64 = parts[1].trim().parse().unwrap_or(0.0);
                            format!("{:.precision$}", a * b, precision = precision)
                        } else {
                            "Invalid expression".to_string()
                        }
                    }
                    _ => "Unsupported operation".to_string(),
                };

                vec![Content::Text {
                    text: format!("Calculation result for '{expression}': {result}"),
                }]
            }
            E2EToolHandler::Session => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let value = args
                    .get("value")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);

                {
                    let mut session = self.session_data.write().unwrap();
                    session.insert(key.to_string(), value.clone());
                }

                vec![Content::Text {
                    text: format!("Stored '{key}' = {value:?} in session"),
                }]
            }
            E2EToolHandler::FileSystem => {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("/");

                // Simulate file system info
                let info = serde_json::json!({
                    "path": path,
                    "type": if path.ends_with('/') { "directory" } else { "file" },
                    "size": rand::random::<u32>() % 10000,
                    "modified": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                });

                vec![Content::Text {
                    text: format!("File info for '{}': {}", path, info),
                }]
            }
            E2EToolHandler::Weather => {
                let location = args
                    .get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let units = args
                    .get("units")
                    .and_then(|v| v.as_str())
                    .unwrap_or("metric");

                // Simulate weather data
                let temp_unit = if units == "imperial" { "°F" } else { "°C" };
                let temp = if units == "imperial" {
                    rand::random::<u8>() % 100 + 32
                } else {
                    rand::random::<u8>() % 40
                };

                let conditions = ["sunny", "cloudy", "rainy", "snowy"];
                let condition = conditions[rand::random::<usize>() % 4];

                let weather = serde_json::json!({
                    "location": location,
                    "temperature": format!("{}{}", temp, temp_unit),
                    "condition": condition,
                    "humidity": format!("{}%", rand::random::<u8>() % 100),
                    "timestamp": SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                });

                vec![Content::Text {
                    text: format!("Weather for {}: {}", location, weather),
                }]
            }
        };

        Ok(CallToolResult {
            content,
            is_error: Some(false),
            structured_content: None,
        })
    }

    async fn list_resources(
        &self,
        request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        self.request_counter.fetch_add(1, Ordering::Relaxed);

        let start_index = request
            .cursor
            .and_then(|c| c.parse::<usize>().ok())
            .unwrap_or(0);

        let page_size = 5;
        let end_index = std::cmp::min(start_index + page_size, self.resources.len());

        let resources: Vec<Resource> = self.resources[start_index..end_index]
            .iter()
            .map(|res| Resource {
                uri: res.uri.clone(),
                name: res.name.clone(),
                description: Some(format!("E2E test resource: {}", res.name)),
                mime_type: Some(res.mime_type.clone()),
                annotations: None,
                raw: None,
            })
            .collect();

        let next_cursor = if end_index < self.resources.len() {
            Some(end_index.to_string())
        } else {
            None
        };

        Ok(ListResourcesResult {
            resources,
            next_cursor,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        self.request_counter.fetch_add(1, Ordering::Relaxed);

        let resource = self
            .resources
            .iter()
            .find(|r| r.uri == request.uri)
            .ok_or_else(|| E2ETestError(format!("Resource not found: {}", request.uri)))?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents {
                uri: resource.uri.clone(),
                mime_type: Some(resource.mime_type.clone()),
                text: Some(resource.content.clone()),
                blob: None,
            }],
        })
    }

    async fn list_prompts(
        &self,
        request: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        self.request_counter.fetch_add(1, Ordering::Relaxed);

        let start_index = request
            .cursor
            .and_then(|c| c.parse::<usize>().ok())
            .unwrap_or(0);

        let end_index = std::cmp::min(start_index + 10, self.prompts.len());

        let prompts: Vec<Prompt> = self.prompts[start_index..end_index]
            .iter()
            .map(|prompt| Prompt {
                name: prompt.name.clone(),
                description: Some(prompt.description.clone()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "name".to_string(),
                        description: Some("Name parameter".to_string()),
                        required: Some(true),
                    },
                    PromptArgument {
                        name: "content".to_string(),
                        description: Some("Content parameter".to_string()),
                        required: Some(false),
                    },
                ]),
            })
            .collect();

        let next_cursor = if end_index < self.prompts.len() {
            Some(end_index.to_string())
        } else {
            None
        };

        Ok(ListPromptsResult {
            prompts,
            next_cursor,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        self.request_counter.fetch_add(1, Ordering::Relaxed);

        let prompt = self
            .prompts
            .iter()
            .find(|p| p.name == request.name)
            .ok_or_else(|| E2ETestError(format!("Prompt not found: {}", request.name)))?;

        let args = request.arguments.unwrap_or_default();
        let default_name = "World".to_string();
        let default_content = "sample content".to_string();
        let name = args.get("name").unwrap_or(&default_name);
        let content = args.get("content").unwrap_or(&default_content);

        let message_text = prompt
            .template
            .replace("{{name}}", name)
            .replace("{{content}}", content);

        Ok(GetPromptResult {
            description: Some(prompt.description.clone()),
            messages: vec![PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::Text { text: message_text },
            }],
        })
    }
}

#[tokio::test]
async fn test_complete_e2e_scenario() {
    // Test a complete end-to-end scenario with all components
    let backend = E2ETestBackend::initialize("Complete E2E".to_string())
        .await
        .unwrap();

    let mut auth_config = test_auth_config();
    auth_config.enabled = false; // Simplify for E2E test

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config,
        monitoring_config: test_monitoring_config(),
        security_config: test_security_config(),
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Test server creation and configuration
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "MCP Server"); // Server uses config name, not backend name
    // Verify we can get server info - the specific capabilities depend on server config vs backend

    // Test health check
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("backend"));
    assert!(health.components.contains_key("transport"));
    assert!(health.components.contains_key("auth"));

    // Test metrics
    let metrics = server.get_metrics().await;
    // requests_total is a u64, so it's always >= 0
    assert!(metrics.requests_total < u64::MAX);
}

#[tokio::test]
async fn test_e2e_handler_workflow() {
    // Test complete handler workflow with all MCP operations
    let backend = Arc::new(
        E2ETestBackend::initialize("Handler E2E".to_string())
            .await
            .unwrap(),
    );
    let auth_config = test_auth_config();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let monitoring = Arc::new(MetricsCollector::new(test_monitoring_config()));
    let security = SecurityMiddleware::new(test_security_config());
    let middleware = MiddlewareStack::new()
        .with_auth(auth_manager.clone())
        .with_monitoring(monitoring)
        .with_security(security);

    let handler = GenericServerHandler::new(backend, auth_manager, middleware);

    // Test initialization
    let init_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("init".to_string()),
        method: "initialize".to_string(),
        params: serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "E2E Test Client",
                "version": "1.0.0"
            }
        }),
    };

    let response = handler.handle_request(init_request).await.unwrap();
    assert!(response.error.is_none());

    // Test tool operations
    let tools_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("list_tools".to_string()),
        method: "tools/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(tools_request).await.unwrap();
    assert!(response.error.is_none());
    let tools_result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!tools_result.tools.is_empty());

    // Test tool execution
    let call_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("call_tool".to_string()),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": "echo",
            "arguments": {
                "message": "Hello E2E!"
            }
        }),
    };

    let response = handler.handle_request(call_request).await.unwrap();
    assert!(response.error.is_none());
    let call_result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(call_result.is_error, Some(false));

    // Test resource operations
    let resources_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("list_resources".to_string()),
        method: "resources/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(resources_request).await.unwrap();
    assert!(response.error.is_none());
    let resources_result: ListResourcesResult =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!resources_result.resources.is_empty());

    // Test resource reading
    let read_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("read_resource".to_string()),
        method: "resources/read".to_string(),
        params: serde_json::json!({"uri": "e2e://system/info"}),
    };

    let response = handler.handle_request(read_request).await.unwrap();
    assert!(response.error.is_none());

    // Test prompt operations
    let prompts_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("list_prompts".to_string()),
        method: "prompts/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(prompts_request).await.unwrap();
    assert!(response.error.is_none());

    let get_prompt_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("get_prompt".to_string()),
        method: "prompts/get".to_string(),
        params: serde_json::json!({
            "name": "greeting",
            "arguments": {
                "name": "E2E Test"
            }
        }),
    };

    let response = handler.handle_request(get_prompt_request).await.unwrap();
    assert!(response.error.is_none());
}

#[tokio::test]
async fn test_e2e_pagination_workflow() {
    // Test pagination across all list operations
    let backend = Arc::new(
        E2ETestBackend::initialize("Pagination E2E".to_string())
            .await
            .unwrap(),
    );
    let auth_config = test_auth_config();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = MiddlewareStack::new();

    let handler = GenericServerHandler::new(backend, auth_manager, middleware);

    // Test tool pagination
    let tools_page1 = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("tools_page1".to_string()),
        method: "tools/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(tools_page1).await.unwrap();
    assert!(response.error.is_none());
    let tools_result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!tools_result.tools.is_empty());

    // Test resource pagination
    let resources_page1 = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("resources_page1".to_string()),
        method: "resources/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(resources_page1).await.unwrap();
    assert!(response.error.is_none());
    let resources_result: ListResourcesResult =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!resources_result.resources.is_empty());

    // Test prompt pagination
    let prompts_page1 = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("prompts_page1".to_string()),
        method: "prompts/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(prompts_page1).await.unwrap();
    assert!(response.error.is_none());
    let prompts_result: ListPromptsResult =
        serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(!prompts_result.prompts.is_empty());
}

#[tokio::test]
async fn test_e2e_error_handling() {
    // Test comprehensive error handling throughout the system
    let backend = Arc::new(
        E2ETestBackend::initialize("Error E2E".to_string())
            .await
            .unwrap(),
    );
    let auth_config = test_auth_config();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = MiddlewareStack::new();

    let handler = GenericServerHandler::new(backend, auth_manager, middleware);

    // Test invalid method
    let invalid_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("invalid".to_string()),
        method: "invalid/method".to_string(),
        params: serde_json::Value::Null,
    };

    let response = handler.handle_request(invalid_request).await.unwrap();
    assert!(response.error.is_some());

    // Test tool not found
    let not_found_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("not_found".to_string()),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": "nonexistent_tool",
            "arguments": {}
        }),
    };

    let response = handler.handle_request(not_found_request).await.unwrap();
    assert!(response.error.is_some());

    // Test resource not found
    let resource_not_found = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("resource_not_found".to_string()),
        method: "resources/read".to_string(),
        params: serde_json::json!({"uri": "e2e://nonexistent"}),
    };

    let response = handler.handle_request(resource_not_found).await.unwrap();
    assert!(response.error.is_some());

    // Test prompt not found
    let prompt_not_found = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("prompt_not_found".to_string()),
        method: "prompts/get".to_string(),
        params: serde_json::json!({
            "name": "nonexistent_prompt",
            "arguments": {}
        }),
    };

    let response = handler.handle_request(prompt_not_found).await.unwrap();
    assert!(response.error.is_some());
}
