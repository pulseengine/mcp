//! # MCP Resource Macro Implementation
//!
//! This module implements the `#[mcp_resource]` macro for automatically generating
//! MCP resource implementations from Rust functions. Resources in MCP allow servers
//! to expose data that clients can read.
//!
//! ## Key Features
//! - Automatic URI template parsing and validation
//! - Type-safe parameter extraction from URIs
//! - Integration with server capabilities auto-detection
//! - Support for both sync and async resource functions
//!
//! ## References
//! - [MCP Specification](https://modelcontextprotocol.io/specification/)
//! - [Building with LLMs Tutorial](https://modelcontextprotocol.io/tutorials/building-mcp-with-llms)

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, FnArg, ItemFn, PatType, parse2};

use crate::utils::{extract_doc_comments, parse_attribute_args};

/// Configuration for the mcp_resource macro
#[derive(Debug, Default)]
pub struct McpResourceConfig {
    /// URI template for the resource (e.g., "file://{path}")
    pub uri_template: Option<String>,
    /// Custom name for the resource (defaults to function name)
    pub name: Option<String>,
    /// Custom description (defaults to doc comments)
    pub description: Option<String>,
    /// MIME type of the resource content
    pub mime_type: Option<String>,
}

/// Parse macro attributes into McpResourceConfig
fn parse_resource_attributes(args: TokenStream) -> syn::Result<McpResourceConfig> {
    let mut config = McpResourceConfig::default();
    let parsed_args = parse_attribute_args(args)?;

    for (key, value) in parsed_args {
        match key.as_str() {
            "uri_template" => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = value
                {
                    config.uri_template = Some(lit_str.value());
                } else {
                    return Err(Error::new_spanned(
                        value,
                        "uri_template must be a string literal",
                    ));
                }
            }
            "name" => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = value
                {
                    config.name = Some(lit_str.value());
                } else {
                    return Err(Error::new_spanned(value, "name must be a string literal"));
                }
            }
            "description" => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = value
                {
                    config.description = Some(lit_str.value());
                } else {
                    return Err(Error::new_spanned(
                        value,
                        "description must be a string literal",
                    ));
                }
            }
            "mime_type" => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = value
                {
                    config.mime_type = Some(lit_str.value());
                } else {
                    return Err(Error::new_spanned(
                        value,
                        "mime_type must be a string literal",
                    ));
                }
            }
            _ => {
                return Err(Error::new_spanned(
                    value,
                    format!("Unknown attribute: {key}"),
                ));
            }
        }
    }

    // Validate that uri_template is provided
    if config.uri_template.is_none() {
        return Err(Error::new(
            Span::call_site(),
            "uri_template is required for mcp_resource",
        ));
    }

    Ok(config)
}

/// Extract URI template parameters (e.g., "{path}" from "file://{path}")
fn extract_uri_parameters(uri_template: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut chars = uri_template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut param = String::new();
            for ch in chars.by_ref() {
                if ch == '}' {
                    if !param.is_empty() {
                        params.push(param);
                    }
                    break;
                }
                param.push(ch);
            }
        }
    }

    params
}

/// Generate resource parameter extraction code
fn generate_parameter_extraction(
    uri_params: &[String],
    fn_inputs: &[&PatType],
) -> syn::Result<TokenStream> {
    if uri_params.len() != fn_inputs.len() {
        return Err(Error::new(
            Span::call_site(),
            format!(
                "URI template has {} parameters but function has {} parameters",
                uri_params.len(),
                fn_inputs.len()
            ),
        ));
    }

    let extractions = uri_params
        .iter()
        .zip(fn_inputs.iter())
        .map(|(param_name, pat_type)| {
            let param_ident = &pat_type.pat;
            let param_type = &pat_type.ty;

            quote! {
                let #param_ident: #param_type = uri_params.get(#param_name)
                    .ok_or_else(|| pulseengine_mcp_protocol::Error::invalid_params(
                        format!("Missing parameter: {}", #param_name)
                    ))?
                    .parse()
                    .map_err(|e| pulseengine_mcp_protocol::Error::invalid_params(
                        format!("Invalid parameter {}: {}", #param_name, e)
                    ))?;
            }
        });

    Ok(quote! {
        #(#extractions)*
    })
}

/// Generate the resource implementation
fn generate_resource_impl(
    config: &McpResourceConfig,
    original_fn: &ItemFn,
) -> syn::Result<TokenStream> {
    let fn_name = &original_fn.sig.ident;
    let fn_name_string = fn_name.to_string();
    let resource_name = config.name.as_ref().unwrap_or(&fn_name_string);
    let uri_template = config.uri_template.as_ref().unwrap();
    let description = config.description.clone().unwrap_or_else(|| {
        extract_doc_comments(&original_fn.attrs)
            .unwrap_or_else(|| format!("Resource: {resource_name}"))
    });
    let default_mime_type = "text/plain".to_string();
    let mime_type = config.mime_type.as_ref().unwrap_or(&default_mime_type);

    // Extract URI parameters
    let uri_params = extract_uri_parameters(uri_template);

    // Extract function parameters (excluding &self if present)
    let fn_inputs: Vec<&PatType> = original_fn
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(pat_type) => Some(pat_type),
            FnArg::Receiver(_) => None, // Skip &self
        })
        .collect();

    // Generate parameter extraction code
    let param_extraction = generate_parameter_extraction(&uri_params, &fn_inputs)?;

    // Generate parameter names for function call
    let param_names: Vec<_> = fn_inputs.iter().map(|p| &p.pat).collect();

    // Determine if function is async
    let is_async = original_fn.sig.asyncness.is_some();
    let await_token = if is_async { quote!(.await) } else { quote!() };

    // Generate the resource handler function name
    let handler_name = syn::Ident::new(
        &format!("__mcp_resource_handler_{fn_name}"),
        Span::call_site(),
    );

    // Generate unique resource info function name
    let resource_info_name =
        syn::Ident::new(&format!("__mcp_resource_info_{fn_name}"), Span::call_site());

    Ok(quote! {
        // Original function (unchanged)
        #original_fn

        // Generated resource handler
        pub async fn #handler_name(
            &self,
            uri: &str,
            uri_params: &std::collections::HashMap<String, String>,
        ) -> std::result::Result<pulseengine_mcp_protocol::ResourceContents, pulseengine_mcp_protocol::Error> {
            // Extract parameters from URI
            #param_extraction

            // Call the original function
            let result = self.#fn_name(#(#param_names),*)#await_token;

            // Convert result to ResourceContents
            match result {
                Ok(content) => {
                    let content_str = serde_json::to_string(&content)
                        .map_err(|e| pulseengine_mcp_protocol::Error::internal_error(
                            format!("Failed to serialize resource content: {}", e)
                        ))?;

                    Ok(pulseengine_mcp_protocol::ResourceContents {
                        uri: uri.to_string(),
                        mime_type: Some(#mime_type.to_string()),
                        text: Some(content_str),
                        blob: None,
                    })
                }
                Err(e) => Err(pulseengine_mcp_protocol::Error::internal_error(
                    format!("Resource error: {}", e)
                )),
            }
        }

        // Resource metadata for capability registration
        pub fn #resource_info_name() -> pulseengine_mcp_protocol::Resource {
            pulseengine_mcp_protocol::Resource {
                uri: #uri_template.to_string(),
                name: #resource_name.to_string(),
                description: Some(#description.to_string()),
                mime_type: Some(#mime_type.to_string()),
                annotations: None,
                raw: None,
            }
        }
    })
}

/// Main implementation function for the mcp_resource macro
pub fn mcp_resource_impl(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    // Parse the configuration from macro arguments
    let config = parse_resource_attributes(args)?;

    // Parse the function
    let original_fn: ItemFn = parse2(input)?;

    // Validate function signature
    if original_fn.sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &original_fn.sig,
            "Resource functions must have at least one parameter",
        ));
    }

    // Generate the implementation
    generate_resource_impl(&config, &original_fn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_extract_uri_parameters() {
        assert_eq!(extract_uri_parameters("file://{path}"), vec!["path"]);

        assert_eq!(
            extract_uri_parameters("db://{database}/{table}"),
            vec!["database", "table"]
        );

        assert_eq!(
            extract_uri_parameters("static://content"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_parse_resource_attributes() {
        let args = quote! {
            uri_template = "file://{path}",
            name = "file_reader",
            mime_type = "application/json"
        };

        let config = parse_resource_attributes(args).unwrap();
        assert_eq!(config.uri_template, Some("file://{path}".to_string()));
        assert_eq!(config.name, Some("file_reader".to_string()));
        assert_eq!(config.mime_type, Some("application/json".to_string()));
    }
}
