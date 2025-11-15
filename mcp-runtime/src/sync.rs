//! Synchronization primitives
//!
//! This module provides async-aware synchronization primitives that work
//! across native and WASM platforms.

// For native, re-export tokio's sync primitives
#[cfg(not(target_family = "wasm"))]
pub use tokio::sync::{Mutex, RwLock, MutexGuard, RwLockReadGuard, RwLockWriteGuard};

// For WASM, we need simpler implementations since there's no true concurrency
#[cfg(target_family = "wasm")]
pub use std::sync::{Mutex, RwLock, MutexGuard, RwLockReadGuard, RwLockWriteGuard};

/// One-shot channel for sending a single value
#[cfg(not(target_family = "wasm"))]
pub mod oneshot {
    pub use tokio::sync::oneshot::{channel, Sender, Receiver, error};
}

/// WASM version of oneshot - uses standard library
#[cfg(target_family = "wasm")]
pub mod oneshot {
    use std::sync::{Arc, Mutex};

    /// Error returned when receiver is dropped
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RecvError;

    impl std::fmt::Display for RecvError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "oneshot receiver error")
        }
    }

    impl std::error::Error for RecvError {}

    /// Sender half of oneshot channel
    pub struct Sender<T> {
        inner: Arc<Mutex<Option<T>>>,
    }

    /// Receiver half of oneshot channel
    pub struct Receiver<T> {
        inner: Arc<Mutex<Option<T>>>,
    }

    impl<T> Sender<T> {
        /// Sends a value
        pub fn send(self, value: T) -> Result<(), T> {
            let mut guard = self.inner.lock().unwrap();
            *guard = Some(value);
            Ok(())
        }
    }

    impl<T> Receiver<T> {
        /// Receives a value (blocking in WASM context)
        pub async fn recv(self) -> Result<T, RecvError> {
            let mut guard = self.inner.lock().unwrap();
            guard.take().ok_or(RecvError)
        }
    }

    /// Creates a oneshot channel
    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(Mutex::new(None));
        (
            Sender { inner: inner.clone() },
            Receiver { inner },
        )
    }

    /// Error types
    pub mod error {
        pub use super::RecvError;
    }
}

/// Multi-producer, multi-consumer channel
#[cfg(not(target_family = "wasm"))]
pub mod mpsc {
    pub use tokio::sync::mpsc::{
        channel, unbounded_channel, Sender, Receiver, UnboundedSender, UnboundedReceiver, error,
    };
}

/// WASM version - limited mpsc support
#[cfg(target_family = "wasm")]
pub mod mpsc {
    use std::sync::{Arc, Mutex};
    use std::collections::VecDeque;

    /// Error when sending
    #[derive(Debug)]
    pub struct SendError<T>(pub T);

    impl<T> std::fmt::Display for SendError<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "send error")
        }
    }

    impl<T: std::fmt::Debug> std::error::Error for SendError<T> {}

    /// Sender half of mpsc channel
    pub struct UnboundedSender<T> {
        inner: Arc<Mutex<VecDeque<T>>>,
    }

    impl<T> UnboundedSender<T> {
        /// Sends a value
        pub fn send(&self, value: T) -> Result<(), SendError<T>> {
            let mut guard = self.inner.lock().unwrap();
            guard.push_back(value);
            Ok(())
        }
    }

    impl<T> Clone for UnboundedSender<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    /// Receiver half of mpsc channel
    pub struct UnboundedReceiver<T> {
        inner: Arc<Mutex<VecDeque<T>>>,
    }

    impl<T> UnboundedReceiver<T> {
        /// Receives a value
        pub async fn recv(&mut self) -> Option<T> {
            let mut guard = self.inner.lock().unwrap();
            guard.pop_front()
        }
    }

    /// Creates an unbounded mpsc channel
    pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
        let inner = Arc::new(Mutex::new(VecDeque::new()));
        (
            UnboundedSender { inner: inner.clone() },
            UnboundedReceiver { inner },
        )
    }

    /// Error types
    pub mod error {
        pub use super::SendError;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_mutex() {
        let mutex = Mutex::new(42);
        let guard = mutex.lock().await;
        assert_eq!(*guard, 42);
    }

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_oneshot() {
        let (tx, rx) = oneshot::channel();
        tx.send(42).unwrap();
        assert_eq!(rx.await.unwrap(), 42);
    }

    #[cfg(not(target_family = "wasm"))]
    #[tokio::test]
    async fn test_mpsc() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(42).unwrap();
        assert_eq!(rx.recv().await.unwrap(), 42);
    }
}
