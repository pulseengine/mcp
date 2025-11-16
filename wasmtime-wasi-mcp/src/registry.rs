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
}
