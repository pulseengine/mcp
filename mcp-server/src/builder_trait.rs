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

    /// Configure stdio logging - disables all logging for stdio transport conformance
    ///
    /// For stdio transport, the MCP protocol requires that ONLY JSON-RPC messages
    /// go to stdout. All logging must either be disabled or sent through the MCP
    /// logging protocol. This function completely disables tracing output to ensure
    /// stdio transport conformance.
    fn configure_stdio_logging() {
        #[cfg(feature = "stdio-logging")]
        {
            use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

            // Check if user explicitly wants debug logging via RUST_LOG
            let env_filter = std::env::var("RUST_LOG");

            if env_filter.is_ok() {
                // User explicitly set RUST_LOG, respect it but warn
                eprintln!("Warning: RUST_LOG is set for stdio transport.");
                eprintln!("This may interfere with MCP conformance tests.");
                eprintln!("Logging will go to stderr.");

                use tracing_subscriber::fmt;

                let filter_layer = EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::try_new("info").unwrap());

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
                    .unwrap_or(());
            } else {
                // No RUST_LOG set - disable all logging for conformance
                // Initialize with a filter that blocks everything
                let filter_layer = EnvFilter::try_new("off").unwrap();

                tracing_subscriber::registry()
                    .with(filter_layer)
                    .try_init()
                    .unwrap_or(());
            }
        }

        #[cfg(not(feature = "stdio-logging"))]
        {
            // For non-feature builds, just initialize a no-op subscriber
            // This prevents any logging from happening
            use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

            let filter_layer = EnvFilter::try_new("off").unwrap();
            tracing_subscriber::registry()
                .with(filter_layer)
                .try_init()
                .unwrap_or(());
        }
    }
}

/// Simplified service type alias for convenience
pub type McpService<B> = B;
