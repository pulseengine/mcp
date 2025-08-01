//! Implementation of the #[mcp_tool] macro

use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
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
    let struct_name = &impl_block.self_ty;
    let (impl_generics, ty_generics, where_clause) = impl_block.generics.split_for_impl();

    // Extract tool functions from the impl block
    let mut tool_functions = Vec::new();
    let mut tool_definitions = Vec::new();
    let mut tool_dispatch_cases = Vec::new();

    for item in &impl_block.items {
        if let syn::ImplItem::Fn(method) = item {
            // Skip private methods and methods starting with underscore
            if matches!(method.vis, syn::Visibility::Public(_))
                && !method.sig.ident.to_string().starts_with('_')
            {
                let fn_name = &method.sig.ident;
                let tool_name = function_name_to_tool_name(fn_name);
                let description = extract_doc_comment(&method.attrs);

                // Extract parameter information
                let (param_struct, param_fields) = extract_parameters(&method.sig)?;

                // Generate input schema
                let input_schema = if param_fields.is_empty() {
                    quote! { serde_json::json!({ "type": "object", "properties": {} }) }
                } else {
                    generate_schema_for_type(&param_struct)
                };

                // Handle async functions
                let (call_expr, _is_async) = if method.sig.asyncness.is_some() {
                    (quote! { self.#fn_name(#(#param_fields),*).await }, true)
                } else {
                    (quote! { self.#fn_name(#(#param_fields),*) }, false)
                };

                let description_expr = match description.as_deref() {
                    Some(desc) => quote! { Some(#desc.to_string()) },
                    None => quote! { None },
                };

                // Generate tool definition
                tool_definitions.push(quote! {
                    pulseengine_mcp_protocol::Tool {
                        name: #tool_name.to_string(),
                        description: #description_expr,
                        input_schema: #input_schema,
                        output_schema: None,
                    }
                });

                // Generate error handling
                let error_handling = generate_error_handling(&method.sig.output);

                // Generate parameter extraction
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

                // Generate dispatch case
                tool_dispatch_cases.push(quote! {
                    #tool_name => {
                        #param_extraction
                        let result = #call_expr;
                        #error_handling
                    }
                });

                tool_functions.push(fn_name.clone());
            }
        }
    }

    // Generate the impl block and override the helper methods
    Ok(quote! {
        #impl_block

        // Marker trait to indicate this type has tools
        impl #impl_generics McpToolsProvider for #struct_name #ty_generics #where_clause {
            fn get_tools(&self) -> Vec<pulseengine_mcp_protocol::Tool> {
                vec![#(#tool_definitions),*]
            }

            fn dispatch_tool(
                &self,
                request: pulseengine_mcp_protocol::CallToolRequestParam,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error>> + Send + '_>> {
                Box::pin(async move {
                    match request.name.as_str() {
                        #(#tool_dispatch_cases)*
                        _ => Err(pulseengine_mcp_protocol::Error::invalid_params(
                            format!("Unknown tool: {}", request.name)
                        ))
                    }
                })
            }
        }

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

                    // Generate parameter extraction code with consistent error handling
                    if is_option_type(param_type) {
                        param_fields.push(quote! {
                            args.get(stringify!(#param_name))
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                        });
                    } else {
                        param_fields.push(quote! {
                            match args.get(stringify!(#param_name))
                                .and_then(|v| serde_json::from_value(v.clone()).ok()) {
                                Some(value) => value,
                                None => return Some(Err(pulseengine_mcp_protocol::Error::invalid_params(
                                    format!("Missing required parameter: {}", stringify!(#param_name))
                                ))),
                            }
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
