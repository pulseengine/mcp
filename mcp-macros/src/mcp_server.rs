//! Implementation of the #[mcp_server] macro

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

use crate::utils::*;

/// Attribute parameters for #[mcp_server]
#[derive(FromMeta, Debug, Default)]
#[darling(default)]
pub struct McpServerAttribute {
    /// Server name (required)
    pub name: String,
    /// Application name for storage isolation (optional)
    pub app_name: Option<String>,
    /// Server version (defaults to Cargo package version)
    pub version: Option<String>,
    /// Server description (defaults to doc comments)
    pub description: Option<String>,
    /// Default transport type
    pub transport: Option<String>,
    /// Authentication configuration: "memory", "file", "disabled", or custom
    pub auth: Option<String>,
}

/// Implementation of #[mcp_server] macro
pub fn mcp_server_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let attr_args = darling::ast::NestedMeta::parse_meta_list(attr)?;
    let attribute = McpServerAttribute::from_list(&attr_args)
        .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e.to_string()))?;

    let item_struct = syn::parse2::<ItemStruct>(item.clone())?;
    let struct_name = &item_struct.ident;
    let generics = &item_struct.generics;
    let doc_comment = extract_doc_comment(&item_struct.attrs);

    // Validate that name is not empty
    if attribute.name.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Server name is required. Use #[mcp_server(name = \"Your Server Name\")]",
        ));
    }

    let server_name = &attribute.name;
    let server_version = attribute
        .version
        .map(|v| quote! { #v.to_string() })
        .unwrap_or_else(get_package_version);

    let server_description = attribute
        .description
        .or(doc_comment)
        .map(|desc| quote! { Some(#desc.to_string()) })
        .unwrap_or_else(|| quote! { None });

    let transport_default = match attribute.transport.as_deref() {
        Some("stdio") => quote! { pulseengine_mcp_transport::TransportConfig::Stdio },
        Some("http") => {
            quote! { pulseengine_mcp_transport::TransportConfig::Http { port: 8080, host: None } }
        }
        Some("websocket") => {
            quote! { pulseengine_mcp_transport::TransportConfig::WebSocket { port: 8080, host: None } }
        }
        _ => quote! { pulseengine_mcp_transport::TransportConfig::Stdio }, // Default to stdio
    };

    let server_impl = generate_server_implementation(
        struct_name,
        generics,
        server_name,
        &server_version,
        &server_description,
        &transport_default,
        &attribute.app_name,
        &attribute.auth,
    )?;

    Ok(quote! {
        #item

        // Import necessary traits for macro-generated code
        use pulseengine_mcp_server::McpBackend as _;

        #server_impl
    })
}

/// Generate the complete server implementation with fluent builder API
#[allow(clippy::too_many_arguments)]
fn generate_server_implementation(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    server_name: &str,
    server_version: &TokenStream,
    server_description: &TokenStream,
    transport_default: &TokenStream,
    app_name: &Option<String>,
    auth: &Option<String>,
) -> syn::Result<TokenStream> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let config_type_name = quote::format_ident!("{}Config", struct_name);
    let error_type_name = quote::format_ident!("{}Error", struct_name);
    let service_type_name = quote::format_ident!("{}Service", struct_name);

    // Generate auth configuration call based on auth parameter
    let auth_config_call = match auth.as_deref() {
        Some("memory") => {
            quote! { pulseengine_mcp_auth::AuthConfig::memory() }
        }
        Some("disabled") => {
            quote! { pulseengine_mcp_auth::AuthConfig::disabled() }
        }
        Some("file") => {
            // Use app_name if provided, otherwise default
            if let Some(app_name) = app_name {
                quote! { pulseengine_mcp_auth::for_application(#app_name) }
            } else {
                quote! { pulseengine_mcp_auth::default_config() }
            }
        }
        Some(custom) => {
            // Allow custom auth configuration expressions
            let custom_expr = syn::parse_str::<syn::Expr>(custom)
                .map_err(|_| syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!("Invalid auth configuration: {custom}. Use 'memory', 'file', 'disabled', or a valid Rust expression")
                ))?;
            quote! { #custom_expr }
        }
        None => {
            // Default behavior: use app_name if provided, otherwise default
            if let Some(app_name) = app_name {
                quote! { pulseengine_mcp_auth::for_application(#app_name) }
            } else {
                quote! { pulseengine_mcp_auth::default_config() }
            }
        }
    };

    Ok(quote! {
        // Configuration type
        #[derive(Debug, Clone)]
        pub struct #config_type_name {
            pub server_name: String,
            pub server_version: String,
            pub server_description: Option<String>,
            pub transport: pulseengine_mcp_transport::TransportConfig,
        }

        impl Default for #config_type_name {
            fn default() -> Self {
                Self {
                    server_name: #server_name.to_string(),
                    server_version: #server_version,
                    server_description: #server_description,
                    transport: #transport_default,
                }
            }
        }

        #[cfg(feature = "auth")]
        impl #config_type_name {
            /// Get the appropriate auth configuration for this server
            pub fn get_auth_config() -> pulseengine_mcp_auth::AuthConfig {
                #auth_config_call
            }
        }

        // Error type
        #[derive(Debug, thiserror::Error)]
        pub enum #error_type_name {
            #[error("Invalid parameter: {0}")]
            InvalidParameter(String),

            #[error("Internal error: {0}")]
            Internal(String),

            #[error("Server error: {0}")]
            Server(#[from] pulseengine_mcp_server::BackendError),

            #[error("Server error: {0}")]
            ServerSetup(#[from] pulseengine_mcp_server::ServerError),

            #[error("Transport error: {0}")]
            Transport(String),
        }

        impl From<#error_type_name> for pulseengine_mcp_protocol::Error {
            fn from(err: #error_type_name) -> Self {
                match err {
                    #error_type_name::InvalidParameter(msg) =>
                        pulseengine_mcp_protocol::Error::invalid_params(msg),
                    #error_type_name::Internal(msg) =>
                        pulseengine_mcp_protocol::Error::internal_error(msg),
                    #error_type_name::Server(server_err) => server_err.into(),
                    #error_type_name::ServerSetup(server_err) =>
                        pulseengine_mcp_protocol::Error::internal_error(server_err.to_string()),
                    #error_type_name::Transport(msg) =>
                        pulseengine_mcp_protocol::Error::internal_error(msg),
                }
            }
        }

        // Service wrapper type
        pub struct #service_type_name #ty_generics #where_clause {
            backend: #struct_name #ty_generics,
            server: pulseengine_mcp_server::McpServer<#struct_name #ty_generics>,
        }

        // Backend implementation
        #[async_trait::async_trait]
        impl #impl_generics pulseengine_mcp_server::McpBackend for #struct_name #ty_generics #where_clause {
            type Error = #error_type_name;
            type Config = #config_type_name;

            async fn initialize(_config: Self::Config) -> Result<Self, Self::Error> {
                // Use Default trait if available, or user must provide their own implementation
                Ok(Self::default())
            }

            fn get_server_info(&self) -> pulseengine_mcp_protocol::ServerInfo {
                pulseengine_mcp_protocol::ServerInfo {
                    protocol_version: pulseengine_mcp_protocol::ProtocolVersion::default(),
                    capabilities: pulseengine_mcp_protocol::ServerCapabilities {
                        tools: Some(pulseengine_mcp_protocol::ToolsCapability {
                            list_changed: Some(false),
                        }),
                        resources: Some(pulseengine_mcp_protocol::ResourcesCapability {
                            subscribe: Some(false),
                            list_changed: Some(false),
                        }),
                        prompts: Some(pulseengine_mcp_protocol::PromptsCapability {
                            list_changed: Some(false),
                        }),
                        logging: Some(pulseengine_mcp_protocol::LoggingCapability {
                            level: Some("info".to_string()),
                        }),
                        sampling: None,
                        ..Default::default()
                    },
                    server_info: pulseengine_mcp_protocol::Implementation {
                        name: #server_name.to_string(),
                        version: #server_version,
                    },
                    instructions: #server_description,
                }
            }

            async fn health_check(&self) -> Result<(), Self::Error> {
                Ok(())
            }

            async fn list_tools(
                &self,
                _request: pulseengine_mcp_protocol::PaginatedRequestParam,
            ) -> Result<pulseengine_mcp_protocol::ListToolsResult, Self::Error> {
                // Check if this type implements McpToolsProvider and get tools if available
                let tools = self.get_available_tools();
                Ok(pulseengine_mcp_protocol::ListToolsResult {
                    tools,
                    next_cursor: None,
                })
            }

            async fn call_tool(
                &self,
                request: pulseengine_mcp_protocol::CallToolRequestParam,
            ) -> Result<pulseengine_mcp_protocol::CallToolResult, Self::Error> {
                // Check if this type implements McpToolsProvider and dispatch tools if available
                match self.dispatch_available_tool(request.clone()).await {
                    Some(result) => result.map_err(|e| #error_type_name::InvalidParameter(format!("Tool error: {}", e))),
                    None => Err(#error_type_name::InvalidParameter(
                        format!("Unknown tool: {}", request.name)
                    ))
                }
            }

            async fn list_resources(
                &self,
                _request: pulseengine_mcp_protocol::PaginatedRequestParam,
            ) -> Result<pulseengine_mcp_protocol::ListResourcesResult, Self::Error> {
                // Auto-discover resources from methods marked with #[mcp_resource]
                let mut resources = Vec::new();

                // Get resources from automatic resource discovery (if #[mcp_resource] methods exist)
                let automatic_resources = self.get_automatic_resources();
                resources.extend(automatic_resources);

                Ok(pulseengine_mcp_protocol::ListResourcesResult {
                    resources,
                    next_cursor: None,
                })
            }

            async fn read_resource(
                &self,
                request: pulseengine_mcp_protocol::ReadResourceRequestParam,
            ) -> Result<pulseengine_mcp_protocol::ReadResourceResult, Self::Error> {
                // Try automatic resource dispatch (if #[mcp_resource] methods exist)
                if let Some(result) = self.dispatch_automatic_resource(request.clone()).await {
                    return result.map_err(|e| #error_type_name::InvalidParameter(format!("Resource error: {}", e)));
                }

                Err(#error_type_name::InvalidParameter(
                    format!("Resource not found: {}", request.uri)
                ))
            }

            async fn list_prompts(
                &self,
                _request: pulseengine_mcp_protocol::PaginatedRequestParam,
            ) -> Result<pulseengine_mcp_protocol::ListPromptsResult, Self::Error> {
                // Auto-discover prompts from methods marked with #[mcp_prompt]
                let mut prompts = Vec::new();

                // Get prompts from automatic prompt discovery (if #[mcp_prompt] methods exist)
                let automatic_prompts = self.get_automatic_prompts();
                prompts.extend(automatic_prompts);

                Ok(pulseengine_mcp_protocol::ListPromptsResult {
                    prompts,
                    next_cursor: None,
                })
            }

            async fn get_prompt(
                &self,
                request: pulseengine_mcp_protocol::GetPromptRequestParam,
            ) -> Result<pulseengine_mcp_protocol::GetPromptResult, Self::Error> {
                // Try automatic prompt dispatch (if #[mcp_prompt] methods exist)
                if let Some(result) = self.dispatch_automatic_prompt(request.clone()).await {
                    return result.map_err(|e| #error_type_name::InvalidParameter(format!("Prompt error: {}", e)));
                }

                Err(#error_type_name::InvalidParameter(
                    format!("Prompt not found: {}", request.name)
                ))
            }
        }

        // Define the trait that mcp_tools will implement
        pub trait McpToolsProvider {
            /// Get all available tools
            fn get_tools(&self) -> Vec<pulseengine_mcp_protocol::Tool>;

            /// Dispatch tool calls to appropriate handlers  
            fn dispatch_tool(
                &self,
                request: pulseengine_mcp_protocol::CallToolRequestParam,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error>> + Send + '_>>;
        }

        // Helper methods for tool integration
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Get available tools (default implementation returns empty)
            fn get_available_tools(&self) -> Vec<pulseengine_mcp_protocol::Tool> {
                Vec::new()
            }

            /// Dispatch tool calls (default implementation returns None)
            async fn dispatch_available_tool(
                &self,
                _request: pulseengine_mcp_protocol::CallToolRequestParam,
            ) -> Option<Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error>> {
                None
            }
        }


        // Integration hooks for other MCP features
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Integration hook for automatic resource discovery
            /// This method is designed to be compatible with resources generated by #[mcp_resource]
            #[allow(unused_variables)]
            fn get_automatic_resources(&self) -> Vec<pulseengine_mcp_protocol::Resource> {
                // Default implementation returns empty vec
                // This will be enhanced to collect resources from methods with #[mcp_resource]
                Vec::new()
            }

            /// Integration hook for automatic resource dispatch
            /// This method is designed to be compatible with dispatch generated by #[mcp_resource]
            #[allow(unused_variables)]
            async fn dispatch_automatic_resource(
                &self,
                request: pulseengine_mcp_protocol::ReadResourceRequestParam,
            ) -> Option<Result<pulseengine_mcp_protocol::ReadResourceResult, pulseengine_mcp_protocol::Error>> {
                // Default implementation returns None (no automatic resources available)
                // This will be enhanced to route to methods with #[mcp_resource]
                None
            }

            /// Integration hook for automatic prompt discovery
            /// This method is designed to be compatible with prompts generated by #[mcp_prompt]
            #[allow(unused_variables)]
            fn get_automatic_prompts(&self) -> Vec<pulseengine_mcp_protocol::Prompt> {
                // Default implementation returns empty vec
                // This will be enhanced to collect prompts from methods with #[mcp_prompt]
                Vec::new()
            }

            /// Integration hook for automatic prompt dispatch
            /// This method is designed to be compatible with dispatch generated by #[mcp_prompt]
            #[allow(unused_variables)]
            async fn dispatch_automatic_prompt(
                &self,
                request: pulseengine_mcp_protocol::GetPromptRequestParam,
            ) -> Option<Result<pulseengine_mcp_protocol::GetPromptResult, pulseengine_mcp_protocol::Error>> {
                // Default implementation returns None (no automatic prompts available)
                // This will be enhanced to route to methods with #[mcp_prompt]
                None
            }
        }

        // Fluent builder API - this is where the magic happens!
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Create a new instance with default configuration (requires Default to be derived)
            pub fn with_defaults() -> Self
            where
                Self: Default
            {
                Self::default()
            }

            /// Create an authentication manager with appropriate configuration for this server
            #[cfg(feature = "auth")]
            pub async fn create_auth_manager() -> Result<pulseengine_mcp_auth::AuthenticationManager, pulseengine_mcp_auth::manager::AuthError> {
                let auth_config = #config_type_name::get_auth_config();
                pulseengine_mcp_auth::AuthenticationManager::new(auth_config).await
            }

            /// Serve using stdio transport (default for MCP clients like Claude Desktop)
            pub async fn serve_stdio(self) -> Result<#service_type_name #ty_generics, #error_type_name> {
                let config = #config_type_name {
                    transport: pulseengine_mcp_transport::TransportConfig::Stdio,
                    ..Default::default()
                };
                self.serve_with_config(config).await
            }

            /// Serve using HTTP transport on specified port
            pub async fn serve_http(self, port: u16) -> Result<#service_type_name #ty_generics, #error_type_name> {
                let config = #config_type_name {
                    transport: pulseengine_mcp_transport::TransportConfig::Http { port, host: None },
                    ..Default::default()
                };
                self.serve_with_config(config).await
            }

            /// Serve using WebSocket transport on specified port
            pub async fn serve_websocket(self, port: u16) -> Result<#service_type_name #ty_generics, #error_type_name> {
                let config = #config_type_name {
                    transport: pulseengine_mcp_transport::TransportConfig::WebSocket { port, host: None },
                    ..Default::default()
                };
                self.serve_with_config(config).await
            }

            /// Serve with custom configuration
            pub async fn serve_with_config(self, config: #config_type_name) -> Result<#service_type_name #ty_generics, #error_type_name> {
                let backend = #struct_name::initialize(config.clone()).await?;

                let server_config = pulseengine_mcp_server::ServerConfig {
                    server_info: backend.get_server_info(),
                    transport_config: config.transport,
                    ..Default::default()
                };

                let server = pulseengine_mcp_server::McpServer::new(backend.clone(), server_config)
                    .await
                    .map_err(|e| #error_type_name::ServerSetup(e))?;

                Ok(#service_type_name {
                    backend,
                    server,
                })
            }
        }

        // Service implementation with lifecycle management
        impl #impl_generics #service_type_name #ty_generics #where_clause {
            /// Run the server until shutdown
            pub async fn run(mut self) -> Result<(), #error_type_name> {
                self.server.run().await
                    .map_err(|e| #error_type_name::ServerSetup(e))
            }

            /// Run the server with graceful shutdown handling
            pub async fn run_with_shutdown<F>(mut self, shutdown_signal: F) -> Result<(), #error_type_name>
            where
                F: std::future::Future<Output = ()> + Send + 'static,
            {
                tokio::select! {
                    result = self.server.run() => {
                        result.map_err(|e| #error_type_name::ServerSetup(e))
                    }
                    _ = shutdown_signal => {
                        tracing::info!("Shutdown signal received, stopping server");
                        Ok(())
                    }
                }
            }

            /// Get a reference to the backend
            pub fn backend(&self) -> &#struct_name #ty_generics {
                &self.backend
            }

            /// Get server information
            pub fn server_info(&self) -> pulseengine_mcp_protocol::ServerInfo {
                self.backend.get_server_info()
            }
        }
    })
}
