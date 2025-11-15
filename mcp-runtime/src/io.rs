//! Async I/O abstractions
//!
//! This module provides unified async I/O traits and types that work across
//! both native (Tokio) and WASM (wstd) platforms.

// Native: Re-export tokio's full I/O module
#[cfg(not(target_family = "wasm"))]
pub use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, AsyncBufRead, AsyncBufReadExt,
    BufReader, stdin, stdout, stderr, Stdin, Stdout, Stderr,
};

// WASM: Use wstd and futures traits
#[cfg(all(target_family = "wasm", target_os = "wasi"))]
pub use wstd::io::{stdin, stdout, stderr, Stdin, Stdout, Stderr};

// Re-export futures traits and BufReader for WASM since wstd uses futures
#[cfg(all(target_family = "wasm", target_os = "wasi"))]
pub use futures::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt,
    AsyncBufRead, AsyncBufReadExt, BufReader,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_stdio_available() {
        let _stdin = stdin();
        let _stdout = stdout();
        let _stderr = stderr();
    }
}
