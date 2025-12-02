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
    /// Server version (defaults to "1.0.0")
    pub version: Option<String>,
    /// Server description (defaults to doc comments)
    pub description: Option<String>,
    /// Authentication mode: "memory", "file", "disabled", or omit for no auth
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
        .as_ref()
        .cloned()
        .map(|v| quote! { #v.to_string() })
        .unwrap_or_else(get_package_version);

    let server_description = attribute
        .description
        .as_ref()
        .cloned()
        .or(doc_comment)
        .map(|desc| quote! { Some(#desc.to_string()) })
        .unwrap_or_else(|| quote! { None });

    let server_impl = generate_server_implementation(
        struct_name,
        generics,
        server_name,
        &server_version,
        &server_description,
        &attribute,
    )?;

    Ok(quote! {
        #item

        // Import necessary traits for macro-generated code
        use pulseengine_mcp_server::McpBackend as _;

        #server_impl
    })
}

/// Generate the simplified server implementation
fn generate_server_implementation(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    server_name: &str,
    server_version: &TokenStream,
    server_description: &TokenStream,
    attribute: &McpServerAttribute,
) -> syn::Result<TokenStream> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let config_type_name = quote::format_ident!("{}Config", struct_name);
    let error_type_name = quote::format_ident!("{}Error", struct_name);
    let service_type_name = quote::format_ident!("{}Service", struct_name);

    // Generate auth-related code based on auth parameter
    let auth_config = match attribute.auth.as_deref() {
        None => {
            // Default: no authentication (memory storage, disabled)
            quote! {
                // Default behavior: disable authentication with memory storage (no filesystem access)
                let mut auth_config = pulseengine_mcp_server::auth::AuthConfig::memory();
                auth_config.enabled = false;
                config.auth_config = auth_config;
            }
        }
        Some("disabled") => {
            // Explicitly disabled authentication (memory storage, disabled)
            quote! {
                let mut auth_config = pulseengine_mcp_server::auth::AuthConfig::memory();
                auth_config.enabled = false;
                config.auth_config = auth_config;
            }
        }
        Some("memory") => {
            // Memory-only authentication (for development/testing)
            quote! {
                config.auth_config = pulseengine_mcp_server::auth::AuthConfig::memory();
            }
        }
        Some("file") => {
            // File-based authentication (for production)
            quote! {
                config.auth_config = pulseengine_mcp_server::auth::AuthConfig::default();
            }
        }
        Some(other) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Invalid auth parameter: '{other}'. Valid options are: 'memory', 'file', 'disabled', or omit for no auth"
                ),
            ));
        }
    };

    Ok(quote! {
        // Simplified config type alias
        type #config_type_name = ();

        // Use common error type to reduce generated code
        type #error_type_name = pulseengine_mcp_server::CommonMcpError;

        // Custom server info implementation for this server
        impl #impl_generics pulseengine_mcp_server::HasServerInfo for #struct_name #ty_generics #where_clause {
            fn server_info() -> pulseengine_mcp_protocol::ServerInfo {
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
        }

        // Delegate to common backend implementation to drastically reduce generated code
        #[async_trait::async_trait]
        impl #impl_generics pulseengine_mcp_server::McpBackend for #struct_name #ty_generics #where_clause {
            type Error = #error_type_name;
            type Config = #config_type_name;

            async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::Error> {
                Ok(Self::default())
            }

            fn get_server_info(&self) -> pulseengine_mcp_protocol::ServerInfo {
                <Self as pulseengine_mcp_server::HasServerInfo>::server_info()
            }

            // Delegate all other methods to the common backend
            async fn health_check(&self) -> std::result::Result<(), Self::Error> {
                Ok(())
            }

            async fn list_tools(&self, request: pulseengine_mcp_protocol::PaginatedRequestParam) -> std::result::Result<pulseengine_mcp_protocol::ListToolsResult, Self::Error> {
                // Try to use McpToolsProvider trait if implemented
                let tools = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    <Self as pulseengine_mcp_server::McpToolsProvider>::get_available_tools(self)
                })) {
                    Ok(tools) => tools,
                    Err(_) => vec![], // Trait not implemented, no tools
                };
                Ok(pulseengine_mcp_protocol::ListToolsResult { tools, next_cursor: None })
            }

            async fn call_tool(&self, request: pulseengine_mcp_protocol::CallToolRequestParam) -> std::result::Result<pulseengine_mcp_protocol::CallToolResult, Self::Error> {
                // Try to use McpToolsProvider trait if implemented
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // This is a sync check, we'll handle async in the actual call
                    <Self as pulseengine_mcp_server::McpToolsProvider>::get_available_tools(self)
                })) {
                    Ok(_) => {
                        // Trait is implemented, call the tool
                        match <Self as pulseengine_mcp_server::McpToolsProvider>::call_tool_impl(self, request.clone()).await {
                            Ok(result) => Ok(result),
                            Err(e) => Err(#error_type_name::InvalidParams(e.to_string())),
                        }
                    }
                    Err(_) => {
                        // Trait not implemented
                        Err(#error_type_name::InvalidParams(format!("Unknown tool: {}", request.name)))
                    }
                }
            }

            async fn list_resources(&self, request: pulseengine_mcp_protocol::PaginatedRequestParam) -> std::result::Result<pulseengine_mcp_protocol::ListResourcesResult, Self::Error> {
                // Use helper method that safely checks for trait implementation
                let resources = self.try_get_resources_default();
                Ok(pulseengine_mcp_protocol::ListResourcesResult { resources, next_cursor: request.cursor })
            }

            async fn read_resource(&self, request: pulseengine_mcp_protocol::ReadResourceRequestParam) -> std::result::Result<pulseengine_mcp_protocol::ReadResourceResult, Self::Error> {
                // Use helper method that calls resource implementation
                match self.try_read_resource_default(request.clone()).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(#error_type_name::InvalidParams(e.to_string()))
                }
            }

            async fn list_prompts(&self, _request: pulseengine_mcp_protocol::PaginatedRequestParam) -> std::result::Result<pulseengine_mcp_protocol::ListPromptsResult, Self::Error> {
                Ok(pulseengine_mcp_protocol::ListPromptsResult { prompts: vec![], next_cursor: None })
            }

            async fn get_prompt(&self, request: pulseengine_mcp_protocol::GetPromptRequestParam) -> std::result::Result<pulseengine_mcp_protocol::GetPromptResult, Self::Error> {
                Err(#error_type_name::InvalidParams(format!("Unknown prompt: {}", request.name)))
            }
        }

        // The McpToolsProvider trait is defined by the mcp_tools macro when needed
        // We don't define it here to avoid conflicts when multiple servers exist

        // No longer need helper methods - using direct trait delegation pattern consistently

        // Implement the builder trait to provide common functionality
        impl #impl_generics pulseengine_mcp_server::McpServerBuilder for #struct_name #ty_generics #where_clause {}

        // Auth configuration is handled in serve_stdio method

        // Server transport methods
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Serve using STDIO transport
            pub async fn serve_stdio(self) -> std::result::Result<pulseengine_mcp_server::McpServer<Self>, #error_type_name> {
                use pulseengine_mcp_server::{McpServer, ServerConfig, TransportConfig};

                let mut config = ServerConfig::default();
                config.transport_config = TransportConfig::Stdio;

                // Set the server info to use the macro-generated values
                config.server_info = <Self as pulseengine_mcp_server::HasServerInfo>::server_info();

                // Set auth configuration based on macro parameter
                #auth_config

                let server = McpServer::new(self, config).await.map_err(|e| {
                    #error_type_name::Internal(format!("Failed to create server: {}", e))
                })?;

                Ok(server)
            }

            /// Serve using HTTP (Streamable HTTP) transport
            pub async fn serve_http(self, port: u16) -> std::result::Result<pulseengine_mcp_server::McpServer<Self>, #error_type_name> {
                use pulseengine_mcp_server::{McpServer, ServerConfig, TransportConfig};

                let mut config = ServerConfig::default();
                config.transport_config = TransportConfig::StreamableHttp {
                    port,
                    host: None,
                };

                // Set the server info to use the macro-generated values
                config.server_info = <Self as pulseengine_mcp_server::HasServerInfo>::server_info();

                // Set auth configuration based on macro parameter
                #auth_config

                let server = McpServer::new(self, config).await.map_err(|e| {
                    #error_type_name::Internal(format!("Failed to create server: {}", e))
                })?;

                Ok(server)
            }


            /// Serve using WebSocket transport
            pub async fn serve_websocket(self, port: u16) -> std::result::Result<pulseengine_mcp_server::McpServer<Self>, #error_type_name> {
                use pulseengine_mcp_server::{McpServer, ServerConfig, TransportConfig};

                let mut config = ServerConfig::default();
                config.transport_config = TransportConfig::WebSocket {
                    port,
                    host: None,
                };

                // Set the server info to use the macro-generated values
                config.server_info = <Self as pulseengine_mcp_server::HasServerInfo>::server_info();

                // Set auth configuration based on macro parameter
                #auth_config

                let server = McpServer::new(self, config).await.map_err(|e| {
                    #error_type_name::Internal(format!("Failed to create server: {}", e))
                })?;

                Ok(server)
            }

            /// Build a server with custom configuration
            pub async fn build_server(self, config: pulseengine_mcp_server::ServerConfig) -> std::result::Result<pulseengine_mcp_server::McpServer<Self>, #error_type_name> {
                use pulseengine_mcp_server::McpServer;

                let server = McpServer::new(self, config).await.map_err(|e| {
                    #error_type_name::Internal(format!("Failed to create server: {}", e))
                })?;

                Ok(server)
            }
        }

        // Service type alias for convenience
        type #service_type_name #ty_generics = pulseengine_mcp_server::McpService<#struct_name #ty_generics>;
    })
}

/// Attribute parameters for #[mcp_app]
#[derive(FromMeta, Debug, Default)]
#[darling(default)]
pub struct McpAppAttribute {
    /// Server name (required)
    pub name: String,
    /// Server version (defaults to Cargo package version)
    pub version: Option<String>,
    /// Server description (defaults to doc comments)
    pub description: Option<String>,
    /// Transport type: "stdio" (default), "websocket", "http"
    pub transport: Option<String>,
}

/// Implementation of #[mcp_app] macro - creates complete app with main function
pub fn mcp_app_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let attr_args = darling::ast::NestedMeta::parse_meta_list(attr)?;
    let attribute = McpAppAttribute::from_list(&attr_args)
        .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e.to_string()))?;

    // Parse the impl block
    let impl_block = syn::parse2::<syn::ItemImpl>(item.clone())?;

    // Extract struct name from impl block
    let struct_name = if let syn::Type::Path(type_path) = impl_block.self_ty.as_ref() {
        type_path.path.segments.last().unwrap().ident.clone()
    } else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "mcp_app can only be used on impl blocks for named structs",
        ));
    };

    // Validate that name is not empty
    if attribute.name.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Server name is required. Use #[mcp_app(name = \"Your App Name\")]",
        ));
    }

    let server_name = &attribute.name;
    let server_version = attribute
        .version
        .as_ref()
        .cloned()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    let server_description = attribute
        .description
        .as_ref()
        .cloned()
        .unwrap_or_else(|| server_name.clone());

    let transport = attribute.transport.as_deref().unwrap_or("stdio");

    // Generate transport-specific main function
    let main_function = match transport {
        "stdio" => quote! {
            #[tokio::main]
            async fn main() -> Result<(), Box<dyn std::error::Error>> {
                // Configure logging for STDIO transport
                #struct_name::configure_stdio_logging();

                // Start the server
                let mut server = #struct_name::with_defaults().serve_stdio().await?;
                server.run().await?;

                Ok(())
            }
        },
        "websocket" => quote! {
            #[tokio::main]
            async fn main() -> Result<(), Box<dyn std::error::Error>> {
                // Default WebSocket port
                let port = std::env::var("MCP_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse::<u16>()
                    .unwrap_or(3000);

                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
                println!("🚀 Starting {} server on ws://{}", #server_name, addr);

                let mut server = #struct_name::with_defaults().serve_websocket(addr).await?;
                server.run().await?;

                Ok(())
            }
        },
        "http" => quote! {
            #[tokio::main]
            async fn main() -> Result<(), Box<dyn std::error::Error>> {
                // Default HTTP port
                let port = std::env::var("MCP_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse::<u16>()
                    .unwrap_or(3000);

                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
                println!("🚀 Starting {} server on http://{}", #server_name, addr);

                let mut server = #struct_name::with_defaults().serve_http(addr).await?;
                server.run().await?;

                Ok(())
            }
        },
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "transport must be 'stdio', 'websocket', or 'http'",
            ));
        }
    };

    // Combine everything - struct definition, server implementation, tools, and main
    Ok(quote! {
        // Required imports for the generated code
        use pulseengine_mcp_macros::{mcp_server, mcp_tools};
        use pulseengine_mcp_server::{McpServerBuilder, McpBackend, McpToolsProvider};
        use pulseengine_mcp_protocol;
        use serde_json;

        // Generate the struct
        #[mcp_server(
            name = #server_name,
            version = #server_version,
            description = #server_description
        )]
        #[derive(Default, Clone)]
        pub struct #struct_name;

        // Apply mcp_tools to the impl block
        #[mcp_tools]
        #impl_block

        // Generate the main function
        #main_function
    })
}
