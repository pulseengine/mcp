//! Type conversions between WIT-generated types and pulseengine-mcp-protocol types
//!
//! This module provides conversion functions to bridge WIT types (generated from
//! the wasi-mcp WIT definitions) and the existing pulseengine-mcp-protocol types.

use crate::host::wasi::mcp::{capabilities, content, runtime};
use crate::error::Result;
use pulseengine_mcp_protocol::model;

/// Convert WIT ToolDefinition to mcp-protocol Tool
pub fn tool_definition_to_tool(wit: runtime::ToolDefinition) -> Result<model::Tool> {
    // Parse input schema from JSON bytes
    let input_schema = serde_json::from_slice(&wit.input_schema)
        .map_err(|e| crate::Error::invalid_params(format!("Invalid tool input schema: {}", e)))?;

    // Parse optional output schema
    let output_schema = wit.output_schema
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()
        .map_err(|e| crate::Error::invalid_params(format!("Invalid tool output schema: {}", e)))?;

    Ok(model::Tool {
        name: wit.name,
        title: wit.title,
        description: wit.description,
        input_schema,
        output_schema,
        annotations: wit.annotations.map(tool_annotations_to_annotations),
        icons: None, // TODO: Add icons support to WIT
    })
}

/// Convert WIT ToolAnnotations to mcp-protocol ToolAnnotations
fn tool_annotations_to_annotations(wit: runtime::ToolAnnotations) -> model::ToolAnnotations {
    model::ToolAnnotations {
        read_only_hint: wit.read_only_hint,
        destructive_hint: wit.destructive_hint,
        idempotent_hint: wit.idempotent_hint,
        open_world_hint: wit.open_world_hint,
    }
}

/// Convert WIT ResourceDefinition to mcp-protocol Resource
pub fn resource_definition_to_resource(wit: runtime::ResourceDefinition) -> model::Resource {
    model::Resource {
        uri: wit.uri,
        name: wit.name,
        title: wit.title,
        description: wit.description,
        mime_type: wit.mime_type,
        annotations: None, // TODO: Add annotations support
        icons: None, // TODO: Add icons support to WIT
        raw: None, // Raw resource content not available in definition
    }
}

/// Convert WIT ResourceTemplate to mcp-protocol ResourceTemplate
pub fn resource_template_to_template(wit: runtime::ResourceTemplate) -> model::ResourceTemplate {
    model::ResourceTemplate {
        uri_template: wit.uri_template,
        name: wit.name,
        description: wit.description,
        mime_type: wit.mime_type,
    }
}

/// Convert WIT PromptDefinition to mcp-protocol Prompt
pub fn prompt_definition_to_prompt(wit: runtime::PromptDefinition) -> model::Prompt {
    model::Prompt {
        name: wit.name,
        title: wit.title,
        description: wit.description,
        arguments: wit.arguments.map(|args|
            args.into_iter()
                .map(prompt_argument_to_argument)
                .collect()
        ),
        icons: None, // TODO: Add icons support to WIT
    }
}

/// Convert WIT PromptArgument to mcp-protocol PromptArgument
fn prompt_argument_to_argument(wit: runtime::PromptArgument) -> model::PromptArgument {
    model::PromptArgument {
        name: wit.name,
        description: wit.description,
        required: wit.required,
    }
}

/// Convert WIT ServerInfo to mcp-protocol Implementation
pub fn server_info_to_implementation(wit: runtime::ServerInfo) -> model::Implementation {
    model::Implementation {
        name: wit.name,
        version: wit.version,
    }
}

/// Convert WIT ServerInfo to mcp-protocol ServerCapabilities
pub fn server_info_to_capabilities(wit: &runtime::ServerInfo) -> model::ServerCapabilities {
    server_capabilities_to_capabilities(&wit.capabilities)
}

/// Convert WIT ServerCapabilities to mcp-protocol ServerCapabilities
pub fn server_capabilities_to_capabilities(wit: &capabilities::ServerCapabilities) -> model::ServerCapabilities {
    model::ServerCapabilities {
        tools: wit.tools.as_ref().map(|t| model::ToolsCapability {
            list_changed: t.list_changed,
        }),
        resources: wit.resources.as_ref().map(|r| model::ResourcesCapability {
            subscribe: r.subscribe,
            list_changed: r.list_changed,
        }),
        prompts: wit.prompts.as_ref().map(|p| model::PromptsCapability {
            list_changed: p.list_changed,
        }),
        logging: None, // Note: LoggingCapability in mcp-protocol has a level field
        sampling: wit.sampling.map(|_| model::SamplingCapability {}),
        elicitation: wit.elicitation.map(|_| model::ElicitationCapability {}),
    }
}

/// Convert WIT LogLevel to mcp-protocol log level string
pub fn log_level_to_string(wit: content::LogLevel) -> &'static str {
    match wit {
        content::LogLevel::Debug => "debug",
        content::LogLevel::Info => "info",
        content::LogLevel::Warning => "warning",
        content::LogLevel::Error => "error",
        content::LogLevel::Critical => "critical",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_conversions() {
        assert_eq!(log_level_to_string(content::LogLevel::Debug), "debug");
        assert_eq!(log_level_to_string(content::LogLevel::Info), "info");
        assert_eq!(log_level_to_string(content::LogLevel::Warning), "warning");
        assert_eq!(log_level_to_string(content::LogLevel::Error), "error");
        assert_eq!(log_level_to_string(content::LogLevel::Critical), "critical");
    }

    #[test]
    fn test_server_info_conversion() {
        let wit_info = runtime::ServerInfo {
            name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            capabilities: capabilities::ServerCapabilities {
                tools: None,
                resources: None,
                prompts: None,
                logging: None,
                sampling: None,
                elicitation: None,
                completions: None,
                experimental: None,
            },
            instructions: Some("Test instructions".to_string()),
        };

        let implementation = server_info_to_implementation(wit_info.clone());
        assert_eq!(implementation.name, "test-server");
        assert_eq!(implementation.version, "1.0.0");

        let caps = server_info_to_capabilities(&wit_info);
        assert!(caps.tools.is_none());
        assert!(caps.resources.is_none());
        assert!(caps.prompts.is_none());
    }

    #[test]
    fn test_resource_definition_conversion() {
        let wit_resource = runtime::ResourceDefinition {
            uri: "file:///test.txt".to_string(),
            name: "test-resource".to_string(),
            title: Some("Test Resource".to_string()),
            description: Some("A test resource".to_string()),
            mime_type: Some("text/plain".to_string()),
            size: Some(1024),
        };

        let resource = resource_definition_to_resource(wit_resource);
        assert_eq!(resource.uri, "file:///test.txt");
        assert_eq!(resource.name, "test-resource");
        assert_eq!(resource.title, Some("Test Resource".to_string()));
        assert_eq!(resource.mime_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_tool_definition_conversion() {
        let schema_json = serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            }
        });
        let schema_bytes = serde_json::to_vec(&schema_json).unwrap();

        let wit_tool = runtime::ToolDefinition {
            name: "echo".to_string(),
            title: Some("Echo Tool".to_string()),
            description: "Echoes input".to_string(),
            input_schema: schema_bytes,
            output_schema: None,
            annotations: None,
        };

        let tool = tool_definition_to_tool(wit_tool).unwrap();
        assert_eq!(tool.name, "echo");
        assert_eq!(tool.title, Some("Echo Tool".to_string()));
        assert_eq!(tool.description, "Echoes input");
        assert!(tool.input_schema.is_object());
    }

    #[test]
    fn test_prompt_definition_conversion() {
        let wit_prompt = runtime::PromptDefinition {
            name: "greeting".to_string(),
            title: Some("Greeting Prompt".to_string()),
            description: Some("A greeting prompt".to_string()),
            arguments: Some(vec![
                runtime::PromptArgument {
                    name: "name".to_string(),
                    description: Some("User's name".to_string()),
                    required: Some(true),
                }
            ]),
        };

        let prompt = prompt_definition_to_prompt(wit_prompt);
        assert_eq!(prompt.name, "greeting");
        assert_eq!(prompt.title, Some("Greeting Prompt".to_string()));
        assert!(prompt.arguments.is_some());
        assert_eq!(prompt.arguments.as_ref().unwrap().len(), 1);
        assert_eq!(prompt.arguments.as_ref().unwrap()[0].name, "name");
    }
}
