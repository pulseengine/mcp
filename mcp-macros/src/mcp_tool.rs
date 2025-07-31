//! Implementation of the #[mcp_tool] macro

use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{ImplItemFn, ItemFn, ItemImpl, ReturnType};

use crate::utils::*;

/// Attribute parameters for #[mcp_tool]
#[derive(FromMeta, Default, Debug)]
#[darling(default)]
pub struct McpToolAttribute {
    /// Custom tool name (defaults to function name)
    pub name: Option<String>,
    /// Tool description (defaults to doc comments)
    pub description: Option<String>,
    /// Whether this tool is read-only
    pub read_only: Option<bool>,
    /// Whether this tool is idempotent
    pub idempotent: Option<bool>,
    /// Custom input schema
    pub input_schema: Option<syn::Expr>,
}

/// Implementation of #[mcp_tool] macro
pub fn mcp_tool_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let attribute = if attr.is_empty() {
        Default::default()
    } else {
        let attr_args = NestedMeta::parse_meta_list(attr)?;
        McpToolAttribute::from_list(&attr_args)
            .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e.to_string()))?
    };

    let mut function =
        syn::parse2::<ImplItemFn>(item.clone()).or_else(|_| -> syn::Result<ImplItemFn> {
            // Try parsing as a standalone function
            let standalone_fn = syn::parse2::<ItemFn>(item)?;
            Ok(ImplItemFn {
                attrs: standalone_fn.attrs,
                vis: standalone_fn.vis,
                defaultness: None,
                sig: standalone_fn.sig,
                block: *standalone_fn.block,
            })
        })?;

    let fn_name = &function.sig.ident;
    let tool_name = attribute
        .name
        .unwrap_or_else(|| function_name_to_tool_name(fn_name));
    let description = attribute
        .description
        .or_else(|| extract_doc_comment(&function.attrs));

    // Generate tool definition function
    let tool_def_fn_name = format_ident!("{}_tool_definition", fn_name);

    // Extract parameter information
    let (param_struct, param_fields) = extract_parameters(&function.sig)?;

    // Generate input schema
    let input_schema = if let Some(schema_expr) = attribute.input_schema {
        quote! { #schema_expr }
    } else if param_fields.is_empty() {
        quote! { serde_json::json!({ "type": "object", "properties": {} }) }
    } else {
        generate_schema_for_type(&param_struct)
    };

    // Handle async functions
    let (call_expr, is_async) = if function.sig.asyncness.is_some() {
        (quote! { self.#fn_name(#(#param_fields),*).await }, true)
    } else {
        (quote! { self.#fn_name(#(#param_fields),*) }, false)
    };

    // Generate the tool implementation
    let tool_impl = generate_tool_implementation(
        fn_name,
        &tool_def_fn_name,
        &tool_name,
        description.as_deref(),
        &input_schema,
        &call_expr,
        &function.sig.output,
        is_async,
        &param_fields,
    )?;

    // Generate the enhanced function with tool metadata
    let enhanced_function = enhance_function_with_metadata(&mut function, &tool_name)?;

    Ok(quote! {
        #enhanced_function
        #tool_impl
    })
}

/// Implementation of #[mcp_tools] macro for impl blocks
pub fn mcp_tools_impl(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let impl_block = syn::parse2::<ItemImpl>(item)?;

    // Validate that this is being applied to a proper impl block
    if impl_block
        .self_ty
        .as_ref()
        .to_token_stream()
        .to_string()
        .is_empty()
    {
        return Err(syn::Error::new_spanned(
            &impl_block.self_ty,
            "#[mcp_tools] can only be applied to impl blocks with a valid type",
        ));
    }

    let struct_name = &impl_block.self_ty;
    let (impl_generics, _, where_clause) = impl_block.generics.split_for_impl();

    // Find all public methods that should become tools
    let mut tool_methods = Vec::new();
    let mut tool_definitions = Vec::new();
    let mut tool_dispatch_cases = Vec::new();
    let mut tool_definition_functions = Vec::new();

    for item in &impl_block.items {
        if let syn::ImplItem::Fn(method) = item {
            // Skip private methods
            if !matches!(method.vis, syn::Visibility::Public(_)) {
                continue;
            }

            // Skip methods starting with underscore (internal methods)
            let method_name = &method.sig.ident;
            if method_name.to_string().starts_with('_') {
                continue;
            }

            // Skip methods with self as first parameter that aren't &self
            let has_self_ref = method.sig.inputs.first().is_some_and(
                |arg| matches!(arg, syn::FnArg::Receiver(receiver) if receiver.reference.is_some()),
            );

            if !has_self_ref {
                continue;
            }

            let tool_name = function_name_to_tool_name(method_name);
            let tool_def_fn_name = format_ident!("{}_tool_definition", method_name);

            // Extract parameter information
            let (param_struct, param_fields) = extract_parameters(&method.sig)?;

            // Extract description from doc comments
            let description = extract_doc_comment(&method.attrs);
            let description_expr = match description.as_deref() {
                Some(desc) => quote! { Some(#desc.to_string()) },
                None => quote! { None },
            };

            // Generate input schema
            let input_schema = if param_fields.is_empty() {
                quote! { serde_json::json!({ "type": "object", "properties": {} }) }
            } else {
                generate_schema_for_type(&param_struct)
            };

            // Generate tool definition function
            tool_definition_functions.push(quote! {
                impl #impl_generics #struct_name #where_clause {
                    pub fn #tool_def_fn_name() -> pulseengine_mcp_protocol::Tool {
                        pulseengine_mcp_protocol::Tool {
                            name: #tool_name.to_string(),
                            description: #description_expr,
                            input_schema: #input_schema,
                            output_schema: None,
                        }
                    }
                }
            });

            // Generate tool definition call
            tool_definitions.push(quote! {
                tools.push(Self::#tool_def_fn_name());
            });

            // Generate dispatch case
            let param_extraction = if param_fields.is_empty() {
                quote! {}
            } else {
                quote! {
                    let args = request.arguments.unwrap_or(serde_json::Value::Object(Default::default()));
                    let args = args.as_object().ok_or_else(||
                        pulseengine_mcp_protocol::Error::invalid_params("Arguments must be an object")
                    )?;
                }
            };

            // Handle async functions
            let call_expr = if method.sig.asyncness.is_some() {
                quote! { self.#method_name(#(#param_fields),*).await }
            } else {
                quote! { self.#method_name(#(#param_fields),*) }
            };

            let error_handling = generate_error_handling(&method.sig.output);

            tool_dispatch_cases.push(quote! {
                #tool_name => {
                    #param_extraction

                    let result = #call_expr;
                    #error_handling
                }
            });

            tool_methods.push(method_name.clone());
        }
    }

    // Generate trait implementation to override the default behavior
    let tool_discovery_impl = if !tool_methods.is_empty() {
        quote! {
            // Override the default trait implementation with actual tool discovery
            impl #impl_generics McpToolsDefault for #struct_name #where_clause {
                fn __mcp_get_discovered_tools(&self) -> Vec<pulseengine_mcp_protocol::Tool> {
                    let mut tools = Vec::new();
                    #(#tool_definitions)*
                    tools
                }

                fn __mcp_dispatch_discovered_tool(
                    &self,
                    request: pulseengine_mcp_protocol::CallToolRequestParam,
                ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error>>> + Send + '_>> {
                    Box::pin(async move {
                        match request.name.as_str() {
                            #(#tool_dispatch_cases)*
                            _ => None, // Not an automatically discovered tool
                        }
                    })
                }
            }
        }
    } else {
        quote! {
            // No tools found in impl block - use default trait implementation
        }
    };

    Ok(quote! {
        #impl_block

        // Generate individual tool definition functions
        #(#tool_definition_functions)*

        #tool_discovery_impl
    })
}

/// Extract parameter information from function signature
fn extract_parameters(sig: &syn::Signature) -> syn::Result<(syn::Type, Vec<TokenStream>)> {
    let mut param_fields = Vec::new();
    let mut param_types = Vec::new();
    let mut param_names = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => {
                // Skip self parameter
                continue;
            }
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = &pat_ident.ident;
                    let param_type = &*pat_type.ty;

                    param_names.push(param_name.clone());
                    param_types.push(param_type.clone());

                    // Generate parameter extraction code
                    if is_option_type(param_type) {
                        param_fields.push(quote! {
                            args.get(stringify!(#param_name))
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                        });
                    } else {
                        param_fields.push(quote! {
                            args.get(stringify!(#param_name))
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                                .ok_or_else(|| pulseengine_mcp_protocol::Error::invalid_params(
                                    format!("Missing required parameter: {}", stringify!(#param_name))
                                ))?
                        });
                    }
                }
            }
        }
    }

    // Create a struct type for the parameters
    let param_struct_name = format_ident!("ToolParams");
    let param_struct = if param_types.is_empty() {
        syn::parse2::<syn::Type>(quote! { () })?
    } else {
        syn::parse2::<syn::Type>(quote! {
            struct #param_struct_name {
                #(#param_names: #param_types),*
            }
        })?
    };

    Ok((param_struct, param_fields))
}

/// Parameters for tool implementation generation
#[allow(dead_code)]
struct ToolImplementationParams<'a> {
    fn_name: &'a syn::Ident,
    tool_def_fn_name: &'a syn::Ident,
    tool_name: &'a str,
    description: Option<&'a str>,
    input_schema: &'a TokenStream,
    call_expr: &'a TokenStream,
    return_type: &'a ReturnType,
    is_async: bool,
    param_fields: &'a [TokenStream],
}

/// Generate the tool implementation function
#[allow(clippy::too_many_arguments)]
fn generate_tool_implementation(
    fn_name: &syn::Ident,
    tool_def_fn_name: &syn::Ident,
    tool_name: &str,
    description: Option<&str>,
    input_schema: &TokenStream,
    call_expr: &TokenStream,
    return_type: &ReturnType,
    _is_async: bool,
    param_fields: &[TokenStream],
) -> syn::Result<TokenStream> {
    let description_expr = match description {
        Some(desc) => quote! { Some(#desc.to_string()) },
        None => quote! { None },
    };

    let error_handling = generate_error_handling(return_type);
    let tool_call = quote! {
        let result = #call_expr;
        #error_handling
    };

    let param_extraction = if param_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            let args = request.arguments.unwrap_or(serde_json::Value::Object(Default::default()));
            let args = args.as_object().ok_or_else(||
                pulseengine_mcp_protocol::Error::invalid_params("Arguments must be an object")
            )?;
        }
    };

    let call_tool_fn_name = format_ident!("call_tool_impl_{}", fn_name);

    Ok(quote! {
        pub fn #tool_def_fn_name() -> pulseengine_mcp_protocol::Tool {
            pulseengine_mcp_protocol::Tool {
                name: #tool_name.to_string(),
                description: #description_expr,
                input_schema: #input_schema,
                output_schema: None,
            }
        }

        pub async fn #call_tool_fn_name(
            &self,
            request: pulseengine_mcp_protocol::CallToolRequestParam,
        ) -> Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error> {
            match request.name.as_str() {
                #tool_name => {
                    #param_extraction

                    #tool_call
                }
                _ => Err(pulseengine_mcp_protocol::Error::invalid_params(
                    format!("Unknown tool: {}", request.name)
                ))
            }
        }
    })
}

/// Enhance function with tool metadata
fn enhance_function_with_metadata(
    function: &mut ImplItemFn,
    tool_name: &str,
) -> syn::Result<TokenStream> {
    // Add metadata attributes to the function
    let tool_attr = quote! {
        #[doc = concat!("MCP Tool: ", #tool_name)]
    };

    Ok(quote! {
        #tool_attr
        #function
    })
}
