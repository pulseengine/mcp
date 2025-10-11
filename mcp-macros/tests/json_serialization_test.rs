//! Tests for JSON serialization of tool return values
//!
//! This test suite verifies that the #[mcp_tools] macro correctly serializes
//! structured return types as JSON instead of using Rust's Debug format.
//!
//! Addresses: https://github.com/pulseengine/mcp/issues/62

#![allow(dead_code)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_protocol::{CallToolRequestParam, Content};
use pulseengine_mcp_server::McpServerBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Complex structured type for testing JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SearchResult {
    pub total_count: u32,
    pub items: Vec<SearchItem>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SearchItem {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// Nested structured type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct IssueSearchResult {
    pub search_result: SearchResult,
    pub query: String,
    pub duration_ms: u64,
}

/// Server with tools returning structured data
#[mcp_server(name = "JSON Serialization Test Server")]
#[derive(Clone, Default)]
struct JsonTestServer;

#[mcp_tools]
impl JsonTestServer {
    /// Tool returning a simple structured type
    pub fn get_search_result(&self) -> SearchResult {
        SearchResult {
            total_count: 2,
            items: vec![
                SearchItem {
                    id: 1,
                    title: "First Item".to_string(),
                    description: Some("Description of first item".to_string()),
                    tags: vec!["tag1".to_string(), "tag2".to_string()],
                },
                SearchItem {
                    id: 2,
                    title: "Second Item".to_string(),
                    description: None,
                    tags: vec!["tag3".to_string()],
                },
            ],
            has_more: true,
        }
    }

    /// Tool returning a nested structured type
    pub fn search_issues(&self, query: String) -> IssueSearchResult {
        IssueSearchResult {
            search_result: SearchResult {
                total_count: 1,
                items: vec![SearchItem {
                    id: 42,
                    title: format!("Issue matching '{query}'"),
                    description: Some("This is a test issue".to_string()),
                    tags: vec!["bug".to_string(), "high-priority".to_string()],
                }],
                has_more: false,
            },
            query: query.clone(),
            duration_ms: 150,
        }
    }

    /// Tool returning Result<StructuredType, Error>
    pub fn get_result_type(&self, should_succeed: bool) -> Result<SearchResult, String> {
        if should_succeed {
            Ok(SearchResult {
                total_count: 1,
                items: vec![SearchItem {
                    id: 999,
                    title: "Result type item".to_string(),
                    description: Some("From Result<T, E>".to_string()),
                    tags: vec![],
                }],
                has_more: false,
            })
        } else {
            Err("Intentional failure".to_string())
        }
    }

    /// Tool returning a simple string (should not break)
    pub fn simple_string(&self) -> String {
        "Simple text response".to_string()
    }

    /// Tool returning a number (should serialize as JSON number)
    pub fn simple_number(&self) -> i32 {
        42
    }

    /// Tool returning a bool
    pub fn simple_bool(&self) -> bool {
        true
    }

    /// Tool returning a vector of strings
    pub fn string_vector(&self) -> Vec<String> {
        vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ]
    }
}

#[tokio::test]
async fn test_structured_return_serialization() {
    let server = JsonTestServer::with_defaults();

    // Call tool that returns structured data
    let request = CallToolRequestParam {
        name: "get_search_result".to_string(),
        arguments: None,
    };

    let result = server
        .call_tool(request)
        .await
        .expect("Tool should succeed");

    // Verify the result
    assert_eq!(result.is_error, Some(false));
    assert_eq!(result.content.len(), 1);

    // Extract the text content
    let text_content = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };

    // Verify it's valid JSON (not Debug format)
    let parsed: SearchResult =
        serde_json::from_str(text_content).expect("Content should be valid JSON");

    // Verify the structure
    assert_eq!(parsed.total_count, 2);
    assert_eq!(parsed.items.len(), 2);
    assert_eq!(parsed.items[0].id, 1);
    assert_eq!(parsed.items[0].title, "First Item");
    assert_eq!(
        parsed.items[0].description,
        Some("Description of first item".to_string())
    );
    assert_eq!(parsed.items[0].tags, vec!["tag1", "tag2"]);
    assert!(parsed.has_more);

    // Verify structured_content is also populated (MCP 2025 spec)
    assert!(
        result.structured_content.is_some(),
        "structured_content should be populated for structured types"
    );

    let structured = result.structured_content.unwrap();
    assert_eq!(structured["total_count"], 2);
    assert_eq!(structured["items"][0]["id"], 1);
}

#[tokio::test]
async fn test_nested_structured_return() {
    let server = JsonTestServer::with_defaults();

    let request = CallToolRequestParam {
        name: "search_issues".to_string(),
        arguments: Some(json!({ "query": "test query" })),
    };

    let result = server
        .call_tool(request)
        .await
        .expect("Tool should succeed");

    // Extract text content
    let text_content = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };

    // Verify it's valid JSON
    let parsed: IssueSearchResult =
        serde_json::from_str(text_content).expect("Content should be valid JSON");

    // Verify nested structure
    assert_eq!(parsed.query, "test query");
    assert_eq!(parsed.duration_ms, 150);
    assert_eq!(parsed.search_result.total_count, 1);
    assert_eq!(parsed.search_result.items[0].id, 42);
    assert_eq!(
        parsed.search_result.items[0].title,
        "Issue matching 'test query'"
    );

    // Verify it doesn't contain Debug format markers
    assert!(
        !text_content.contains("SearchResult {"),
        "Should not contain Debug format: {text_content}"
    );
    assert!(
        !text_content.contains("IssueSearchResult {"),
        "Should not contain Debug format: {text_content}"
    );
}

#[tokio::test]
async fn test_result_type_success_serialization() {
    let server = JsonTestServer::with_defaults();

    let request = CallToolRequestParam {
        name: "get_result_type".to_string(),
        arguments: Some(json!({ "should_succeed": true })),
    };

    let result = server
        .call_tool(request)
        .await
        .expect("Tool should succeed");

    // Extract text content
    let text_content = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };

    // Verify it's valid JSON
    let parsed: SearchResult =
        serde_json::from_str(text_content).expect("Content should be valid JSON");

    assert_eq!(parsed.total_count, 1);
    assert_eq!(parsed.items[0].id, 999);
    assert_eq!(parsed.items[0].title, "Result type item");

    // Verify structured_content
    assert!(result.structured_content.is_some());
}

#[tokio::test]
async fn test_result_type_error() {
    let server = JsonTestServer::with_defaults();

    let request = CallToolRequestParam {
        name: "get_result_type".to_string(),
        arguments: Some(json!({ "should_succeed": false })),
    };

    let result = server.call_tool(request).await;

    // Should return an error
    assert!(result.is_err(), "Should fail when should_succeed is false");

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Intentional failure"));
}

#[tokio::test]
async fn test_simple_types_serialization() {
    let server = JsonTestServer::with_defaults();

    // Test string
    let request = CallToolRequestParam {
        name: "simple_string".to_string(),
        arguments: None,
    };
    let result = server.call_tool(request).await.expect("Should succeed");
    let text = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };
    // String should be JSON-encoded (quoted)
    let parsed: String = serde_json::from_str(text).expect("Should be valid JSON");
    assert_eq!(parsed, "Simple text response");

    // Test number
    let request = CallToolRequestParam {
        name: "simple_number".to_string(),
        arguments: None,
    };
    let result = server.call_tool(request).await.expect("Should succeed");
    let text = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };
    let parsed: i32 = serde_json::from_str(text).expect("Should be valid JSON");
    assert_eq!(parsed, 42);

    // Test bool
    let request = CallToolRequestParam {
        name: "simple_bool".to_string(),
        arguments: None,
    };
    let result = server.call_tool(request).await.expect("Should succeed");
    let text = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };
    let parsed: bool = serde_json::from_str(text).expect("Should be valid JSON");
    assert!(parsed);
}

#[tokio::test]
async fn test_vector_serialization() {
    let server = JsonTestServer::with_defaults();

    let request = CallToolRequestParam {
        name: "string_vector".to_string(),
        arguments: None,
    };

    let result = server.call_tool(request).await.expect("Should succeed");

    let text = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };

    // Should be JSON array
    let parsed: Vec<String> = serde_json::from_str(text).expect("Should be valid JSON array");
    assert_eq!(parsed, vec!["item1", "item2", "item3"]);

    // Should not contain Debug format
    assert!(!text.contains("vec!["), "Should not contain Debug format");
}

#[tokio::test]
async fn test_no_debug_format_in_output() {
    let server = JsonTestServer::with_defaults();

    // Test all tools and verify none contain Debug format markers
    let tools_to_test = vec![
        ("get_search_result", None),
        ("search_issues", Some(json!({ "query": "test" }))),
        ("get_result_type", Some(json!({ "should_succeed": true }))),
    ];

    for (tool_name, args) in tools_to_test {
        let request = CallToolRequestParam {
            name: tool_name.to_string(),
            arguments: args,
        };

        let result = server
            .call_tool(request)
            .await
            .expect("Tool should succeed");

        let text = match &result.content[0] {
            Content::Text { text, .. } => text,
            _ => panic!("Expected text content"),
        };

        // Verify no Debug format markers
        let debug_markers = vec![
            "SearchResult {",
            "SearchItem {",
            "IssueSearchResult {",
            ": String",
            ": u32",
            ": u64",
            ": Vec<",
        ];

        for marker in debug_markers {
            assert!(
                !text.contains(marker),
                "Tool '{tool_name}' output contains Debug format marker '{marker}': {text}"
            );
        }

        // Verify it's valid JSON
        serde_json::from_str::<serde_json::Value>(text)
            .unwrap_or_else(|_| panic!("Tool '{tool_name}' should return valid JSON"));
    }
}

#[tokio::test]
async fn test_structured_content_field_populated() {
    let server = JsonTestServer::with_defaults();

    let request = CallToolRequestParam {
        name: "get_search_result".to_string(),
        arguments: None,
    };

    let result = server
        .call_tool(request)
        .await
        .expect("Tool should succeed");

    // MCP 2025-06-18 spec: structured results should populate structured_content
    assert!(
        result.structured_content.is_some(),
        "structured_content should be populated for structured types"
    );

    let structured = result.structured_content.unwrap();

    // Verify structure matches the return type
    assert_eq!(structured["total_count"], 2);
    assert!(structured["items"].is_array());
    assert_eq!(structured["items"].as_array().unwrap().len(), 2);
    assert_eq!(structured["has_more"], true);

    // Text content should also be valid JSON
    let text = match &result.content[0] {
        Content::Text { text, .. } => text,
        _ => panic!("Expected text content"),
    };

    let text_parsed: serde_json::Value =
        serde_json::from_str(text).expect("Text content should be valid JSON");

    // structured_content and text content should represent the same data
    assert_eq!(structured, text_parsed);
}
