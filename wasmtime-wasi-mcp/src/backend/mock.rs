//! Mock backend for testing

use super::Backend;
use crate::error::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Mock backend for testing
///
/// Stores messages in memory queues instead of using actual I/O.
/// Useful for unit tests and integration tests.
#[derive(Debug, Clone)]
pub struct MockBackend {
    /// Messages to be read
    read_queue: Arc<Mutex<VecDeque<Value>>>,
    /// Messages that were written
    write_queue: Arc<Mutex<Vec<Value>>>,
    /// Whether the backend is active
    active: Arc<Mutex<bool>>,
}

impl MockBackend {
    /// Create a new mock backend
    pub fn new() -> Self {
        Self {
            read_queue: Arc::new(Mutex::new(VecDeque::new())),
            write_queue: Arc::new(Mutex::new(Vec::new())),
            active: Arc::new(Mutex::new(true)),
        }
    }

    /// Add a message to the read queue
    pub fn push_message(&self, message: Value) {
        self.read_queue.lock().unwrap().push_back(message);
    }

    /// Get all written messages
    pub fn get_written_messages(&self) -> Vec<Value> {
        self.write_queue.lock().unwrap().clone()
    }

    /// Get the last written message
    pub fn get_last_written(&self) -> Option<Value> {
        self.write_queue.lock().unwrap().last().cloned()
    }

    /// Clear all queues
    pub fn clear(&self) {
        self.read_queue.lock().unwrap().clear();
        self.write_queue.lock().unwrap().clear();
    }

    /// Get number of messages in read queue
    pub fn read_queue_len(&self) -> usize {
        self.read_queue.lock().unwrap().len()
    }

    /// Get number of written messages
    pub fn write_queue_len(&self) -> usize {
        self.write_queue.lock().unwrap().len()
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Backend for MockBackend {
    async fn read_message(&mut self) -> Result<Value> {
        let message = self.read_queue.lock().unwrap().pop_front()
            .ok_or_else(|| crate::Error::internal("No messages in read queue"))?;
        Ok(message)
    }

    async fn write_message(&mut self, message: &Value) -> Result<()> {
        self.write_queue.lock().unwrap().push(message.clone());
        Ok(())
    }

    fn is_active(&self) -> bool {
        *self.active.lock().unwrap()
    }

    async fn shutdown(&mut self) -> Result<()> {
        *self.active.lock().unwrap() = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mock_backend_creation() {
        let backend = MockBackend::new();
        assert!(backend.is_active());
        assert_eq!(backend.read_queue_len(), 0);
        assert_eq!(backend.write_queue_len(), 0);
    }

    #[test]
    fn test_mock_backend_push_message() {
        let backend = MockBackend::new();
        backend.push_message(json!({"test": "message"}));
        assert_eq!(backend.read_queue_len(), 1);
    }

    #[tokio::test]
    async fn test_mock_backend_read_message() {
        let mut backend = MockBackend::new();
        let test_msg = json!({"method": "test"});
        backend.push_message(test_msg.clone());

        let read_msg = backend.read_message().await.unwrap();
        assert_eq!(read_msg, test_msg);
        assert_eq!(backend.read_queue_len(), 0);
    }

    #[tokio::test]
    async fn test_mock_backend_write_message() {
        let mut backend = MockBackend::new();
        let test_msg = json!({"result": "ok"});

        backend.write_message(&test_msg).await.unwrap();
        assert_eq!(backend.write_queue_len(), 1);

        let written = backend.get_last_written().unwrap();
        assert_eq!(written, test_msg);
    }

    #[tokio::test]
    async fn test_mock_backend_read_empty_queue() {
        let mut backend = MockBackend::new();
        let result = backend.read_message().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_backend_shutdown() {
        let mut backend = MockBackend::new();
        assert!(backend.is_active());

        backend.shutdown().await.unwrap();
        assert!(!backend.is_active());
    }

    #[test]
    fn test_mock_backend_clear() {
        let backend = MockBackend::new();
        backend.push_message(json!({"test": 1}));
        backend.push_message(json!({"test": 2}));

        assert_eq!(backend.read_queue_len(), 2);

        backend.clear();
        assert_eq!(backend.read_queue_len(), 0);
        assert_eq!(backend.write_queue_len(), 0);
    }

    #[test]
    fn test_mock_backend_multiple_messages() {
        let backend = MockBackend::new();
        backend.push_message(json!({"id": 1}));
        backend.push_message(json!({"id": 2}));
        backend.push_message(json!({"id": 3}));

        assert_eq!(backend.read_queue_len(), 3);
    }

    #[tokio::test]
    async fn test_mock_backend_get_written_messages() {
        let mut backend = MockBackend::new();
        backend.write_message(&json!({"msg": 1})).await.unwrap();
        backend.write_message(&json!({"msg": 2})).await.unwrap();

        let messages = backend.get_written_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], json!({"msg": 1}));
        assert_eq!(messages[1], json!({"msg": 2}));
    }
}
