//! Implementation of the #[mcp_tool] macro

use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItemFn, ItemFn, ReturnType};

use crate::utils::*;

/// Information about a resource method for code generation
#[derive(Clone)]
struct ResourceInfo {
    method_name: syn::Ident,
    #[allow(dead_code)]
    resource_name: String,
    #[allow(dead_code)]
    description: String,
    #[allow(dead_code)]
    uri_template: String,
    path_pattern: String,
    param_names: Vec<String>,
    method_param_names: Vec<syn::Ident>,
    method_param_types: Vec<syn::Type>,
    is_async: bool,
    has_params: bool,
}

/// Helper to parse URI template and extract path pattern for matchit
fn parse_uri_template(uri_template: &str) -> (String, Vec<String>) {
    // Convert "timedate://current-time/{timezone}" to "/current-time/{timezone}"
    let path = if let Some(scheme_end) = uri_template.find("://") {
        let after_scheme = &uri_template[scheme_end + 3..];

        // For custom URI schemes like "timedate://current-time/{timezone}",
        // treat everything after :// as the path since there's no host part
        // Add leading slash to make it a proper matchit path
        return (
            format!("/{after_scheme}"),
            extract_uri_parameters(after_scheme),
        );
    } else {
        // No scheme, assume it's already a path
        if uri_template.starts_with('/') {
            uri_template
        } else {
            // Add leading slash
            return (
                format!("/{uri_template}"),
                extract_uri_parameters(uri_template),
            );
        }
    };

    (path.to_string(), extract_uri_parameters(path))
}

/// Extract parameter names from URI template path
fn extract_uri_parameters(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut param_name = String::new();
            for ch in chars.by_ref() {
                if ch == '}' {
                    break;
                }
                param_name.push(ch);
            }
            if !param_name.is_empty() {
                params.push(param_name);
            }
        }
    }

    params
}

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

/// Generate matchit-based resource provider implementation
fn generate_matchit_resource_impl(
    resource_definitions: &[TokenStream],
    resource_infos: &[ResourceInfo],
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: &Option<&syn::WhereClause>,
    struct_name: &syn::Ident,
) -> TokenStream {
    if resource_infos.is_empty() {
        // Generate empty implementation when no resources are defined
        return quote! {
            impl #impl_generics pulseengine_mcp_server::McpResourcesProvider for #struct_name #ty_generics #where_clause {
                fn get_available_resources(&self) -> Vec<pulseengine_mcp_protocol::Resource> {
                    vec![]
                }

                fn read_resource_impl(
                    &self,
                    request: pulseengine_mcp_protocol::ReadResourceRequestParam,
                ) -> impl std::future::Future<Output = std::result::Result<pulseengine_mcp_protocol::ReadResourceResult, pulseengine_mcp_protocol::Error>> + Send {
                    async move {
                        Err(pulseengine_mcp_protocol::Error::invalid_params(
                            format!("Unknown resource: {}", request.uri)
                        ))
                    }
                }
            }
        };
    }

    // Generate resource handler enum
    let resource_handler_variants: Vec<_> = resource_infos
        .iter()
        .enumerate()
        .map(|(i, _info)| {
            let variant_name = format_ident!("Resource{}", i);
            quote! { #variant_name }
        })
        .collect();

    let resource_handler_enum = quote! {
        #[derive(Clone, Copy)]
        enum ResourceHandler {
            #(#resource_handler_variants),*
        }
    };

    // Generate router setup code
    let router_inserts: Vec<_> = resource_infos
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let variant_name = format_ident!("Resource{}", i);
            let path_pattern = &info.path_pattern;
            quote! {
                router.insert(#path_pattern, ResourceHandler::#variant_name)
                    .map_err(|e| pulseengine_mcp_protocol::Error::internal_error(
                        format!("Failed to insert route {}: {}", #path_pattern, e)
                    ))?;
            }
        })
        .collect();

    // Generate match arms for resource dispatch
    let resource_match_arms: Vec<_> = resource_infos
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let variant_name = format_ident!("Resource{}", i);
            let method_name = &info.method_name;
            let await_token = if info.is_async {
                quote!(.await)
            } else {
                quote!()
            };

            if info.has_params {
                // Generate parameter extraction for parameterized resources
                let param_extractions: Vec<_> = info
                    .param_names
                    .iter()
                    .enumerate()
                    .map(|(param_idx, param_name)| {
                        let method_param = match info.method_param_names.get(param_idx) {
                            Some(p) => p,
                            None => return quote! {},
                        };
                        let param_type = match info.method_param_types.get(param_idx) {
                            Some(t) => t,
                            None => return quote! {},
                        };

                        // Check if the type is String - if so, no parsing needed
                        let is_string_type = if let syn::Type::Path(type_path) = param_type {
                            type_path.path.segments.last()
                                .map(|seg| seg.ident == "String")
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        if is_string_type {
                            // String type - just convert directly
                            quote! {
                                let #method_param: #param_type = matched.params.get(#param_name)
                                    .unwrap_or("")
                                    .to_string();
                            }
                        } else {
                            // Non-String type - parse from string
                            quote! {
                                let #method_param: #param_type = matched.params.get(#param_name)
                                    .unwrap_or("")
                                    .parse()
                                    .map_err(|e| pulseengine_mcp_protocol::Error::invalid_params(
                                        format!("Failed to parse parameter '{}': {}", #param_name, e)
                                    ))?;
                            }
                        }
                    })
                    .collect();

                let method_call_params: Vec<_> = info
                    .method_param_names
                    .iter()
                    .map(|param| quote! { #param })
                    .collect();

                quote! {
                    ResourceHandler::#variant_name => {
                        #(#param_extractions)*
                        let result = self.#method_name(#(#method_call_params),*)#await_token;
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
                                        _meta: None,
                                    }]
                                })
                            }
                            Err(e) => Err(pulseengine_mcp_protocol::Error::internal_error(
                                format!("Resource error: {}", e)
                            ))
                        }
                    }
                }
            } else {
                // No parameters - simple resource call
                quote! {
                    ResourceHandler::#variant_name => {
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
                                        _meta: None,
                                    }]
                                })
                            }
                            Err(e) => Err(pulseengine_mcp_protocol::Error::internal_error(
                                format!("Resource error: {}", e)
                            ))
                        }
                    }
                }
            }
        })
        .collect();

    // Generate the complete implementation
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
                    // Helper to extract path from URI
                    fn extract_path_from_uri(uri: &str) -> String {
                        if let Some(pos) = uri.find("://") {
                            // For custom URI schemes like "timedate://current-time/{timezone}",
                            // treat everything after :// as the path
                            format!("/{}", &uri[pos + 3..])
                        } else {
                            if uri.starts_with('/') {
                                uri.to_string()
                            } else {
                                format!("/{}", uri)
                            }
                        }
                    }

                    // Resource handler enum (local to this function)
                    #resource_handler_enum

                    // Build router
                    let mut build_router = || -> Result<matchit::Router<ResourceHandler>, pulseengine_mcp_protocol::Error> {
                        let mut router = matchit::Router::new();
                        #(#router_inserts)*
                        Ok(router)
                    };

                    let router = build_router()?;
                    let uri = &request.uri;
                    let path = extract_path_from_uri(uri);

                    // Match URI against router
                    match router.at(&path) {
                        Ok(matched) => {
                            match matched.value {
                                #(#resource_match_arms)*
                            }
                        }
                        Err(_) => Err(pulseengine_mcp_protocol::Error::invalid_params(
                            format!("Unknown resource: {}", uri)
                        ))
                    }
                }
            }
        }
    }
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

    // Collect resource information for matchit router generation
    let mut resource_infos = Vec::new();

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

                    // Parse URI template to get matchit path pattern
                    let (path_pattern, template_param_names) = parse_uri_template(&uri_template);

                    // Extract method parameter names and types
                    let mut method_param_names = Vec::new();
                    let mut method_param_types = Vec::new();
                    for input in &method.sig.inputs {
                        match input {
                            syn::FnArg::Receiver(_) => continue,
                            syn::FnArg::Typed(pat_type) => {
                                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                                    method_param_names.push(pat_ident.ident.clone());
                                    method_param_types.push((*pat_type.ty).clone());
                                }
                            }
                        }
                    }

                    let resource_info = ResourceInfo {
                        method_name: method_name.clone(),
                        resource_name: resource_name.clone(),
                        description: description.clone(),
                        uri_template: uri_template.clone(),
                        path_pattern,
                        param_names: template_param_names,
                        method_param_names,
                        method_param_types,
                        is_async: method.sig.asyncness.is_some(),
                        has_params: method.sig.inputs.len() > 1,
                    };

                    resource_infos.push(resource_info);

                    // Create resource definition for list_resources
                    resource_definitions.push(quote! {
                        pulseengine_mcp_protocol::Resource {
                            uri: #uri_template.to_string(),
                            name: #resource_name.to_string(),
                            title: None,
                            description: Some(#description.to_string()),
                            mime_type: Some("application/json".to_string()),
                            annotations: None,
                            icons: None,
                            raw: None,
                            _meta: None,
                        }
                    });
                } else {
                    // Handle as tool (existing logic)
                    let tool_name = method.sig.ident.to_string();

                    // Extract documentation from method
                    let doc_comment = extract_doc_comment(&method.attrs);
                    let description =
                        doc_comment.unwrap_or_else(|| format!("Generated tool for {tool_name}"));

                    // Generate JSON schema for parameters from function signature
                    let schema = generate_input_schema_for_method(&method.sig)?;

                    // Create tool definition
                    tool_definitions.push(quote! {
                        pulseengine_mcp_protocol::Tool {
                            name: #tool_name.to_string(),
                            title: None,
                            description: #description.to_string(),
                            input_schema: #schema,
                            output_schema: None,
                            annotations: None,
                            icons: None,
                            _meta: None,
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

    // Generate matchit-based resource provider implementation
    let resource_provider_impl = generate_matchit_resource_impl(
        &resource_definitions,
        &resource_infos,
        &impl_generics,
        &ty_generics,
        &where_clause,
        &struct_name,
    );

    // Resource backend override temporarily disabled to avoid trait conflicts

    // Strip #[mcp_resource] attributes from the impl block before outputting
    let mut cleaned_impl_block = impl_block.clone();
    for item in &mut cleaned_impl_block.items {
        if let syn::ImplItem::Fn(method) = item {
            method
                .attrs
                .retain(|attr| !attr.path().is_ident("mcp_resource"));
        }
    }

    // Generate the enhanced impl block that uses a trait-based approach to avoid method conflicts
    let enhanced_impl = quote! {
        #cleaned_impl_block

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

    let helper_methods =
        generate_helper_methods(&struct_name, &impl_generics, &ty_generics, &where_clause);

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
                }
            }
        }
    }

    // Detect single parameter case
    if param_names.len() == 1 {
        let param_name = &param_names[0];
        let param_type = &param_types[0];

        // Check if it's a custom struct (not primitive/std type)
        if is_primitive_or_std_type(param_type) {
            // Primitive - extract by name (standard behavior)
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
        } else {
            // Custom struct - deserialize entire args object (flattened)
            param_fields.push(quote! {
                match serde_json::from_value::<#param_type>(
                    serde_json::Value::Object(args.clone())
                ) {
                    Ok(value) => value,
                    Err(e) => return Err(pulseengine_mcp_protocol::Error::invalid_params(
                        format!("Failed to deserialize parameters for tool '{}': {}", #tool_name, e)
                    )),
                }
            });
        }
    } else {
        // Multi-parameter - extract by name
        for (param_name, param_type) in param_names.iter().zip(param_types.iter()) {
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
                title: None,
                description: #description_expr,
                input_schema: #input_schema,
                output_schema: None,
                annotations: None,
                icons: None,
                _meta: None,
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

/// Generate JSON schema for method parameters from function signature
fn generate_input_schema_for_method(sig: &syn::Signature) -> syn::Result<TokenStream> {
    // Collect non-self parameters
    let params: Vec<_> = sig
        .inputs
        .iter()
        .filter_map(|input| {
            if let syn::FnArg::Typed(pat_type) = input {
                Some(pat_type)
            } else {
                None
            }
        })
        .collect();

    match params.len() {
        0 => {
            // No parameters - return empty schema
            Ok(quote! {
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                })
            })
        }
        1 => {
            // Single parameter - use JsonSchema trait (requires JsonSchema derive)
            let param_type = &params[0].ty;
            Ok(quote! {
                {
                    use ::schemars::JsonSchema;
                    // Use the JsonSchema trait to get the schema
                    let mut schema_gen = ::schemars::SchemaGenerator::default();
                    let schema = <#param_type as ::schemars::JsonSchema>::json_schema(&mut schema_gen);
                    ::serde_json::to_value(&schema).unwrap_or_else(|_|
                        ::serde_json::json!({"type": "object", "properties": {}})
                    )
                }
            })
        }
        _ => {
            // Multiple parameters - generate schema from individual parameter types
            generate_multi_parameter_schema(&params)
        }
    }
}

/// Generate JSON schema for multiple individual parameters
fn generate_multi_parameter_schema(params: &[&syn::PatType]) -> syn::Result<TokenStream> {
    let mut properties = Vec::new();
    let mut required_fields = Vec::new();

    for param in params {
        // Extract parameter name
        let param_name = match &*param.pat {
            syn::Pat::Ident(ident) => ident.ident.to_string(),
            _ => {
                return Err(syn::Error::new_spanned(
                    param.pat.clone(),
                    "Complex parameter patterns are not supported",
                ));
            }
        };

        // Extract parameter type and determine if it's optional
        let param_type = &param.ty;
        let (is_optional, inner_type) = extract_option_inner_type(param_type);

        if !is_optional {
            required_fields.push(param_name.clone());
        }

        // Generate schema for this parameter type
        let type_schema =
            generate_type_schema_for_type(if is_optional { inner_type } else { param_type });

        properties.push(quote! {
            (#param_name, #type_schema)
        });
    }

    Ok(quote! {
        {
            let mut properties = ::serde_json::Map::new();
            #(
                properties.insert(#properties.0.to_string(), #properties.1);
            )*

            let mut schema = ::serde_json::json!({
                "type": "object",
                "properties": properties
            });

            let required_fields: Vec<&str> = vec![#(#required_fields),*];
            if !required_fields.is_empty() {
                schema["required"] = ::serde_json::json!(required_fields);
            }

            schema
        }
    })
}

/// Check if a type is Option<T> and extract the inner type T
fn extract_option_inner_type(ty: &syn::Type) -> (bool, &syn::Type) {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return (true, inner_ty);
                    }
                }
            }
        }
    }
    (false, ty)
}

/// Check if a type is a primitive or standard library type (not a custom struct)
fn is_primitive_or_std_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                matches!(
                    segment.ident.to_string().as_str(),
                    "String"
                        | "str"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "Vec"
                        | "HashMap"
                        | "BTreeMap"
                        | "HashSet"
                        | "BTreeSet"
                        | "Option"
                        | "Value" // serde_json::Value
                )
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Generate JSON schema for a specific type
fn generate_type_schema_for_type(ty: &syn::Type) -> TokenStream {
    // Convert Rust type to JSON schema
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                match segment.ident.to_string().as_str() {
                    "String" | "str" => quote! { ::serde_json::json!({"type": "string"}) },
                    "i8" | "i16" | "i32" | "i64" | "isize" => {
                        quote! { ::serde_json::json!({"type": "integer"}) }
                    }
                    "u8" | "u16" | "u32" | "u64" | "usize" => {
                        quote! { ::serde_json::json!({"type": "integer", "minimum": 0}) }
                    }
                    "f32" | "f64" => quote! { ::serde_json::json!({"type": "number"}) },
                    "bool" => quote! { ::serde_json::json!({"type": "boolean"}) },
                    "Vec" => {
                        // Handle Vec<T> - extract T and create array schema
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let items_schema = generate_type_schema_for_type(inner_ty);
                                return quote! { ::serde_json::json!({"type": "array", "items": #items_schema}) };
                            }
                        }
                        quote! { ::serde_json::json!({"type": "array"}) }
                    }
                    _ => {
                        // For custom types, try to use JsonSchema trait as fallback
                        quote! {
                            {
                                // Try to use JsonSchema trait for custom types
                                match std::panic::catch_unwind(|| {
                                    use ::schemars::JsonSchema;
                                    let mut schema_gen = ::schemars::SchemaGenerator::default();
                                    <#ty as ::schemars::JsonSchema>::json_schema(&mut schema_gen)
                                }) {
                                    Ok(schema) => ::serde_json::to_value(&schema).unwrap_or_else(|_|
                                        ::serde_json::json!({"type": "object"})
                                    ),
                                    Err(_) => ::serde_json::json!({"type": "object"})
                                }
                            }
                        }
                    }
                }
            } else {
                quote! { ::serde_json::json!({"type": "object"}) }
            }
        }
        _ => quote! { ::serde_json::json!({"type": "object"}) },
    }
}

/// Generate JSON schema for method parameters (legacy function - keeping for compatibility)
#[allow(dead_code)]
fn generate_parameter_schema(sig: &syn::Signature) -> syn::Result<TokenStream> {
    generate_input_schema_for_method(sig)
}

/// Generate parameter extraction and method call for tools
fn generate_method_call_with_params(
    sig: &syn::Signature,
    method_name: &syn::Ident,
    is_async: bool,
) -> syn::Result<TokenStream> {
    let mut param_declarations = Vec::new();
    let mut param_names = Vec::new();
    let mut param_types = Vec::new();

    // Collect all parameters (skip self)
    for input in &sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => continue, // Skip self
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = &pat_ident.ident;
                    let param_type = &*pat_type.ty;

                    param_names.push(param_name);
                    param_types.push(param_type);
                }
            }
        }
    }

    // Detect single parameter case
    if param_names.len() == 1 {
        let param_name = param_names[0];
        let param_type = param_types[0];

        // Check if it's a custom struct (not primitive/std type)
        if is_primitive_or_std_type(param_type) {
            // Primitive - extract by name (standard behavior)
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
        } else {
            // Custom struct - deserialize entire args object (flattened)
            param_declarations.push(quote! {
                let #param_name: #param_type = serde_json::from_value(
                    serde_json::Value::Object(args.clone())
                ).map_err(|e| pulseengine_mcp_protocol::Error::invalid_params(
                    format!("Failed to deserialize parameters: {}", e)
                ))?;
            });
        }
    } else {
        // Multi-parameter or no parameters - extract by name
        for (param_name, param_type) in param_names.iter().zip(param_types.iter()) {
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
fn generate_helper_methods(
    struct_name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: &Option<&syn::WhereClause>,
) -> TokenStream {
    quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Helper method to get available tools (used in tests)
            #[allow(dead_code)]
            pub fn try_get_tools_default(&self) -> Option<Vec<pulseengine_mcp_protocol::Tool>> {
                Some(<Self as pulseengine_mcp_server::McpToolsProvider>::get_available_tools(self))
            }

            /// Helper method to check if resources are available (used in tests)
            #[allow(dead_code)]
            pub fn try_get_resources_default(&self) -> Vec<pulseengine_mcp_protocol::Resource> {
                <Self as pulseengine_mcp_server::McpResourcesProvider>::get_available_resources(self)
            }

            /// Helper method to read resources (used by mcp_server macro)
            #[allow(dead_code)]
            pub async fn try_read_resource_default(&self, request: pulseengine_mcp_protocol::ReadResourceRequestParam) -> std::result::Result<pulseengine_mcp_protocol::ReadResourceResult, pulseengine_mcp_protocol::Error> {
                <Self as pulseengine_mcp_server::McpResourcesProvider>::read_resource_impl(self, request).await
            }
        }
    }
}
