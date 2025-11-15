//! Time utilities
//!
//! This module provides time-related functionality that works across
//! native and WASM platforms.

use std::time::Duration;

pub use crate::runtime::sleep;

/// Timeout error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed;

impl std::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deadline has elapsed")
    }
}

impl std::error::Error for Elapsed {}

/// Runs a future with a timeout
///
/// # Native (Tokio)
/// Uses `tokio::time::timeout`.
///
/// # WASM
/// Currently not supported, returns the future result immediately.
#[cfg(not(target_family = "wasm"))]
pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, Elapsed>
where
    F: std::future::Future,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| Elapsed)
}

/// WASM version - no timeout support yet
#[cfg(target_family = "wasm")]
pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, Elapsed>
where
    F: std::future::Future,
{
    // WASM doesn't support true timeouts yet
    // For now, just execute the future
    // Note: timeout() on WASM is not yet supported, executing without timeout
    let _ = duration; // Silence unused warning
    Ok(future.await)
}

/// Interval timer
#[cfg(not(target_family = "wasm"))]
pub struct Interval {
    inner: tokio::time::Interval,
}

#[cfg(not(target_family = "wasm"))]
impl Interval {
    /// Waits for the next tick
    pub async fn tick(&mut self) {
        self.inner.tick().await;
    }
}

/// Creates an interval that ticks at a fixed rate
#[cfg(not(target_family = "wasm"))]
pub fn interval(period: Duration) -> Interval {
    Interval {
        inner: tokio::time::interval(period),
    }
}

/// WASM version - limited interval support
#[cfg(target_family = "wasm")]
pub struct Interval {
    period: Duration,
}

#[cfg(target_family = "wasm")]
impl Interval {
    /// Waits for the next tick
    pub async fn tick(&mut self) {
        sleep(self.period).await;
    }
}

#[cfg(target_family = "wasm")]
pub fn interval(period: Duration) -> Interval {
    Interval { period }
}

/// Instant in time
#[cfg(not(target_family = "wasm"))]
pub use tokio::time::Instant;

#[cfg(target_family = "wasm")]
pub use std::time::Instant;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_timeout_success() {
        let result = timeout(Duration::from_secs(1), async { 42 }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_timeout_elapsed() {
        let result = timeout(Duration::from_millis(10), async {
            sleep(Duration::from_secs(1)).await;
            42
        })
        .await;
        assert!(result.is_err());
    }

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_interval() {
        let mut interval = interval(Duration::from_millis(10));
        interval.tick().await;
        interval.tick().await;
    }
}
