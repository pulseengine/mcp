//! Implementation of the #[mcp_tool] macro

use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItemFn, ItemFn, ReturnType};

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
    let (param_struct, param_fields) = extract_parameters(&function.sig, &tool_name)?;

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

/// Extract URI template from mcp_resource attribute
fn extract_uri_template_from_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("mcp_resource") {
            if let Ok(meta_list) = attr.meta.require_list() {
                // Parse the meta list tokens properly
                if let Ok(nested_meta) =
                    darling::ast::NestedMeta::parse_meta_list(meta_list.tokens.clone())
                {
                    for nested in nested_meta {
                        if let darling::ast::NestedMeta::Meta(syn::Meta::NameValue(name_value)) =
                            nested
                        {
                            if name_value.path.is_ident("uri_template") {
                                if let syn::Expr::Lit(syn::ExprLit {
                                    lit: syn::Lit::Str(lit_str),
                                    ..
                                }) = name_value.value
                                {
                                    return Some(lit_str.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Implementation of #[mcp_tools] macro for impl blocks
pub fn mcp_tools_impl(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let impl_block = syn::parse2::<syn::ItemImpl>(item)?;

    // Extract struct name from impl block
    let struct_name = match &*impl_block.self_ty {
        syn::Type::Path(type_path) => type_path.path.segments.last().unwrap().ident.clone(),
        _ => {
            return Err(syn::Error::new_spanned(
                &impl_block.self_ty,
                "#[mcp_tools] can only be applied to impl blocks for named structs",
            ));
        }
    };

    // Extract generics if any
    let (impl_generics, ty_generics, where_clause) = impl_block.generics.split_for_impl();

    // Collect public methods that should become tools or resources
    let mut tool_definitions = Vec::new();
    let mut tool_dispatch_cases = Vec::new();
    let mut resource_definitions = Vec::new();
    let mut resource_dispatch_cases = Vec::new();

    for item in &impl_block.items {
        if let syn::ImplItem::Fn(method) = item {
            // Only process public methods
            if matches!(method.vis, syn::Visibility::Public(_)) {
                let method_name = &method.sig.ident;

                // Check if this method has #[mcp_resource] attribute
                let has_resource_attr = method
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("mcp_resource"));

                if has_resource_attr {
                    // Handle as resource
                    let resource_name = method.sig.ident.to_string();

                    // Extract documentation from method
                    let doc_comment = extract_doc_comment(&method.attrs);
                    let description = doc_comment
                        .unwrap_or_else(|| format!("Generated resource for {resource_name}"));

                    // Extract URI template from mcp_resource attribute
                    let uri_template = extract_uri_template_from_attr(&method.attrs)
                        .unwrap_or_else(|| format!("resource://{resource_name}"));

                    // Create resource definition
                    resource_definitions.push(quote! {
                        pulseengine_mcp_protocol::Resource {
                            uri: #uri_template.to_string(),
                            name: #resource_name.to_string(),
                            description: Some(#description.to_string()),
                            mime_type: Some("application/json".to_string()),
                            annotations: None,
                            raw: None,
                        }
                    });

                    // Generate resource dispatch case for read_resource_impl
                    let is_async = method.sig.asyncness.is_some();
                    let await_token = if is_async { quote!(.await) } else { quote!() };

                    // Check if method has parameters (beyond &self)
                    let has_params = method.sig.inputs.len() > 1;

                    if has_params {
                        // Extract parameter names for URI template parameter extraction
                        let mut param_names = Vec::new();
                        for input in &method.sig.inputs {
                            match input {
                                syn::FnArg::Receiver(_) => continue,
                                syn::FnArg::Typed(pat_type) => {
                                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                        param_names.push(&pat_ident.ident);
                                    }
                                }
                            }
                        }

                        // For now, use hardcoded parameter values - this is a limitation
                        // TODO: Implement proper URI template parameter extraction
                        let param_values = match param_names.len() {
                            1 => quote! { "default".to_string() },
                            2 => quote! { "default".to_string(), "value".to_string() },
                            _ => quote! { "default".to_string() },
                        };

                        resource_dispatch_cases.push(quote! {
                            #uri_template => {
                                let result = self.#method_name(#param_values)#await_token;
                                match result {
                                    Ok(content) => {
                                        let content_str = serde_json::to_string(&content)
                                            .map_err(|e| pulseengine_mcp_protocol::Error::internal_error(
                                                format!("Failed to serialize resource content: {}", e)
                                            ))?;
                                        Ok(pulseengine_mcp_protocol::ReadResourceResult {
                                            contents: vec![pulseengine_mcp_protocol::ResourceContents {
                                                uri: uri.to_string(),
                                                mime_type: Some("application/json".to_string()),
                                                text: Some(content_str),
                                                blob: None,
                                            }]
                                        })
                                    }
                                    Err(e) => Err(pulseengine_mcp_protocol::Error::internal_error(
                                        format!("Resource error: {}", e)
                                    ))
                                }
                            }
                        });
                    } else {
                        // No parameters - simple resource call
                        resource_dispatch_cases.push(quote! {
                            #uri_template => {
                                let result = self.#method_name()#await_token;
                                match result {
                                    Ok(content) => {
                                        let content_str = serde_json::to_string(&content)
                                            .map_err(|e| pulseengine_mcp_protocol::Error::internal_error(
                                                format!("Failed to serialize resource content: {}", e)
                                            ))?;
                                        Ok(pulseengine_mcp_protocol::ReadResourceResult {
                                            contents: vec![pulseengine_mcp_protocol::ResourceContents {
                                                uri: uri.to_string(),
                                                mime_type: Some("application/json".to_string()),
                                                text: Some(content_str),
                                                blob: None,
                                            }]
                                        })
                                    }
                                    Err(e) => Err(pulseengine_mcp_protocol::Error::internal_error(
                                        format!("Resource error: {}", e)
                                    ))
                                }
                            }
                        });
                    }
                } else {
                    // Handle as tool (existing logic)
                    let tool_name = method.sig.ident.to_string();

                    // Extract documentation from method
                    let doc_comment = extract_doc_comment(&method.attrs);
                    let description =
                        doc_comment.unwrap_or_else(|| format!("Generated tool for {tool_name}"));

                    // Generate JSON schema for parameters
                    let schema =
                        quote! { serde_json::json!({ "type": "object", "properties": {} }) };

                    // Create tool definition
                    tool_definitions.push(quote! {
                        pulseengine_mcp_protocol::Tool {
                            name: #tool_name.to_string(),
                            description: #description.to_string(),
                            input_schema: #schema,
                            output_schema: None,
                        }
                    });

                    // Generate dispatch case
                    let is_async = method.sig.asyncness.is_some();
                    let method_call =
                        generate_method_call_with_params(&method.sig, method_name, is_async)?;
                    let error_handling = generate_error_handling(&method.sig.output);

                    tool_dispatch_cases.push(quote! {
                        #tool_name => {
                            let empty_map = serde_json::Map::new();
                            let empty_value = serde_json::Value::Object(empty_map);
                            let args = request.arguments.as_ref().unwrap_or(&empty_value);
                            let args = args.as_object().ok_or_else(|| {
                                pulseengine_mcp_protocol::Error::invalid_params("Arguments must be an object".to_string())
                            })?;

                            // Call method and handle result based on return type
                            let result = #method_call;
                            #error_handling
                        }
                    });
                }
            }
        }
    }

    // Generate resource provider implementation if there are resources
    let resource_provider_impl = if !resource_definitions.is_empty() {
        quote! {
            impl #impl_generics pulseengine_mcp_server::McpResourcesProvider for #struct_name #ty_generics #where_clause {
                fn get_available_resources(&self) -> Vec<pulseengine_mcp_protocol::Resource> {
                    vec![
                        #(#resource_definitions),*
                    ]
                }

                fn read_resource_impl(
                    &self,
                    request: pulseengine_mcp_protocol::ReadResourceRequestParam,
                ) -> impl std::future::Future<Output = std::result::Result<pulseengine_mcp_protocol::ReadResourceResult, pulseengine_mcp_protocol::Error>> + Send {
                    async move {
                        let uri = &request.uri;
                        match uri.as_str() {
                            #(#resource_dispatch_cases)*
                            _ => Err(pulseengine_mcp_protocol::Error::invalid_params(
                                format!("Unknown resource: {}", uri)
                            ))
                        }
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    // Resource backend override temporarily disabled to avoid trait conflicts

    // Generate the enhanced impl block that uses a trait-based approach to avoid method conflicts
    let enhanced_impl = quote! {
        #impl_block

        // Implement a tools provider trait
        impl #impl_generics pulseengine_mcp_server::McpToolsProvider for #struct_name #ty_generics #where_clause {
            fn get_available_tools(&self) -> Vec<pulseengine_mcp_protocol::Tool> {
                vec![
                    #(#tool_definitions),*
                ]
            }

            fn call_tool_impl(
                &self,
                request: pulseengine_mcp_protocol::CallToolRequestParam,
            ) -> impl std::future::Future<Output = std::result::Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error>> + Send {
                async move {
                    match request.name.as_str() {
                        #(#tool_dispatch_cases)*
                        _ => Err(pulseengine_mcp_protocol::Error::invalid_params(
                            format!("Unknown tool: {}", request.name)
                        ))
                    }
                }
            }
        }

        // Resource provider implementation (if resources exist)
        #resource_provider_impl

    };

    let helper_methods = generate_helper_methods(&struct_name);

    let final_impl = quote! {
        #enhanced_impl
        #helper_methods
    };

    Ok(final_impl)
}

/// Extract parameter information from function signature
fn extract_parameters(
    sig: &syn::Signature,
    tool_name: &str,
) -> syn::Result<(syn::Type, Vec<TokenStream>)> {
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
                                None => return Err(pulseengine_mcp_protocol::Error::invalid_params(
                                    format!("Missing required parameter '{}' for tool '{}'. Expected type: {}",
                                        stringify!(#param_name), #tool_name, stringify!(#param_type))
                                )),
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
                pulseengine_mcp_protocol::Error::invalid_params(
                    format!("Tool '{}' requires arguments as JSON object, got: {}",
                        #tool_name, match &args {
                            serde_json::Value::Array(_) => "array",
                            serde_json::Value::String(_) => "string",
                            serde_json::Value::Number(_) => "number",
                            serde_json::Value::Bool(_) => "boolean",
                            serde_json::Value::Null => "null",
                            _ => "unknown"
                        })
                )
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
        ) -> std::result::Result<pulseengine_mcp_protocol::CallToolResult, pulseengine_mcp_protocol::Error> {
            match request.name.as_str() {
                #tool_name => {
                    #param_extraction

                    #tool_call
                }
                _ => Err(pulseengine_mcp_protocol::Error::invalid_params(
                    format!("Unknown tool '{}'. Available tools: [{}]",
                        request.name, #tool_name)
                ))
            }
        }
    })
}

/// Generate JSON schema for method parameters
#[allow(dead_code)]
fn generate_parameter_schema(sig: &syn::Signature) -> syn::Result<TokenStream> {
    let mut properties = Vec::new();
    let mut required = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => continue, // Skip self
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = &pat_ident.ident;
                    let param_type = &*pat_type.ty;
                    let param_name_str = param_name.to_string();

                    // Generate schema based on type
                    let schema = generate_type_schema(param_type)?;

                    properties.push(quote! {
                        (#param_name_str, #schema)
                    });

                    // Check if parameter is required (not Option<T>)
                    if !is_option_type(param_type) {
                        required.push(param_name_str);
                    }
                }
            }
        }
    }

    Ok(quote! {
        {
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));

            let mut properties = serde_json::Map::new();
            #(
                let (name, prop_schema) = #properties;
                properties.insert(name.to_string(), prop_schema);
            )*
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));

            if !vec![#(#required),*].is_empty() {
                schema.insert(
                    "required".to_string(),
                    serde_json::Value::Array(vec![#(serde_json::Value::String(#required.to_string())),*])
                );
            }

            serde_json::Value::Object(schema)
        }
    })
}

/// Generate type-specific JSON schema
#[allow(dead_code)]
fn generate_type_schema(ty: &syn::Type) -> syn::Result<TokenStream> {
    match ty {
        syn::Type::Path(type_path) => {
            let path = &type_path.path;

            // Handle common types
            if let Some(segment) = path.segments.last() {
                let type_name = segment.ident.to_string();

                match type_name.as_str() {
                    "String" | "str" => Ok(quote! {
                        serde_json::json!({ "type": "string" })
                    }),
                    "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize"
                    | "usize" => Ok(quote! {
                        serde_json::json!({ "type": "integer" })
                    }),
                    "f32" | "f64" => Ok(quote! {
                        serde_json::json!({ "type": "number" })
                    }),
                    "bool" => Ok(quote! {
                        serde_json::json!({ "type": "boolean" })
                    }),
                    "Vec" => {
                        // Handle Vec<T>
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
                            {
                                let inner_schema = generate_type_schema(inner_type)?;
                                return Ok(quote! {
                                    serde_json::json!({
                                        "type": "array",
                                        "items": #inner_schema
                                    })
                                });
                            }
                        }
                        Ok(quote! {
                            serde_json::json!({ "type": "array" })
                        })
                    }
                    "Option" => {
                        // Handle Option<T>
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first()
                            {
                                return generate_type_schema(inner_type);
                            }
                        }
                        Ok(quote! {
                            serde_json::json!({ "type": "string" })
                        })
                    }
                    _ => {
                        // Default to object for custom types
                        Ok(quote! {
                            serde_json::json!({ "type": "object" })
                        })
                    }
                }
            } else {
                Ok(quote! {
                    serde_json::json!({ "type": "object" })
                })
            }
        }
        _ => {
            // Default for complex types
            Ok(quote! {
                serde_json::json!({ "type": "object" })
            })
        }
    }
}

/// Generate parameter extraction and method call for tools
fn generate_method_call_with_params(
    sig: &syn::Signature,
    method_name: &syn::Ident,
    is_async: bool,
) -> syn::Result<TokenStream> {
    let mut param_declarations = Vec::new();
    let mut param_names = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => continue, // Skip self
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = &pat_ident.ident;
                    let param_type = &*pat_type.ty;

                    param_names.push(param_name);

                    // Generate parameter extraction based on whether it's optional
                    if is_option_type(param_type) {
                        param_declarations.push(quote! {
                            let #param_name = args.get(stringify!(#param_name))
                                .and_then(|v| serde_json::from_value(v.clone()).ok());
                        });
                    } else {
                        param_declarations.push(quote! {
                            let #param_name = args.get(stringify!(#param_name))
                                .and_then(|v| serde_json::from_value(v.clone()).ok())
                                .ok_or_else(|| pulseengine_mcp_protocol::Error::invalid_params(
                                    format!("Missing required parameter '{}'", stringify!(#param_name))
                                ))?;
                        });
                    }
                }
            }
        }
    }

    if param_declarations.is_empty() {
        // No parameters - call method directly
        if is_async {
            Ok(quote! {
                self.#method_name().await
            })
        } else {
            Ok(quote! {
                self.#method_name()
            })
        }
    } else {
        // Has parameters - extract them and call method
        if is_async {
            Ok(quote! {
                {
                    #(#param_declarations)*
                    self.#method_name(#(#param_names),*).await
                }
            })
        } else {
            Ok(quote! {
                {
                    #(#param_declarations)*
                    self.#method_name(#(#param_names),*)
                }
            })
        }
    }
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

/// Generate helper methods for development and testing
fn generate_helper_methods(struct_name: &syn::Ident) -> TokenStream {
    quote! {
        impl #struct_name {
            /// Helper method to get available tools (used in tests)
            #[allow(dead_code)]
            pub fn try_get_tools_default(&self) -> Option<Vec<pulseengine_mcp_protocol::Tool>> {
                Some(<Self as pulseengine_mcp_server::McpToolsProvider>::get_available_tools(self))
            }

            /// Helper method to check if resources are available (used in tests) 
            #[allow(dead_code)]
            pub fn try_get_resources_default(&self) -> Vec<pulseengine_mcp_protocol::Resource> {
                vec![] // Return empty for now to avoid trait bound issues
            }
        }
    }
}
