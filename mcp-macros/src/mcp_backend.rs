//! Implementation of the #[mcp_backend] macro

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemEnum, ItemStruct};

use crate::utils::*;

/// Attribute parameters for #[mcp_backend]
#[derive(FromMeta, Default, Debug)]
#[darling(default)]
pub struct McpBackendAttribute {
    /// Server name (required)
    pub name: String,
    /// Server version (defaults to Cargo package version)
    pub version: Option<String>,
    /// Server description (defaults to doc comments)
    pub description: Option<String>,
    /// Custom capabilities
    pub capabilities: Option<syn::Expr>,
}

/// Implementation of #[mcp_backend] macro
pub fn mcp_backend_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let attr_args = darling::ast::NestedMeta::parse_meta_list(attr)?;
    let attribute = McpBackendAttribute::from_list(&attr_args)
        .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e.to_string()))?;

    // Try parsing as struct first, then enum
    let (struct_name, generics, fields, doc_comment) =
        if let Ok(item_struct) = syn::parse2::<ItemStruct>(item.clone()) {
            let doc = extract_doc_comment(&item_struct.attrs);
            (
                item_struct.ident,
                item_struct.generics,
                Some(item_struct.fields),
                doc,
            )
        } else if let Ok(item_enum) = syn::parse2::<ItemEnum>(item.clone()) {
            let doc = extract_doc_comment(&item_enum.attrs);
            (item_enum.ident, item_enum.generics, None, doc)
        } else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[mcp_backend] can only be applied to structs or enums",
            ));
        };

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

    // Generate capabilities based on available features
    let capabilities = attribute.capabilities.unwrap_or_else(|| {
        // TODO: Auto-detect resources and prompts from impl blocks
        // This will be enhanced to scan for #[mcp_resource] and #[mcp_prompt] attributes
        syn::parse2(quote! {
            pulseengine_mcp_protocol::ServerCapabilities {
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
                logging: Some(pulseengine_mcp_protocol::LoggingCapability {}),
                sampling: None,
                ..Default::default()
            }
        })
        .unwrap()
    });

    // Generate error type if not already defined
    let error_type_name = quote::format_ident!("{}Error", struct_name);

    let backend_impl = generate_backend_implementation(
        &struct_name,
        &generics,
        server_name,
        &server_version,
        &server_description,
        &capabilities,
        &error_type_name,
        fields.as_ref(),
    )?;

    let original_item = item;

    Ok(quote! {
        #original_item
        #backend_impl
    })
}

/// Generate the complete McpBackend implementation
#[allow(clippy::too_many_arguments)]
fn generate_backend_implementation(
    struct_name: &syn::Ident,
    generics: &syn::Generics,
    server_name: &str,
    server_version: &TokenStream,
    server_description: &TokenStream,
    capabilities: &syn::Expr,
    error_type_name: &syn::Ident,
    _fields: Option<&syn::Fields>,
) -> syn::Result<TokenStream> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        // Generate error type if not exists
        #[derive(Debug, thiserror::Error)]
        pub enum #error_type_name {
            #[error("Invalid parameter: {0}")]
            InvalidParameter(String),

            #[error("Internal error: {0}")]
            Internal(String),

            #[error("Backend error: {0}")]
            Backend(#[from] pulseengine_mcp_server::BackendError),
        }

        impl From<#error_type_name> for pulseengine_mcp_protocol::Error {
            fn from(err: #error_type_name) -> Self {
                match err {
                    #error_type_name::InvalidParameter(msg) =>
                        pulseengine_mcp_protocol::Error::invalid_params(msg),
                    #error_type_name::Internal(msg) =>
                        pulseengine_mcp_protocol::Error::internal_error(msg),
                    #error_type_name::Backend(backend_err) => backend_err.into(),
                }
            }
        }

        #[async_trait::async_trait]
        impl #impl_generics pulseengine_mcp_server::McpBackend for #struct_name #ty_generics #where_clause {
            type Error = #error_type_name;
            type Config = ();

            async fn initialize(_config: Self::Config) -> Result<Self, Self::Error> {
                // User must provide their own initialization logic
                Err(#error_type_name::Internal(
                    "initialize method must be implemented manually".to_string()
                ))
            }

            fn get_server_info(&self) -> pulseengine_mcp_protocol::ServerInfo {
                pulseengine_mcp_protocol::ServerInfo {
                    protocol_version: pulseengine_mcp_protocol::ProtocolVersion::default(),
                    capabilities: #capabilities,
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
                // Auto-discover tools from impl blocks with #[mcp_tool]
                let mut tools = Vec::new();

                // This will be enhanced to automatically collect tools
                // from methods marked with #[mcp_tool]

                Ok(pulseengine_mcp_protocol::ListToolsResult {
                    tools,
                    next_cursor: None,
                })
            }

            async fn call_tool(
                &self,
                request: pulseengine_mcp_protocol::CallToolRequestParam,
            ) -> Result<pulseengine_mcp_protocol::CallToolResult, Self::Error> {
                // Auto-dispatch to tool implementations
                Err(#error_type_name::InvalidParameter(
                    format!("Unknown tool: {}", request.name)
                ))
            }

            async fn list_resources(
                &self,
                _request: pulseengine_mcp_protocol::PaginatedRequestParam,
            ) -> Result<pulseengine_mcp_protocol::ListResourcesResult, Self::Error> {
                // Auto-discover resources from methods marked with #[mcp_resource]
                let mut resources = Vec::new();

                // This will be enhanced to automatically collect resources
                // from methods with #[mcp_resource] attribute

                Ok(pulseengine_mcp_protocol::ListResourcesResult {
                    resources,
                    next_cursor: None,
                })
            }

            async fn read_resource(
                &self,
                request: pulseengine_mcp_protocol::ReadResourceRequestParam,
            ) -> Result<pulseengine_mcp_protocol::ReadResourceResult, Self::Error> {
                // Auto-dispatch to resource implementations
                // This will be enhanced to automatically route to methods with #[mcp_resource]
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

                // This will be enhanced to automatically collect prompts
                // from methods with #[mcp_prompt] attribute

                Ok(pulseengine_mcp_protocol::ListPromptsResult {
                    prompts,
                    next_cursor: None,
                })
            }

            async fn get_prompt(
                &self,
                request: pulseengine_mcp_protocol::GetPromptRequestParam,
            ) -> Result<pulseengine_mcp_protocol::GetPromptResult, Self::Error> {
                // Auto-dispatch to prompt implementations
                // This will be enhanced to automatically route to methods with #[mcp_prompt]
                Err(#error_type_name::InvalidParameter(
                    format!("Prompt not found: {}", request.name)
                ))
            }
        }

        // Note: Default implementation should be manually provided
        // or derived on the struct if needed
    })
}
