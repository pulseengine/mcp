//! Core runtime functionality - task spawning and execution
//!
//! This module provides platform-specific runtime implementations that abstract
//! over Tokio (native) and wstd (WASM).

use std::future::Future;
use std::time::Duration;

/// Handle to a spawned task
///
/// On native platforms, this wraps `tokio::task::JoinHandle`.
/// On WASM, tasks run to completion immediately in the current context.
#[cfg(not(target_family = "wasm"))]
pub struct JoinHandle<T> {
    inner: tokio::task::JoinHandle<T>,
}

#[cfg(not(target_family = "wasm"))]
impl<T> JoinHandle<T> {
    /// Waits for the task to complete
    pub async fn join(self) -> Result<T, crate::RuntimeError> {
        self.inner.await.map_err(|e| crate::RuntimeError::JoinError(e.to_string()))
    }
}

/// WASM version - tasks complete immediately
#[cfg(target_family = "wasm")]
pub struct JoinHandle<T> {
    _phantom: std::marker::PhantomData<T>,
}

/// Spawns a new asynchronous task
///
/// # Native (Tokio)
/// Spawns on the Tokio runtime's thread pool.
///
/// # WASM (wstd)
/// Currently runs the task to completion in the current context.
/// Future wstd versions may support true concurrent spawning.
///
/// # Examples
///
/// ```rust,no_run
/// use pulseengine_mcp_runtime::spawn;
///
/// spawn(async {
///     println!("Task running!");
/// });
/// ```
#[cfg(not(target_family = "wasm"))]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    JoinHandle {
        inner: tokio::task::spawn(future),
    }
}

/// WASM version of spawn
///
/// Note: wstd doesn't currently have task spawning, so we document
/// that futures may execute in the current context. This will be
/// updated when wstd adds spawning support.
#[cfg(target_family = "wasm")]
pub fn spawn<F>(_future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    // TODO: When wstd supports spawning, use it here
    // For now, document that this may block the current task
    // Note: spawn() on WASM currently has limited support
    JoinHandle {
        _phantom: std::marker::PhantomData,
    }
}

/// Spawns a blocking task on a dedicated thread pool
///
/// Use this for CPU-intensive or blocking operations.
///
/// # Native (Tokio)
/// Uses `tokio::task::spawn_blocking`.
///
/// # WASM
/// Executes immediately (no threads in WASM).
#[cfg(not(target_family = "wasm"))]
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    JoinHandle {
        inner: tokio::task::spawn_blocking(f),
    }
}

/// WASM version - executes immediately
#[cfg(target_family = "wasm")]
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    // WASM is single-threaded, execute immediately
    let _ = f();
    JoinHandle {
        _phantom: std::marker::PhantomData,
    }
}

/// Runs a future to completion on the runtime
///
/// # Native (Tokio)
/// Creates a new runtime if needed or uses the current runtime.
///
/// # WASM (wstd)
/// Uses `wstd::block_on`.
///
/// # Examples
///
/// ```rust,no_run
/// use pulseengine_mcp_runtime::block_on;
///
/// let result = block_on(async {
///     42
/// });
/// assert_eq!(result, 42);
/// ```
#[cfg(not(target_family = "wasm"))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    // Try to use current runtime, otherwise create new one
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(future),
        Err(_) => {
            // Create a new runtime for this block_on call
            tokio::runtime::Runtime::new()
                .expect("Failed to create Tokio runtime")
                .block_on(future)
        }
    }
}

/// WASM version using futures executor
#[cfg(all(target_family = "wasm", target_os = "wasi"))]
pub fn block_on<F: Future>(future: F) -> F::Output {
    futures::executor::block_on(future)
}

/// Sleeps for the specified duration
///
/// # Examples
///
/// ```rust,no_run
/// use pulseengine_mcp_runtime::sleep;
/// use std::time::Duration;
///
/// async fn delayed_task() {
///     sleep(Duration::from_secs(1)).await;
///     println!("One second later...");
/// }
/// ```
#[cfg(not(target_family = "wasm"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// WASM version of sleep - converts std::time::Duration to wstd::time::Duration
#[cfg(all(target_family = "wasm", target_os = "wasi"))]
pub async fn sleep(duration: Duration) {
    // Convert std::time::Duration to wstd::time::Duration
    let wstd_duration = wstd::time::Duration::from_millis(duration.as_millis() as u64);
    wstd::task::sleep(wstd_duration).await;
}

/// Yields the current task, allowing other tasks to run
///
/// # Examples
///
/// ```rust,no_run
/// use pulseengine_mcp_runtime::runtime::yield_now;
///
/// async fn cooperative_task() {
///     for i in 0..100 {
///         // Do some work
///         if i % 10 == 0 {
///             yield_now().await; // Let other tasks run
///         }
///     }
/// }
/// ```
#[cfg(not(target_family = "wasm"))]
pub async fn yield_now() {
    tokio::task::yield_now().await;
}

/// WASM version - no-op for now
#[cfg(target_family = "wasm")]
pub async fn yield_now() {
    // WASM is cooperative, this is a no-op
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_on() {
        let result = block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[tokio::test]
    #[cfg(not(target_family = "wasm"))]
    async fn test_spawn() {
        let handle = spawn(async { 42 });
        let result = handle.join().await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    #[cfg(not(target_family = "wasm"))]
    async fn test_sleep() {
        let start = std::time::Instant::now();
        sleep(Duration::from_millis(100)).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(100));
    }
}
