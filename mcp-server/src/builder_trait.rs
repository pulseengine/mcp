//! Simplified builder trait for reducing generated code in server implementations
//!
//! This module provides a basic trait that servers can implement to get
//! common functionality without generating repetitive code.

use crate::McpBackend;

/// Simplified builder trait that provides common functionality for all MCP servers
pub trait McpServerBuilder: McpBackend + Sized {
    /// Create server with default configuration
    fn with_defaults() -> Self
    where
        Self: Default,
    {
        Self::default()
    }

    /// Configure stdio logging to redirect logs to stderr
    fn configure_stdio_logging() {
        #[cfg(feature = "stdio-logging")]
        {
            use tracing_subscriber::{
                EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
            };

            let filter_layer = EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap();

            let fmt_layer = fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_thread_ids(false)
                .with_thread_names(false);

            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .try_init()
                .unwrap_or(()); // Ignore if already initialized
        }

        #[cfg(not(feature = "stdio-logging"))]
        {
            eprintln!("Warning: stdio logging configuration requires 'stdio-logging' feature");
            eprintln!(
                "Add to Cargo.toml: pulseengine-mcp-macros = {{ version = \"*\", features = [\"stdio-logging\"] }}"
            );
            eprintln!("And: tracing-subscriber = \"0.3\"");
        }
    }
}

/// Simplified service type alias for convenience
pub type McpService<B> = B;
