//! Runtime abstraction layer for MCP Framework
//!
//! This crate provides a unified async runtime interface that works across both
//! native platforms (using Tokio) and WebAssembly (using wstd for wasm32-wasip2).
//!
//! # Architecture
//!
//! The abstraction is designed to:
//! - Provide zero-cost abstraction on native platforms (direct Tokio usage)
//! - Enable WASM compatibility through wstd on wasm32-wasip2
//! - Allow gradual migration of the codebase
//! - Support feature flags for different runtime capabilities
//!
//! # Usage
//!
//! ```rust,no_run
//! use pulseengine_mcp_runtime::spawn;
//!
//! async fn my_task() {
//!     println!("Running on the runtime!");
//! }
//!
//! // Spawns on Tokio (native) or wstd (WASM)
//! spawn(my_task());
//! ```
//!
//! # Feature Flags
//!
//! - `io` - I/O utilities (stdin/stdout/stderr)
//! - `net` - Networking support
//! - `time` - Time and sleep utilities
//! - `sync` - Synchronization primitives
//! - `full` - All features
//!
//! # Platform Support
//!
//! - **Native (not WASM)**: Uses Tokio runtime
//! - **wasm32-wasip2**: Uses wstd runtime
//! - **Other WASM targets**: Not currently supported

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod io;
pub mod runtime;
pub mod sync;
pub mod time;

// Re-export core types
pub use runtime::{spawn, spawn_blocking, block_on, sleep};
pub use io::{AsyncRead, AsyncWrite, AsyncBufRead, BufReader};
pub use thiserror::Error;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::runtime::{spawn, spawn_blocking, block_on, sleep};
    pub use crate::io::{AsyncRead, AsyncWrite, AsyncBufRead, AsyncReadExt, AsyncWriteExt, BufReader};
    pub use crate::sync::{Mutex, RwLock};
}

/// Runtime errors
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Task join error
    #[error("Task join error: {0}")]
    JoinError(String),

    /// Runtime not available
    #[error("Runtime not available: {0}")]
    NotAvailable(String),
}

/// Result type for runtime operations
pub type Result<T> = std::result::Result<T, RuntimeError>;
