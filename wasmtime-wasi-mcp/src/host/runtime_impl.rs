//! Runtime Host trait implementation
//!
//! Implements the `wasi::mcp::runtime::Host` trait for `WasiMcpCtx`,
//! providing the MCP runtime interface to WASM components.

use crate::conversions;
use crate::ctx::WasiMcpCtx;
use crate::host::wasi::mcp::runtime;
use std::pin::Pin;
use std::future::Future;

// Re-export generated types for convenience
pub use runtime::{
    LogLevel, Notification, ProgressToken,
    PromptDefinition, ResourceDefinition, ResourceTemplate, ServerInfo,
    ToolDefinition,
};

/// Implement the runtime Host trait for our context
impl runtime::Host for WasiMcpCtx {
    fn register_server<'life0, 'async_trait>(
        &'life0 mut self,
        info: ServerInfo,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] register_server: {} v{}", info.name, info.version);

            // Store server implementation info
            self.server_info = Some(conversions::server_info_to_implementation(info.clone()));

            // Store server capabilities
            self.capabilities = Some(conversions::server_info_to_capabilities(&info));

            // Store instructions if provided
            self.instructions = info.instructions;

            Ok(())
        })
    }

    fn register_tools<'life0, 'async_trait>(
        &'life0 mut self,
        tools: Vec<ToolDefinition>,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] register_tools: {} tools", tools.len());

            // Convert and add each tool to our registry
            for tool_def in tools {
                eprintln!("[HOST]   - {}: {}", tool_def.name, tool_def.description);

                match conversions::tool_definition_to_tool(tool_def) {
                    Ok(tool) => {
                        if let Err(e) = self.registry.add_tool(tool) {
                            eprintln!("[HOST]   ERROR: Failed to register tool: {}", e);
                            // TODO: Return error resource instead of Ok
                        }
                    }
                    Err(e) => {
                        eprintln!("[HOST]   ERROR: Failed to convert tool: {}", e);
                        // TODO: Return error resource instead of Ok
                    }
                }
            }

            Ok(())
        })
    }

    fn register_resources<'life0, 'async_trait>(
        &'life0 mut self,
        resources: Vec<ResourceDefinition>,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] register_resources: {} resources", resources.len());

            // Convert and add each resource to our registry
            for resource_def in resources {
                eprintln!("[HOST]   - {}", resource_def.uri);

                let resource = conversions::resource_definition_to_resource(resource_def);
                if let Err(e) = self.registry.add_resource(resource) {
                    eprintln!("[HOST]   ERROR: Failed to register resource: {}", e);
                    // TODO: Return error resource instead of Ok
                }
            }

            Ok(())
        })
    }

    fn register_resource_templates<'life0, 'async_trait>(
        &'life0 mut self,
        templates: Vec<ResourceTemplate>,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] register_resource_templates: {} templates", templates.len());

            // Note: Registry doesn't currently support templates
            // TODO: Add template support to registry
            for template in &templates {
                eprintln!("[HOST]   - {} (not stored)", template.uri_template);
            }

            Ok(())
        })
    }

    fn register_prompts<'life0, 'async_trait>(
        &'life0 mut self,
        prompts: Vec<PromptDefinition>,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] register_prompts: {} prompts", prompts.len());

            // Convert and add each prompt to our registry
            for prompt_def in prompts {
                let desc = prompt_def.description.as_deref().unwrap_or("");
                eprintln!("[HOST]   - {}: {}", prompt_def.name, desc);

                let prompt = conversions::prompt_definition_to_prompt(prompt_def);
                if let Err(e) = self.registry.add_prompt(prompt) {
                    eprintln!("[HOST]   ERROR: Failed to register prompt: {}", e);
                    // TODO: Return error resource instead of Ok
                }
            }

            Ok(())
        })
    }

    fn serve<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] serve: Starting MCP event loop");

            // This is the main event loop where the host takes control
            // For now, just return to avoid blocking
            // TODO: Implement full event loop with JSON-RPC handling

            eprintln!("[HOST] serve: Event loop complete");
            Ok(())
        })
    }

    fn send_notification<'life0, 'async_trait>(
        &'life0 mut self,
        _notification: Notification,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] send_notification");

            // Send notification via transport
            // TODO: Convert to JSON-RPC and send via backend

            Ok(())
        })
    }

    fn log<'life0, 'async_trait>(
        &'life0 mut self,
        level: LogLevel,
        message: String,
        data: Option<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            eprintln!("[HOST] log [{:?}] {}", level, message);
            if let Some(bytes) = data {
                if let Ok(s) = String::from_utf8(bytes) {
                    eprintln!("[HOST]   data: {}", s);
                }
            }

            // TODO: Send as MCP logging notification

            Ok(())
        })
    }

    fn report_progress<'life0, 'async_trait>(
        &'life0 mut self,
        _progress_token: ProgressToken,
        progress: u64,
        total: Option<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<(), wasmtime::component::Resource<runtime::Error>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            if let Some(total_val) = total {
                eprintln!("[HOST] report_progress: {}/{}", progress, total_val);
            } else {
                eprintln!("[HOST] report_progress: {}", progress);
            }

            // TODO: Send as MCP progress notification

            Ok(())
        })
    }
}
