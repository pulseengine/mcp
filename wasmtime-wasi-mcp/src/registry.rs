//! Registry for MCP tools, resources, and prompts

use crate::error::{Error, Result};
use pulseengine_mcp_protocol::model::{Prompt, Resource, Tool};
use std::collections::HashMap;

/// Registry for MCP server capabilities
///
/// Stores tools, resources, and prompts registered by components.
#[derive(Debug, Default)]
pub struct Registry {
    tools: HashMap<String, Tool>,
    resources: HashMap<String, Resource>,
    prompts: HashMap<String, Prompt>,
}

impl Registry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool
    pub fn add_tool(&mut self, tool: Tool) -> Result<()> {
        let name = tool.name.clone();
        if self.tools.contains_key(&name) {
            return Err(Error::invalid_params(format!("Tool '{}' already registered", name)));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Register multiple tools
    pub fn add_tools(&mut self, tools: Vec<Tool>) -> Result<()> {
        for tool in tools {
            self.add_tool(tool)?;
        }
        Ok(())
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// List all tools
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools.values().cloned().collect()
    }

    /// Register a resource
    pub fn add_resource(&mut self, resource: Resource) -> Result<()> {
        let uri = resource.uri.clone();
        if self.resources.contains_key(&uri) {
            return Err(Error::invalid_params(format!("Resource '{}' already registered", uri)));
        }
        self.resources.insert(uri, resource);
        Ok(())
    }

    /// Register multiple resources
    pub fn add_resources(&mut self, resources: Vec<Resource>) -> Result<()> {
        for resource in resources {
            self.add_resource(resource)?;
        }
        Ok(())
    }

    /// Get a resource by URI
    pub fn get_resource(&self, uri: &str) -> Option<&Resource> {
        self.resources.get(uri)
    }

    /// List all resources
    pub fn list_resources(&self) -> Vec<Resource> {
        self.resources.values().cloned().collect()
    }

    /// Register a prompt
    pub fn add_prompt(&mut self, prompt: Prompt) -> Result<()> {
        let name = prompt.name.clone();
        if self.prompts.contains_key(&name) {
            return Err(Error::invalid_params(format!("Prompt '{}' already registered", name)));
        }
        self.prompts.insert(name, prompt);
        Ok(())
    }

    /// Register multiple prompts
    pub fn add_prompts(&mut self, prompts: Vec<Prompt>) -> Result<()> {
        for prompt in prompts {
            self.add_prompt(prompt)?;
        }
        Ok(())
    }

    /// Get a prompt by name
    pub fn get_prompt(&self, name: &str) -> Option<&Prompt> {
        self.prompts.get(name)
    }

    /// List all prompts
    pub fn list_prompts(&self) -> Vec<Prompt> {
        self.prompts.values().cloned().collect()
    }

    /// Clear all registered capabilities
    pub fn clear(&mut self) {
        self.tools.clear();
        self.resources.clear();
        self.prompts.clear();
    }

    /// Get number of registered tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Get number of registered resources
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    /// Get number of registered prompts
    pub fn prompt_count(&self) -> usize {
        self.prompts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_registry_creation() {
        let registry = Registry::new();
        assert_eq!(registry.tool_count(), 0);
        assert_eq!(registry.resource_count(), 0);
        assert_eq!(registry.prompt_count(), 0);
    }

    #[test]
    fn test_add_tool() {
        let mut registry = Registry::new();

        let tool = Tool {
            name: "test-tool".to_string(),
            title: Some("Test Tool".to_string()),
            description: "A test tool".to_string(),
            input_schema: json!({"type": "object"}),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        assert!(registry.add_tool(tool).is_ok());
        assert_eq!(registry.tool_count(), 1);
        assert!(registry.get_tool("test-tool").is_some());
    }

    #[test]
    fn test_duplicate_tool_error() {
        let mut registry = Registry::new();

        let tool1 = Tool {
            name: "duplicate".to_string(),
            title: None,
            description: "Tool 1".to_string(),
            input_schema: json!({}),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        let tool2 = Tool {
            name: "duplicate".to_string(),
            title: None,
            description: "Tool 2".to_string(),
            input_schema: json!({}),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        assert!(registry.add_tool(tool1).is_ok());
        assert!(registry.add_tool(tool2).is_err());
    }

    #[test]
    fn test_add_resource() {
        let mut registry = Registry::new();

        let resource = Resource {
            uri: "file:///test.txt".to_string(),
            name: "test".to_string(),
            title: None,
            description: None,
            mime_type: Some("text/plain".to_string()),
            annotations: None,
            icons: None,
            raw: None,
        };

        assert!(registry.add_resource(resource).is_ok());
        assert_eq!(registry.resource_count(), 1);
        assert!(registry.get_resource("file:///test.txt").is_some());
    }

    #[test]
    fn test_add_prompt() {
        let mut registry = Registry::new();

        let prompt = Prompt {
            name: "test-prompt".to_string(),
            title: None,
            description: None,
            arguments: None,
            icons: None,
        };

        assert!(registry.add_prompt(prompt).is_ok());
        assert_eq!(registry.prompt_count(), 1);
        assert!(registry.get_prompt("test-prompt").is_some());
    }

    #[test]
    fn test_list_tools() {
        let mut registry = Registry::new();

        for i in 0..3 {
            let tool = Tool {
                name: format!("tool-{}", i),
                title: None,
                description: format!("Tool {}", i),
                input_schema: json!({}),
                output_schema: None,
                annotations: None,
                icons: None,
            };
            registry.add_tool(tool).unwrap();
        }

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_clear_registry() {
        let mut registry = Registry::new();

        let tool = Tool {
            name: "tool".to_string(),
            title: None,
            description: "Tool".to_string(),
            input_schema: json!({}),
            output_schema: None,
            annotations: None,
            icons: None,
        };
        registry.add_tool(tool).unwrap();

        assert_eq!(registry.tool_count(), 1);

        registry.clear();

        assert_eq!(registry.tool_count(), 0);
        assert_eq!(registry.resource_count(), 0);
        assert_eq!(registry.prompt_count(), 0);
    }
}
