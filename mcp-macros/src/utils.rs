//! Utility functions for macro implementations

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Expr, Lit, Meta};

/// Custom parser for attribute arguments
struct AttributeArgs {
    args: Vec<(String, Expr)>,
}

impl syn::parse::Parse for AttributeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = Vec::new();

        while !input.is_empty() {
            let meta: syn::Meta = input.parse()?;

            match meta {
                syn::Meta::NameValue(name_value) => {
                    let key = name_value
                        .path
                        .get_ident()
                        .ok_or_else(|| {
                            syn::Error::new_spanned(
                                &name_value.path,
                                "Expected identifier (parameter name)",
                            )
                        })?
                        .to_string();
                    args.push((key, name_value.value));
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Expected name-value pairs like key = \"value\". Example: #[mcp_server(name = \"My Server\")]",
                    ));
                }
            }

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(AttributeArgs { args })
    }
}

/// Parse attribute arguments into a vector of key-value pairs
pub fn parse_attribute_args(args: TokenStream) -> syn::Result<Vec<(String, Expr)>> {
    if args.is_empty() {
        return Ok(Vec::new());
    }

    let parsed = syn::parse2::<AttributeArgs>(args)?;
    Ok(parsed.args)
}

/// Extract documentation from function attributes (alias for backward compatibility)
pub fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    extract_doc_comment(attrs)
}

/// Extract documentation from function attributes
pub fn extract_doc_comment(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();

    for attr in attrs {
        if let Meta::NameValue(meta) = &attr.meta {
            if meta.path.is_ident("doc") {
                if let Expr::Lit(expr_lit) = &meta.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        let content = lit_str.value().trim().to_string();
                        if !content.is_empty() {
                            docs.push(content);
                        }
                    }
                }
            }
        }
    }

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

/// Generate JSON schema for a type
pub fn generate_schema_for_type(ty: &syn::Type) -> TokenStream {
    quote! {
        {
            let schema = schemars::schema_for!(#ty);
            serde_json::to_value(schema).unwrap_or_else(|_| serde_json::json!({}))
        }
    }
}

/// Convert a function name to tool name (snake_case)
pub fn function_name_to_tool_name(ident: &syn::Ident) -> String {
    ident.to_string()
}

/// Generate a unique identifier for a tool
#[allow(dead_code)]
pub fn generate_tool_id(base_name: &str) -> syn::Ident {
    syn::Ident::new(
        &format!("{base_name}_tool_def"),
        proc_macro2::Span::call_site(),
    )
}

/// Check if a type is an Option<T>
pub fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Extract the inner type from Option<T>
#[allow(dead_code)]
pub fn extract_option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

/// Generate error handling code for a function result
pub fn generate_error_handling(return_type: &syn::ReturnType) -> TokenStream {
    match return_type {
        syn::ReturnType::Default => {
            quote! {
                Ok(pulseengine_mcp_protocol::CallToolResult {
                    content: vec![],
                    is_error: Some(false),
                    structured_content: None,
                })
            }
        }
        syn::ReturnType::Type(_, ty) => {
            // Check if it's a Result type
            if let syn::Type::Path(type_path) = &**ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Result" {
                        // It's already a Result, wrap it properly for the dispatch context
                        return quote! {
                            match result {
                                Ok(value) => Ok(pulseengine_mcp_protocol::CallToolResult {
                                    content: vec![pulseengine_mcp_protocol::Content::text(format!("{:?}", value))],
                                    is_error: Some(false),
                                    structured_content: None,
                                }),
                                Err(e) => Err(pulseengine_mcp_protocol::Error::internal_error(e.to_string())),
                            }
                        };
                    }
                }
            }

            // Not a Result, wrap it with simple Display formatting
            quote! {
                Ok(pulseengine_mcp_protocol::CallToolResult {
                    content: vec![pulseengine_mcp_protocol::Content::text(result.to_string())],
                    is_error: Some(false),
                    structured_content: None,
                })
            }
        }
    }
}

/// Generate package version from environment
pub fn get_package_version() -> TokenStream {
    quote! {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

/// Generate package name from environment
#[allow(dead_code)]
pub fn get_package_name() -> TokenStream {
    quote! {
        env!("CARGO_PKG_NAME").to_string()
    }
}
