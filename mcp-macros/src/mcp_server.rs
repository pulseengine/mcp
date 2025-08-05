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

    let server_impl = generate_server_implementation(
        struct_name,
        generics,
        server_name,
        &server_version,
        &server_description,
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
) -> syn::Result<TokenStream> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let config_type_name = quote::format_ident!("{}Config", struct_name);
    let error_type_name = quote::format_ident!("{}Error", struct_name);
    let service_type_name = quote::format_ident!("{}Service", struct_name);

    // Simplified - no complex auth logic
    let auth_manager_method = quote! {};

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
                // Check if this implements McpToolsProvider
                if let Some(tools) = self.try_get_tools() {
                    Ok(pulseengine_mcp_protocol::ListToolsResult { tools, next_cursor: None })
                } else {
                    Ok(pulseengine_mcp_protocol::ListToolsResult { tools: vec![], next_cursor: None })
                }
            }

            async fn call_tool(&self, request: pulseengine_mcp_protocol::CallToolRequestParam) -> std::result::Result<pulseengine_mcp_protocol::CallToolResult, Self::Error> {
                if let Some(result) = self.try_call_tool(request.clone()).await {
                    result.map_err(|e| #error_type_name::InvalidParams(e.to_string()))
                } else {
                    Err(#error_type_name::InvalidParams(format!("Unknown tool: {}", request.name)))
                }
            }

            async fn list_resources(&self, _request: pulseengine_mcp_protocol::PaginatedRequestParam) -> std::result::Result<pulseengine_mcp_protocol::ListResourcesResult, Self::Error> {
                Ok(pulseengine_mcp_protocol::ListResourcesResult { resources: vec![], next_cursor: None })
            }

            async fn read_resource(&self, request: pulseengine_mcp_protocol::ReadResourceRequestParam) -> std::result::Result<pulseengine_mcp_protocol::ReadResourceResult, Self::Error> {
                Err(#error_type_name::InvalidParams(format!("Unknown resource: {}", request.uri)))
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

        // Helper methods for tool integration
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Try to get tools if this type implements McpToolsProvider
            #[allow(unused_variables)]
            fn try_get_tools(&self) -> Option<Vec<pulseengine_mcp_protocol::Tool>> {
                None  // Default: no tools unless mcp_tools macro is used
            }

            /// Try to call a tool if this type implements McpToolsProvider
            #[allow(unused_variables)]
            async fn try_call_tool(
                &self,
                request: pulseengine_mcp_protocol::CallToolRequestParam,
            ) -> Option<std::result::Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error>> {
                None  // Default: no tools unless mcp_tools macro is used
            }
        }

        // Implement the builder trait to provide common functionality
        impl #impl_generics pulseengine_mcp_server::McpServerBuilder for #struct_name #ty_generics #where_clause {}

        // Add the auth manager method if auth is configured
        #auth_manager_method

        // Server transport methods
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Serve using STDIO transport
            pub async fn serve_stdio(self) -> std::result::Result<pulseengine_mcp_server::McpServer<Self>, #error_type_name> {
                use pulseengine_mcp_server::{McpServer, ServerConfig};

                let config = ServerConfig::default();
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
