//! Tests for MCP client transport

use super::transport::*;
use pulseengine_mcp_protocol::{NumberOrString, Request};
use serde_json::json;
use tokio::io::{AsyncWriteExt, duplex};

#[tokio::test]
async fn test_stdio_transport_send() {
    let (client_read, _server_write) = duplex(1024);
    let (server_read, client_write) = duplex(1024);

    let transport = StdioClientTransport::new(client_read, client_write);

    let request = Request {
        jsonrpc: "2.0".to_string(),
        method: "test".to_string(),
        params: json!({}),
        id: Some(NumberOrString::Number(1)),
    };

    // Send request
    transport.send(&request).await.unwrap();

    // Read from "server" side
    let mut reader = tokio::io::BufReader::new(server_read);
    use tokio::io::AsyncBufReadExt;
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    // Verify the message
    assert!(line.contains("\"method\":\"test\""));
    assert!(line.contains("\"id\":1"));
}

#[tokio::test]
async fn test_stdio_transport_recv_response() {
    let (client_read, mut server_write) = duplex(1024);
    let (_server_read, client_write) = duplex(1024);

    let transport = StdioClientTransport::new(client_read, client_write);

    // Server sends a response
    let response_json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
    server_write
        .write_all(format!("{response_json}\n").as_bytes())
        .await
        .unwrap();
    server_write.flush().await.unwrap();

    // Client receives it
    let msg = transport.recv().await.unwrap();

    match msg {
        JsonRpcMessage::Response(resp) => {
            assert_eq!(resp.id, Some(NumberOrString::Number(1)));
            assert!(resp.result.is_some());
        }
        _ => panic!("Expected Response"),
    }
}

#[tokio::test]
async fn test_stdio_transport_recv_notification() {
    let (client_read, mut server_write) = duplex(1024);
    let (_server_read, client_write) = duplex(1024);

    let transport = StdioClientTransport::new(client_read, client_write);

    // Server sends a notification
    let notification_json =
        r#"{"jsonrpc":"2.0","method":"notifications/progress","params":{"progress":50}}"#;
    server_write
        .write_all(format!("{notification_json}\n").as_bytes())
        .await
        .unwrap();
    server_write.flush().await.unwrap();

    // Client receives it
    let msg = transport.recv().await.unwrap();

    match msg {
        JsonRpcMessage::Notification { method, params } => {
            assert_eq!(method, "notifications/progress");
            assert_eq!(params["progress"], 50);
        }
        _ => panic!("Expected Notification"),
    }
}

#[tokio::test]
async fn test_stdio_transport_recv_request() {
    let (client_read, mut server_write) = duplex(1024);
    let (_server_read, client_write) = duplex(1024);

    let transport = StdioClientTransport::new(client_read, client_write);

    // Server sends a request (e.g., sampling/createMessage)
    let request_json =
        r#"{"jsonrpc":"2.0","method":"sampling/createMessage","params":{},"id":"srv-1"}"#;
    server_write
        .write_all(format!("{request_json}\n").as_bytes())
        .await
        .unwrap();
    server_write.flush().await.unwrap();

    // Client receives it
    let msg = transport.recv().await.unwrap();

    match msg {
        JsonRpcMessage::Request(req) => {
            assert_eq!(req.method, "sampling/createMessage");
            assert!(req.id.is_some());
        }
        _ => panic!("Expected Request"),
    }
}

#[tokio::test]
async fn test_stdio_transport_close() {
    let (client_read, _server_write) = duplex(1024);
    let (_server_read, client_write) = duplex(1024);

    let transport = StdioClientTransport::new(client_read, client_write);

    // Close should succeed
    transport.close().await.unwrap();
}

#[tokio::test]
async fn test_stdio_transport_skip_empty_lines() {
    let (client_read, mut server_write) = duplex(1024);
    let (_server_read, client_write) = duplex(1024);

    let transport = StdioClientTransport::new(client_read, client_write);

    // Server sends empty lines then a response
    server_write.write_all(b"\n\n").await.unwrap();
    let response_json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
    server_write
        .write_all(format!("{response_json}\n").as_bytes())
        .await
        .unwrap();
    server_write.flush().await.unwrap();

    // Client should skip empty lines and receive the response
    let msg = transport.recv().await.unwrap();
    assert!(matches!(msg, JsonRpcMessage::Response(_)));
}

#[tokio::test]
async fn test_request_id_generation() {
    let id1 = next_request_id();
    let id2 = next_request_id();
    let id3 = next_request_id();

    // All IDs should be different
    let ids: Vec<String> = vec![id1, id2, id3]
        .into_iter()
        .map(|id| match id {
            NumberOrString::Number(n) => n.to_string(),
            NumberOrString::String(s) => s.to_string(),
        })
        .collect();

    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);
    assert_ne!(ids[0], ids[2]);
}

#[test]
fn test_json_rpc_message_parse_variants() {
    // Test all message types

    // Response with result
    let msg = JsonRpcMessage::parse(r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#).unwrap();
    assert!(matches!(msg, JsonRpcMessage::Response(_)));

    // Response with error (use a valid error code)
    let msg = JsonRpcMessage::parse(
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"err"}}"#,
    )
    .unwrap();
    assert!(matches!(msg, JsonRpcMessage::Response(_)));

    // Request (has method and id)
    let msg =
        JsonRpcMessage::parse(r#"{"jsonrpc":"2.0","method":"test","params":{},"id":"x"}"#).unwrap();
    assert!(matches!(msg, JsonRpcMessage::Request(_)));

    // Notification (has method but no id)
    let msg = JsonRpcMessage::parse(r#"{"jsonrpc":"2.0","method":"notify","params":{}}"#).unwrap();
    assert!(matches!(msg, JsonRpcMessage::Notification { .. }));

    // Notification with null id (treated as notification)
    let msg = JsonRpcMessage::parse(r#"{"jsonrpc":"2.0","method":"notify","params":{},"id":null}"#)
        .unwrap();
    assert!(matches!(msg, JsonRpcMessage::Notification { .. }));
}

#[test]
fn test_json_rpc_message_parse_errors() {
    // Invalid JSON
    assert!(JsonRpcMessage::parse("not json").is_err());

    // Missing required fields
    assert!(JsonRpcMessage::parse(r#"{"jsonrpc":"2.0"}"#).is_err());

    // Empty object
    assert!(JsonRpcMessage::parse(r#"{}"#).is_err());
}
