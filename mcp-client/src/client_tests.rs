//! Tests for MCP client

use crate::client::McpClient;
use crate::error::ClientError;
use crate::transport::{JsonRpcMessage, StdioClientTransport};
use std::time::Duration;
use tokio::io::{DuplexStream, duplex};

/// Create a mock transport for testing
fn create_mock_transport() -> (
    StdioClientTransport<DuplexStream, DuplexStream>,
    DuplexStream,
    DuplexStream,
) {
    let (client_read, server_write) = duplex(1024);
    let (server_read, client_write) = duplex(1024);

    let transport = StdioClientTransport::new(client_read, client_write);

    (transport, server_read, server_write)
}

#[tokio::test]
async fn test_client_creation() {
    let (transport, _server_read, _server_write) = create_mock_transport();
    let client = McpClient::new(transport);

    assert!(!client.is_initialized());
    assert!(client.server_info().is_none());
}

#[tokio::test]
async fn test_client_not_initialized_error() {
    let (transport, _server_read, _server_write) = create_mock_transport();
    let client = McpClient::new(transport);

    // Trying to list tools without initialization should fail
    let result = client.list_tools().await;
    assert!(matches!(result, Err(ClientError::NotInitialized)));
}

#[tokio::test]
async fn test_client_with_timeout() {
    let (transport, _server_read, _server_write) = create_mock_transport();
    let client = McpClient::new(transport).with_timeout(Duration::from_secs(60));

    // Timeout is set internally, verify client was created
    assert!(!client.is_initialized());
}

#[tokio::test]
async fn test_client_with_client_info() {
    let (transport, _server_read, _server_write) = create_mock_transport();
    let client = McpClient::new(transport).with_client_info("test-client", "1.0.0");

    // Client info is set internally, we verify it works by checking the client was created
    assert!(!client.is_initialized());
}

#[test]
fn test_json_rpc_message_parse_response() {
    let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
    let msg = JsonRpcMessage::parse(json).unwrap();
    assert!(matches!(msg, JsonRpcMessage::Response(_)));

    if let JsonRpcMessage::Response(resp) = msg {
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }
}

#[test]
fn test_json_rpc_message_parse_error_response() {
    let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let msg = JsonRpcMessage::parse(json).unwrap();
    assert!(matches!(msg, JsonRpcMessage::Response(_)));

    if let JsonRpcMessage::Response(resp) = msg {
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
    }
}

#[test]
fn test_json_rpc_message_parse_request() {
    let json = r#"{"jsonrpc":"2.0","method":"sampling/createMessage","params":{},"id":"req-1"}"#;
    let msg = JsonRpcMessage::parse(json).unwrap();
    assert!(matches!(msg, JsonRpcMessage::Request(_)));

    if let JsonRpcMessage::Request(req) = msg {
        assert_eq!(req.method, "sampling/createMessage");
    }
}

#[test]
fn test_json_rpc_message_parse_notification() {
    let json = r#"{"jsonrpc":"2.0","method":"notifications/progress","params":{"progress":50}}"#;
    let msg = JsonRpcMessage::parse(json).unwrap();

    if let JsonRpcMessage::Notification { method, params } = msg {
        assert_eq!(method, "notifications/progress");
        assert_eq!(params["progress"], 50);
    } else {
        panic!("Expected notification");
    }
}

#[test]
fn test_json_rpc_message_parse_invalid() {
    let json = r#"{"jsonrpc":"2.0"}"#;
    let result = JsonRpcMessage::parse(json);
    assert!(result.is_err());
}

#[test]
fn test_client_error_display() {
    let err = ClientError::NotInitialized;
    assert_eq!(
        err.to_string(),
        "Client not initialized - call initialize() first"
    );

    let err = ClientError::Timeout(Duration::from_secs(30));
    assert!(err.to_string().contains("30"));

    let err = ClientError::ServerError {
        code: -32600,
        message: "Invalid Request".to_string(),
        data: None,
    };
    assert!(err.to_string().contains("-32600"));
    assert!(err.to_string().contains("Invalid Request"));
}

#[test]
fn test_client_error_is_retryable() {
    assert!(ClientError::Timeout(Duration::from_secs(1)).is_retryable());
    assert!(ClientError::Transport("connection lost".to_string()).is_retryable());
    assert!(!ClientError::NotInitialized.is_retryable());
    assert!(!ClientError::Protocol("invalid".to_string()).is_retryable());
}
