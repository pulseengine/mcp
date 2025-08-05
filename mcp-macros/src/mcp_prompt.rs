//! # MCP Prompt Macro Implementation
//!
//! This module implements the `#[mcp_prompt]` macro for automatically generating
//! MCP prompt implementations from Rust functions. Prompts in MCP allow servers
//! to provide reusable prompt templates for AI interactions.
//!
//! ## Key Features
//! - Automatic prompt argument validation and processing
//! - Type-safe parameter handling
//! - Integration with server capabilities auto-detection
//! - Support for both sync and async prompt functions
//!
//! ## References
//! - [MCP Specification](https://modelcontextprotocol.io/specification/)
//! - [Building with LLMs Tutorial](https://modelcontextprotocol.io/tutorials/building-mcp-with-llms)

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, FnArg, ItemFn, PatType, parse2};

use crate::utils::{extract_doc_comments, parse_attribute_args};

/// Configuration for the mcp_prompt macro
#[derive(Debug, Default)]
pub struct McpPromptConfig {
    /// Name of the prompt (defaults to function name)
    pub name: Option<String>,
    /// Custom description (defaults to doc comments)
    pub description: Option<String>,
    /// Arguments that the prompt accepts
    pub arguments: Option<Vec<String>>,
}

/// Parse macro attributes into McpPromptConfig
fn parse_prompt_attributes(args: TokenStream) -> syn::Result<McpPromptConfig> {
    let mut config = McpPromptConfig::default();
    let parsed_args = parse_attribute_args(args)?;

    for (key, value) in parsed_args {
        match key.as_str() {
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
            "arguments" => {
                // Parse array of strings for arguments
                if let syn::Expr::Array(array) = value {
                    let mut args = Vec::new();
                    for elem in array.elems {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit_str),
                            ..
                        }) = elem
                        {
                            args.push(lit_str.value());
                        } else {
                            return Err(Error::new_spanned(
                                elem,
                                "argument names must be string literals",
                            ));
                        }
                    }
                    config.arguments = Some(args);
                } else {
                    return Err(Error::new_spanned(
                        value,
                        "arguments must be an array of strings",
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

    Ok(config)
}

/// Generate prompt parameter extraction code
fn generate_prompt_parameter_extraction(fn_inputs: &[&PatType]) -> syn::Result<TokenStream> {
    let extractions = fn_inputs.iter().map(|pat_type| {
        let param_ident = &pat_type.pat;
        let param_type = &pat_type.ty;
        let param_name = quote!(#param_ident).to_string();

        quote! {
            let #param_ident: #param_type = arguments.get(#param_name)
                .ok_or_else(|| pulseengine_mcp_protocol::McpError::InvalidParams {
                    message: format!("Missing argument: {}", #param_name),
                })?
                .clone();
        }
    });

    Ok(quote! {
        #(#extractions)*
    })
}

/// Generate the prompt implementation
fn generate_prompt_impl(config: &McpPromptConfig, original_fn: &ItemFn) -> syn::Result<TokenStream> {
    let fn_name = &original_fn.sig.ident;
    let fn_name_string = fn_name.to_string();
    let prompt_name = config.name.as_ref().unwrap_or(&fn_name_string);
    let description = config.description.clone().unwrap_or_else(|| {
        extract_doc_comments(&original_fn.attrs).unwrap_or_else(|| format!("Prompt: {prompt_name}"))
    });

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
    let param_extraction = generate_prompt_parameter_extraction(&fn_inputs)?;

    // Generate argument schema for prompt info
    let argument_schemas = fn_inputs.iter().map(|pat_type| {
        let param_name = quote!(#pat_type.pat).to_string();
        let param_type = &pat_type.ty;

        quote! {
            serde_json::json!({
                "name": #param_name,
                "description": format!("Parameter of type {}", stringify!(#param_type)),
                "required": true
            })
        }
    });

    // Determine if function is async
    let is_async = original_fn.sig.asyncness.is_some();
    let await_token = if is_async { quote!(.await) } else { quote!() };

    // Generate the prompt handler function name
    let handler_name = syn::Ident::new(
        &format!("__mcp_prompt_handler_{fn_name}"),
        Span::call_site(),
    );

    // Generate parameter passing for function call
    let param_names: Vec<_> = fn_inputs.iter().map(|p| &p.pat).collect();

    Ok(quote! {
        // Original function (unchanged)
        #original_fn

        // Generated prompt handler
        pub async fn #handler_name(
            &self,
            name: &str,
            arguments: &std::collections::HashMap<String, serde_json::Value>,
        ) -> std::result::Result<pulseengine_mcp_protocol::GetPromptResult, pulseengine_mcp_protocol::McpError> {
            // Extract parameters from arguments
            #param_extraction

            // Call the original function
            let result = self.#fn_name(#(#param_names),*)#await_token;

            // Convert result to GetPromptResult
            match result {
                Ok(prompt_message) => {
                    Ok(pulseengine_mcp_protocol::GetPromptResult {
                        description: Some(#description.to_string()),
                        messages: vec![prompt_message],
                    })
                }
                Err(e) => Err(pulseengine_mcp_protocol::McpError::InternalError {
                    message: format!("Prompt error: {}", e),
                }),
            }
        }

        // Prompt metadata for capability registration
        pub fn __mcp_prompt_info() -> pulseengine_mcp_protocol::Prompt {
            pulseengine_mcp_protocol::Prompt {
                name: #prompt_name.to_string(),
                description: Some(#description.to_string()),
                arguments: Some(vec![#(#argument_schemas),*]),
            }
        }
    })
}

/// Main implementation function for the mcp_prompt macro
pub fn mcp_prompt_impl(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    // Parse the configuration from macro arguments
    let config = parse_prompt_attributes(args)?;

    // Parse the function
    let original_fn: ItemFn = parse2(input)?;

    // Validate function signature
    if original_fn.sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &original_fn.sig,
            "Prompt functions must have at least one parameter",
        ));
    }

    // Generate the implementation
    generate_prompt_impl(&config, &original_fn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_parse_prompt_attributes() {
        let args = quote! {
            name = "code_review",
            description = "Generate a code review prompt"
        };

        let config = parse_prompt_attributes(args).unwrap();
        assert_eq!(config.name, Some("code_review".to_string()));
        assert_eq!(
            config.description,
            Some("Generate a code review prompt".to_string())
        );
    }

    #[test]
    fn test_parse_prompt_attributes_with_arguments() {
        let args = quote! {
            name = "test_prompt",
            arguments = ["code", "language", "style"]
        };

        let config = parse_prompt_attributes(args).unwrap();
        assert_eq!(config.name, Some("test_prompt".to_string()));
        assert_eq!(
            config.arguments,
            Some(vec![
                "code".to_string(),
                "language".to_string(),
                "style".to_string()
            ])
        );
    }
}
