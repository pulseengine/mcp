//! MCP Conformance Test Server
//!
//! This server implements all fixtures required by the official
//! `@modelcontextprotocol/conformance` test suite.
//!
//! Run with: cargo run --bin conformance-server
//! Test with: npx @modelcontextprotocol/conformance server --url http://localhost:3000/mcp

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::common_backend::CommonMcpError;
use pulseengine_mcp_server::{
    try_current_context, CreateMessageRequest, ElicitationRequest, McpBackend, McpServer,
    SamplingContent, SamplingMessage, SamplingRole, ServerConfig, TransportConfig,
};

/// Minimal 1x1 red PNG image (base64 encoded)
const MINIMAL_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";

/// Minimal WAV audio (44 bytes: RIFF header + minimal data)
fn minimal_wav_base64() -> String {
    // Minimal valid WAV: 44-byte header with 0 data samples
    let wav_bytes: Vec<u8> = vec![
        0x52, 0x49, 0x46, 0x46, // "RIFF"
        0x24, 0x00, 0x00, 0x00, // File size - 8
        0x57, 0x41, 0x56, 0x45, // "WAVE"
        0x66, 0x6D, 0x74, 0x20, // "fmt "
        0x10, 0x00, 0x00, 0x00, // Subchunk1Size (16)
        0x01, 0x00, // AudioFormat (1 = PCM)
        0x01, 0x00, // NumChannels (1)
        0x44, 0xAC, 0x00, 0x00, // SampleRate (44100)
        0x88, 0x58, 0x01, 0x00, // ByteRate
        0x02, 0x00, // BlockAlign
        0x10, 0x00, // BitsPerSample (16)
        0x64, 0x61, 0x74, 0x61, // "data"
        0x00, 0x00, 0x00, 0x00, // Subchunk2Size (0)
    ];
    BASE64.encode(&wav_bytes)
}

#[derive(Clone)]
struct ConformanceBackend;

#[async_trait]
impl McpBackend for ConformanceBackend {
    type Error = CommonMcpError;
    type Config = ();

    async fn initialize(_config: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(Self)
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .enable_logging()
                .build(),
            server_info: Implementation::new("MCP Conformance Test Server", "1.0.0"),
            instructions: Some("Conformance test server for MCP protocol validation".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    // ==================== TOOLS ====================

    async fn list_tools(
        &self,
        _params: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        let empty_schema = serde_json::json!({
            "type": "object",
            "properties": {}
        });

        Ok(ListToolsResult {
            tools: vec![
                // tools-call-simple-text
                Tool {
                    name: "test_simple_text".to_string(),
                    title: Some("Simple Text Tool".to_string()),
                    description: "Returns simple text content".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-image
                Tool {
                    name: "test_image_content".to_string(),
                    title: Some("Image Content Tool".to_string()),
                    description: "Returns image content".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-audio
                Tool {
                    name: "test_audio_content".to_string(),
                    title: Some("Audio Content Tool".to_string()),
                    description: "Returns audio content".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-embedded-resource
                Tool {
                    name: "test_embedded_resource".to_string(),
                    title: Some("Embedded Resource Tool".to_string()),
                    description: "Returns embedded resource content".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-mixed-content (test_multiple_content_types)
                Tool {
                    name: "test_multiple_content_types".to_string(),
                    title: Some("Multiple Content Types Tool".to_string()),
                    description: "Returns multiple content types".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-with-logging
                Tool {
                    name: "test_tool_with_logging".to_string(),
                    title: Some("Tool With Logging".to_string()),
                    description: "Tool that emits log notifications".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-error
                Tool {
                    name: "test_error_handling".to_string(),
                    title: Some("Error Handling Tool".to_string()),
                    description: "Tool that returns an error".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-with-progress
                Tool {
                    name: "test_tool_with_progress".to_string(),
                    title: Some("Tool With Progress".to_string()),
                    description: "Tool that emits progress notifications".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-sampling
                Tool {
                    name: "test_sampling".to_string(),
                    title: Some("Sampling Tool".to_string()),
                    description: "Tool that requests LLM sampling".to_string(),
                    input_schema: empty_schema.clone(),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
                // tools-call-elicitation
                Tool {
                    name: "test_elicitation".to_string(),
                    title: Some("Elicitation Tool".to_string()),
                    description: "Tool that requests user input".to_string(),
                    input_schema: empty_schema,
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    execution: None,
                    _meta: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        match request.name.as_str() {
            "test_simple_text" => Ok(CallToolResult {
                content: vec![Content::text(
                    "This is a simple text response from the test tool.",
                )],
                is_error: Some(false),
                structured_content: None,
                _meta: None,
            }),

            "test_image_content" => Ok(CallToolResult {
                content: vec![Content::image(MINIMAL_PNG, "image/png")],
                is_error: Some(false),
                structured_content: None,
                _meta: None,
            }),

            "test_audio_content" => Ok(CallToolResult {
                content: vec![Content::audio(minimal_wav_base64(), "audio/wav")],
                is_error: Some(false),
                structured_content: None,
                _meta: None,
            }),

            "test_embedded_resource" => Ok(CallToolResult {
                content: vec![Content::resource(
                    "test://static-text",
                    Some("text/plain".to_string()),
                    Some("Embedded resource content".to_string()),
                )],
                is_error: Some(false),
                structured_content: None,
                _meta: None,
            }),

            "test_multiple_content_types" => Ok(CallToolResult {
                content: vec![
                    Content::text("Text content"),
                    Content::image(MINIMAL_PNG, "image/png"),
                    Content::resource(
                        "test://static-text",
                        Some("text/plain".to_string()),
                        Some("Resource content".to_string()),
                    ),
                ],
                is_error: Some(false),
                structured_content: None,
                _meta: None,
            }),

            "test_tool_with_logging" => {
                // Send log notifications if context is available
                if let Some(ctx) = try_current_context() {
                    // Send a few log messages at different levels
                    let _ = ctx
                        .send_log(
                            LogLevel::Info,
                            Some("conformance_test"),
                            serde_json::json!({"message": "Starting tool execution"}),
                        )
                        .await;
                    let _ = ctx
                        .send_log(
                            LogLevel::Debug,
                            Some("conformance_test"),
                            serde_json::json!({"step": 1, "action": "processing"}),
                        )
                        .await;
                    let _ = ctx
                        .send_log(
                            LogLevel::Info,
                            Some("conformance_test"),
                            serde_json::json!({"message": "Tool execution completed"}),
                        )
                        .await;
                }
                Ok(CallToolResult {
                    content: vec![Content::text("Tool executed with logging")],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }

            "test_error_handling" => Ok(CallToolResult {
                content: vec![Content::text("This is an error message from the tool")],
                is_error: Some(true),
                structured_content: None,
                _meta: None,
            }),

            "test_tool_with_progress" => {
                // Send progress notifications if context is available
                if let Some(ctx) = try_current_context() {
                    // Simulate progress over a few steps
                    let total = 10u64;
                    for i in 0..=total {
                        let _ = ctx.send_progress(i, Some(total)).await;
                        // Small delay to make progress visible in tests
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
                Ok(CallToolResult {
                    content: vec![Content::text("Tool executed with progress")],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }

            "test_sampling" => {
                // Request LLM sampling if context is available
                if let Some(ctx) = try_current_context() {
                    let sampling_request = CreateMessageRequest {
                        messages: vec![SamplingMessage {
                            role: SamplingRole::User,
                            content: SamplingContent::Text {
                                text: "What is 2 + 2? Answer with just the number.".to_string(),
                            },
                        }],
                        system_prompt: Some("You are a helpful assistant.".to_string()),
                        max_tokens: 100,
                        temperature: Some(0.0),
                        ..Default::default()
                    };

                    match ctx
                        .request_sampling(sampling_request, std::time::Duration::from_secs(30))
                        .await
                    {
                        Ok(response) => {
                            let response_text = match &response.content {
                                SamplingContent::Text { text } => text.clone(),
                                SamplingContent::Image { .. } => "Image response".to_string(),
                            };
                            return Ok(CallToolResult {
                                content: vec![Content::text(format!(
                                    "LLM response: {} (model: {})",
                                    response_text, response.model
                                ))],
                                is_error: Some(false),
                                structured_content: None,
                                _meta: None,
                            });
                        }
                        Err(e) => {
                            return Ok(CallToolResult {
                                content: vec![Content::text(format!("Sampling error: {e}"))],
                                is_error: Some(true),
                                structured_content: None,
                                _meta: None,
                            });
                        }
                    }
                }
                // No context available - return a message indicating this
                Ok(CallToolResult {
                    content: vec![Content::text("Sampling not available (no context)")],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }

            "test_elicitation" => {
                // Request user input via elicitation if context is available
                if let Some(ctx) = try_current_context() {
                    let elicitation_request = ElicitationRequest {
                        message: "Please provide your name:".to_string(),
                        requested_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "description": "Your name"
                                }
                            },
                            "required": ["name"]
                        }),
                        meta: None,
                    };

                    match ctx
                        .request_elicitation(
                            elicitation_request,
                            std::time::Duration::from_secs(60),
                        )
                        .await
                    {
                        Ok(response) => {
                            let user_input = response
                                .content
                                .as_ref()
                                .and_then(|c| c.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let action = match &response.action {
                                pulseengine_mcp_server::ElicitationAction::Accept => "accepted",
                                pulseengine_mcp_server::ElicitationAction::Decline => "declined",
                                pulseengine_mcp_server::ElicitationAction::Cancel => "cancelled",
                            };
                            return Ok(CallToolResult {
                                content: vec![Content::text(format!(
                                    "User {action} with name: {user_input}"
                                ))],
                                is_error: Some(false),
                                structured_content: None,
                                _meta: None,
                            });
                        }
                        Err(e) => {
                            return Ok(CallToolResult {
                                content: vec![Content::text(format!("Elicitation error: {e}"))],
                                is_error: Some(true),
                                structured_content: None,
                                _meta: None,
                            });
                        }
                    }
                }
                // No context available - return a message indicating this
                Ok(CallToolResult {
                    content: vec![Content::text("Elicitation not available (no context)")],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }

            _ => Err(CommonMcpError::InvalidParams(format!(
                "Unknown tool: {}",
                request.name
            ))),
        }
    }

    // ==================== RESOURCES ====================

    async fn list_resources(
        &self,
        _params: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult {
            resources: vec![
                // resources-read-text
                Resource {
                    uri: "test://static-text".to_string(),
                    name: "Static Text Resource".to_string(),
                    title: None,
                    description: Some("A static text resource for conformance testing".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    annotations: None,
                    icons: None,
                    raw: None,
                    _meta: None,
                },
                // resources-read-binary
                Resource {
                    uri: "test://static-binary".to_string(),
                    name: "Static Binary Resource".to_string(),
                    title: None,
                    description: Some("A static binary resource (PNG image)".to_string()),
                    mime_type: Some("image/png".to_string()),
                    annotations: None,
                    icons: None,
                    raw: None,
                    _meta: None,
                },
                // resources-subscribe / resources-unsubscribe
                Resource {
                    uri: "test://watched-resource".to_string(),
                    name: "Watched Resource".to_string(),
                    title: None,
                    description: Some("A resource that can be subscribed to".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    annotations: None,
                    icons: None,
                    raw: None,
                    _meta: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _params: PaginatedRequestParam,
    ) -> std::result::Result<ListResourceTemplatesResult, Self::Error> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![
                // resources-templates-read
                ResourceTemplate {
                    uri_template: "test://template/{id}/data".to_string(),
                    name: "Template Resource".to_string(),
                    description: Some("A parameterized template resource".to_string()),
                    mime_type: Some("text/plain".to_string()),
                },
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        params: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        let uri = &params.uri;

        // Handle static resources
        if uri == "test://static-text" {
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents {
                    uri: uri.clone(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some("This is the content of the static text resource.".to_string()),
                    blob: None,
                    _meta: None,
                }],
            });
        }

        if uri == "test://static-binary" {
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents {
                    uri: uri.clone(),
                    mime_type: Some("image/png".to_string()),
                    text: None,
                    blob: Some(MINIMAL_PNG.to_string()),
                    _meta: None,
                }],
            });
        }

        if uri == "test://watched-resource" {
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents {
                    uri: uri.clone(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some("Watched resource content".to_string()),
                    blob: None,
                    _meta: None,
                }],
            });
        }

        // Handle template resources: test://template/{id}/data
        if uri.starts_with("test://template/") && uri.ends_with("/data") {
            let id = uri
                .strip_prefix("test://template/")
                .and_then(|s| s.strip_suffix("/data"))
                .unwrap_or("unknown");

            return Ok(ReadResourceResult {
                contents: vec![ResourceContents {
                    uri: uri.clone(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some(format!("Template resource data for id: {id}")),
                    blob: None,
                    _meta: None,
                }],
            });
        }

        Err(CommonMcpError::InvalidParams(format!(
            "Resource not found: {uri}"
        )))
    }

    // ==================== PROMPTS ====================

    async fn list_prompts(
        &self,
        _params: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult {
            prompts: vec![
                // prompts-get-simple
                Prompt {
                    name: "test_simple_prompt".to_string(),
                    title: Some("Simple Test Prompt".to_string()),
                    description: Some("A simple prompt without arguments".to_string()),
                    arguments: None,
                    icons: None,
                },
                // prompts-get-with-args
                Prompt {
                    name: "test_prompt_with_arguments".to_string(),
                    title: Some("Prompt With Arguments".to_string()),
                    description: Some("A prompt that requires arguments".to_string()),
                    arguments: Some(vec![
                        PromptArgument {
                            name: "arg1".to_string(),
                            description: Some("First argument".to_string()),
                            required: Some(true),
                        },
                        PromptArgument {
                            name: "arg2".to_string(),
                            description: Some("Second argument".to_string()),
                            required: Some(true),
                        },
                    ]),
                    icons: None,
                },
                // prompts-get-embedded-resource
                Prompt {
                    name: "test_prompt_with_embedded_resource".to_string(),
                    title: Some("Prompt With Embedded Resource".to_string()),
                    description: Some("A prompt that includes an embedded resource".to_string()),
                    arguments: Some(vec![PromptArgument {
                        name: "resourceUri".to_string(),
                        description: Some("URI of the resource to embed".to_string()),
                        required: Some(true),
                    }]),
                    icons: None,
                },
                // prompts-get-with-image
                Prompt {
                    name: "test_prompt_with_image".to_string(),
                    title: Some("Prompt With Image".to_string()),
                    description: Some("A prompt that includes an image".to_string()),
                    arguments: None,
                    icons: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        params: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        match params.name.as_str() {
            "test_simple_prompt" => Ok(GetPromptResult {
                description: Some("A simple test prompt".to_string()),
                messages: vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    "This is a simple test prompt message.",
                )],
            }),

            "test_prompt_with_arguments" => {
                let arg1 = params
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("arg1"))
                    .map(|s| s.as_str())
                    .unwrap_or("default1");
                let arg2 = params
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("arg2"))
                    .map(|s| s.as_str())
                    .unwrap_or("default2");

                Ok(GetPromptResult {
                    description: Some("A prompt with arguments".to_string()),
                    messages: vec![PromptMessage::new_text(
                        PromptMessageRole::User,
                        format!("Prompt with arg1={arg1} and arg2={arg2}"),
                    )],
                })
            }

            "test_prompt_with_embedded_resource" => {
                // Return actual embedded resource content
                let resource_uri = params
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("resourceUri"))
                    .map(|s| s.as_str())
                    .unwrap_or("test://static-text");

                Ok(GetPromptResult {
                    description: Some("A prompt with an embedded resource".to_string()),
                    messages: vec![PromptMessage::new_resource(
                        PromptMessageRole::User,
                        resource_uri,
                        Some("text/plain".to_string()),
                        Some("This is the embedded resource content.".to_string()),
                    )],
                })
            }

            "test_prompt_with_image" => Ok(GetPromptResult {
                description: Some("A prompt with an image".to_string()),
                messages: vec![PromptMessage::new_image(
                    PromptMessageRole::User,
                    MINIMAL_PNG,
                    "image/png",
                )],
            }),

            _ => Err(CommonMcpError::InvalidParams(format!(
                "Unknown prompt: {}",
                params.name
            ))),
        }
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let backend = ConformanceBackend::initialize(()).await?;

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let mut config = ServerConfig::default();
    config.auth_config.enabled = false;
    config.transport_config = TransportConfig::StreamableHttp { port, host: None };

    let mut server = McpServer::new(backend, config).await?;

    eprintln!("MCP Conformance Test Server running on http://localhost:{port}");
    eprintln!();
    eprintln!("Test with:");
    eprintln!("  npx @modelcontextprotocol/conformance server --url http://localhost:{port}/mcp");
    eprintln!();

    server.run().await?;
    Ok(())
}
