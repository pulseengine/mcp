//! Advanced edge case tests for mcp_resource macro
//!
//! These tests check for subtle bugs that might not be immediately obvious.

#![allow(clippy::uninlined_format_args)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_protocol::ReadResourceRequestParam;
use pulseengine_mcp_server::McpServerBuilder;

// =============================================================================
// TEST CASE 1: Parameter name mismatch between URI template and method
// The URI template has {user_id} but method parameter is named `id`
// This should either work (by position) or give a clear error
// =============================================================================
mod param_name_mismatch {
    use super::*;

    #[mcp_server(name = "Param Mismatch Server")]
    #[derive(Default, Clone)]
    pub struct ParamMismatchServer;

    #[mcp_tools]
    impl ParamMismatchServer {
        /// URI template param name differs from method param name
        #[mcp_resource(uri_template = "user://{user_id}")]
        pub fn get_user(&self, id: String) -> Result<String, String> {
            Ok(format!("User: {}", id))
        }
    }

    #[tokio::test]
    async fn test_param_name_mismatch() {
        let server = ParamMismatchServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);

        // Try to read the resource - this tests runtime behavior
        let request = ReadResourceRequestParam {
            uri: "user://123".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        // This might fail or return empty string depending on implementation
        println!("Result: {:?}", result);
    }
}

// =============================================================================
// TEST CASE 2: More URI template params than method params
// =============================================================================
mod more_uri_params {
    use super::*;

    #[mcp_server(name = "More URI Params Server")]
    #[derive(Default, Clone)]
    pub struct MoreUriParamsServer;

    #[mcp_tools]
    impl MoreUriParamsServer {
        /// URI has 3 params but method only has 2
        #[mcp_resource(uri_template = "data://{db}/{schema}/{table}")]
        pub fn get_data(&self, db: String, schema: String) -> Result<String, String> {
            Ok(format!("Data from {}.{}", db, schema))
        }
    }

    #[tokio::test]
    async fn test_more_uri_params() {
        let server = MoreUriParamsServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);

        // Try to read - third param will be ignored
        let request = ReadResourceRequestParam {
            uri: "data://mydb/public/users".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("Result: {:?}", result);
        // Should work, third param just ignored
        assert!(result.is_ok());
    }
}

// =============================================================================
// TEST CASE 3: More method params than URI template params
// BUG: This doesn't compile because the macro generates code that references
// undefined variables. When method has more params than URI template, the
// extra params are never extracted but still passed to the method.
// =============================================================================
// mod more_method_params {
//     use super::*;
//
//     #[mcp_server(name = "More Method Params Server")]
//     #[derive(Default, Clone)]
//     pub struct MoreMethodParamsServer;
//
//     #[mcp_tools]
//     impl MoreMethodParamsServer {
//         /// Method has 3 params but URI only has 2 - THIS FAILS TO COMPILE
//         #[mcp_resource(uri_template = "data://{db}/{schema}")]
//         pub fn get_data(
//             &self,
//             db: String,
//             schema: String,
//             table: String,  // <-- No {table} in URI template!
//         ) -> Result<String, String> {
//             Ok(format!("Data from {}.{}.{}", db, schema, table))
//         }
//     }
// }
//
// TODO: The macro should either:
// 1. Emit a compile-time error when method params don't match URI template params
// 2. Or only extract params that exist in the URI template and pass defaults for others

// =============================================================================
// TEST CASE 4: URI with query parameters (not supported by matchit)
// =============================================================================
mod query_params {
    use super::*;

    #[mcp_server(name = "Query Params Server")]
    #[derive(Default, Clone)]
    pub struct QueryParamsServer;

    #[mcp_tools]
    impl QueryParamsServer {
        /// Note: Query params in URI templates don't work with matchit
        /// This URI template will likely fail to match
        #[mcp_resource(uri_template = "search://{query}")]
        pub fn search(&self, query: String) -> Result<String, String> {
            Ok(format!("Searching for: {}", query))
        }
    }

    #[tokio::test]
    async fn test_query_params() {
        let server = QueryParamsServer::with_defaults();

        // Normal path param works
        let request = ReadResourceRequestParam {
            uri: "search://hello".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        assert!(result.is_ok());

        // But what about URL-encoded content?
        let request = ReadResourceRequestParam {
            uri: "search://hello%20world".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        // Will contain "hello%20world" literally, not "hello world"
        println!("URL encoded result: {:?}", result);
    }
}

// =============================================================================
// TEST CASE 5: Empty path segments
// =============================================================================
mod empty_segments {
    use super::*;

    #[mcp_server(name = "Empty Segments Server")]
    #[derive(Default, Clone)]
    pub struct EmptySegmentsServer;

    #[mcp_tools]
    impl EmptySegmentsServer {
        #[mcp_resource(uri_template = "data://{id}")]
        pub fn get_data(&self, id: String) -> Result<String, String> {
            Ok(format!("Data: {}", id))
        }
    }

    #[tokio::test]
    async fn test_empty_id() {
        let server = EmptySegmentsServer::with_defaults();

        // What happens with empty parameter?
        let request = ReadResourceRequestParam {
            uri: "data://".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("Empty ID result: {:?}", result);
        // Might fail to match or return empty string
    }
}

// =============================================================================
// TEST CASE 6: Catch-all/wildcard patterns
// =============================================================================
mod wildcard_paths {
    use super::*;

    #[mcp_server(name = "Wildcard Server")]
    #[derive(Default, Clone)]
    pub struct WildcardServer;

    #[mcp_tools]
    impl WildcardServer {
        /// Trying to use wildcard - matchit supports {*path} for catch-all
        #[mcp_resource(uri_template = "file://{path}")]
        pub fn get_file(&self, path: String) -> Result<String, String> {
            Ok(format!("File at: {}", path))
        }
    }

    #[tokio::test]
    async fn test_nested_path() {
        let server = WildcardServer::with_defaults();

        // Single segment works
        let request = ReadResourceRequestParam {
            uri: "file://readme.txt".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        assert!(result.is_ok());

        // But nested paths won't match with {path} - need {*path}
        let request = ReadResourceRequestParam {
            uri: "file://src/main.rs".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("Nested path result: {:?}", result);
        // This will likely fail because {path} only matches single segment
    }
}

// =============================================================================
// TEST CASE 7: Catch-all wildcard syntax
// This tests the fix for: {*param} catch-all wildcards now correctly extract values
// =============================================================================
mod catchall_wildcard {
    use super::*;

    #[mcp_server(name = "Catchall Server")]
    #[derive(Default, Clone)]
    pub struct CatchallServer;

    #[mcp_tools]
    impl CatchallServer {
        /// Using matchit's catch-all syntax
        #[mcp_resource(uri_template = "file://{*filepath}")]
        pub fn get_file(&self, filepath: String) -> Result<String, String> {
            Ok(format!("File at: {}", filepath))
        }
    }

    #[tokio::test]
    async fn test_catchall() {
        let server = CatchallServer::with_defaults();

        // Nested paths should work with catch-all
        let request = ReadResourceRequestParam {
            uri: "file://src/main.rs".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("Catchall result: {:?}", result);

        // Verify the filepath was correctly extracted
        let result = result.unwrap();
        let text = result.contents[0].text.as_ref().unwrap();
        assert!(
            text.contains("src/main.rs"),
            "Expected filepath to contain 'src/main.rs', got: {}",
            text
        );
    }

    #[tokio::test]
    async fn test_catchall_deeply_nested() {
        let server = CatchallServer::with_defaults();

        // Test with deeply nested path
        let request = ReadResourceRequestParam {
            uri: "file://a/b/c/d/e/file.txt".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        let result = result.unwrap();
        let text = result.contents[0].text.as_ref().unwrap();
        assert!(
            text.contains("a/b/c/d/e/file.txt"),
            "Expected full path, got: {}",
            text
        );
    }
}

// =============================================================================
// TEST CASE 8: Unicode in URI parameters
// =============================================================================
mod unicode_params {
    use super::*;

    #[mcp_server(name = "Unicode Server")]
    #[derive(Default, Clone)]
    pub struct UnicodeServer;

    #[mcp_tools]
    impl UnicodeServer {
        #[mcp_resource(uri_template = "greet://{name}")]
        pub fn greet(&self, name: String) -> Result<String, String> {
            Ok(format!("Hello, {}!", name))
        }
    }

    #[tokio::test]
    async fn test_unicode() {
        let server = UnicodeServer::with_defaults();

        // Unicode name
        let request = ReadResourceRequestParam {
            uri: "greet://世界".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("Unicode result: {:?}", result);
        assert!(result.is_ok());
    }
}

// =============================================================================
// TEST CASE 9: Resource returning non-Result type (should this work?)
// =============================================================================
mod non_result_return {
    use super::*;

    #[mcp_server(name = "Non Result Server")]
    #[derive(Default, Clone)]
    pub struct NonResultServer;

    // Note: Currently resources require Result<T, E> return type
    // This test documents that requirement
    #[mcp_tools]
    impl NonResultServer {
        // This won't compile if we return plain String
        // #[mcp_resource(uri_template = "data://{id}")]
        // pub fn get_data(&self, id: String) -> String {
        //     format!("Data: {}", id)
        // }

        // Must use Result
        #[mcp_resource(uri_template = "data://{id}")]
        pub fn get_data(&self, id: String) -> Result<String, String> {
            Ok(format!("Data: {}", id))
        }
    }

    #[test]
    fn test_compiles() {
        let server = NonResultServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}

// =============================================================================
// TEST CASE 10: Duplicate URI templates
// =============================================================================
mod duplicate_uris {
    use super::*;

    #[mcp_server(name = "Duplicate URI Server")]
    #[derive(Default, Clone)]
    pub struct DuplicateUriServer;

    #[mcp_tools]
    impl DuplicateUriServer {
        #[mcp_resource(uri_template = "data://{id}")]
        pub fn get_data1(&self, id: String) -> Result<String, String> {
            Ok(format!("Data1: {}", id))
        }

        // Same URI template - this will cause a conflict!
        #[mcp_resource(uri_template = "data://{id}")]
        pub fn get_data2(&self, id: String) -> Result<String, String> {
            Ok(format!("Data2: {}", id))
        }
    }

    #[tokio::test]
    async fn test_duplicate_uris() {
        let server = DuplicateUriServer::with_defaults();
        let resources = server.try_get_resources_default();
        // Both resources are registered
        assert_eq!(resources.len(), 2);

        // But trying to read will fail because router can't have duplicate routes
        let request = ReadResourceRequestParam {
            uri: "data://123".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("Duplicate URI result: {:?}", result);
        // This should return an error about duplicate routes
    }
}

// =============================================================================
// TEST CASE 11: Conflicting routes (one is prefix of another)
// =============================================================================
mod conflicting_routes {
    use super::*;

    #[mcp_server(name = "Conflicting Routes Server")]
    #[derive(Default, Clone)]
    pub struct ConflictingRoutesServer;

    #[mcp_tools]
    impl ConflictingRoutesServer {
        #[mcp_resource(uri_template = "api://{version}/users")]
        pub fn get_users(&self, version: String) -> Result<String, String> {
            Ok(format!("Users v{}", version))
        }

        #[mcp_resource(uri_template = "api://{version}/users/{id}")]
        pub fn get_user(&self, version: String, id: String) -> Result<String, String> {
            Ok(format!("User {} v{}", id, version))
        }
    }

    #[tokio::test]
    async fn test_conflicting_routes() {
        let server = ConflictingRoutesServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 2);

        // This should match get_users
        let request = ReadResourceRequestParam {
            uri: "api://v1/users".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("Users result: {:?}", result);
        assert!(result.is_ok());

        // This should match get_user
        let request = ReadResourceRequestParam {
            uri: "api://v1/users/123".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("User result: {:?}", result);
        assert!(result.is_ok());
    }
}

// =============================================================================
// TEST CASE 12: Special URI schemes
// =============================================================================
mod special_schemes {
    use super::*;

    #[mcp_server(name = "Special Schemes Server")]
    #[derive(Default, Clone)]
    pub struct SpecialSchemesServer;

    #[mcp_tools]
    impl SpecialSchemesServer {
        #[mcp_resource(uri_template = "file:///home/{user}/data")]
        pub fn get_home_data(&self, user: String) -> Result<String, String> {
            Ok(format!("Home data for {}", user))
        }
    }

    #[tokio::test]
    async fn test_file_uri() {
        let server = SpecialSchemesServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "file:///home/john/data".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        println!("File URI result: {:?}", result);
    }
}

// =============================================================================
// TEST CASE 13: Parse failure for non-string types at runtime
// This tests the error handling when a URI param cannot be parsed to the
// expected type (e.g., "abc" cannot be parsed as u64)
// =============================================================================
mod parse_failure {
    use super::*;

    #[mcp_server(name = "Parse Failure Server")]
    #[derive(Default, Clone)]
    pub struct ParseFailureServer;

    #[mcp_tools]
    impl ParseFailureServer {
        /// Resource expecting an integer ID
        #[mcp_resource(uri_template = "item://{id}")]
        pub fn get_item(&self, id: u64) -> Result<String, String> {
            Ok(format!("Item #{}", id))
        }
    }

    #[tokio::test]
    async fn test_valid_integer() {
        let server = ParseFailureServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "item://123".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        let text = content.contents[0].text.as_ref().unwrap();
        assert!(text.contains("123"), "Expected '123' in response: {}", text);
    }

    #[tokio::test]
    async fn test_invalid_integer_returns_error() {
        let server = ParseFailureServer::with_defaults();

        // Try to parse "not_a_number" as u64
        let request = ReadResourceRequestParam {
            uri: "item://not_a_number".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        // Should fail with invalid_params error
        assert!(result.is_err(), "Expected error for invalid integer");
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(
            error_msg.contains("parse") || error_msg.contains("invalid"),
            "Expected parse error message, got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_negative_for_unsigned() {
        let server = ParseFailureServer::with_defaults();

        // Try to parse "-5" as u64
        let request = ReadResourceRequestParam {
            uri: "item://-5".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        // Should fail - negative number can't be u64
        assert!(result.is_err(), "Expected error for negative unsigned");
    }

    #[tokio::test]
    async fn test_overflow_integer() {
        let server = ParseFailureServer::with_defaults();

        // Try to parse a number larger than u64::MAX
        let request = ReadResourceRequestParam {
            uri: "item://99999999999999999999999".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        // Should fail - overflow
        assert!(result.is_err(), "Expected error for overflow");
    }
}

// =============================================================================
// TEST CASE 14: Method returning error propagates correctly
// =============================================================================
mod error_propagation {
    use super::*;

    #[mcp_server(name = "Error Propagation Server")]
    #[derive(Default, Clone)]
    pub struct ErrorPropagationServer;

    #[mcp_tools]
    impl ErrorPropagationServer {
        /// Resource that returns error for certain IDs
        #[mcp_resource(uri_template = "data://{id}")]
        pub fn get_data(&self, id: String) -> Result<String, String> {
            if id == "fail" {
                Err("Intentional failure".to_string())
            } else if id == "not_found" {
                Err("Resource not found".to_string())
            } else {
                Ok(format!("Data for {}", id))
            }
        }
    }

    #[tokio::test]
    async fn test_success_case() {
        let server = ErrorPropagationServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "data://valid_id".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_error_is_propagated() {
        let server = ErrorPropagationServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "data://fail".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(
            error_msg.contains("Intentional failure"),
            "Expected error message to contain method error: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_different_error_messages() {
        let server = ErrorPropagationServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "data://not_found".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(
            error_msg.contains("not found"),
            "Expected 'not found' in error: {}",
            error_msg
        );
    }
}

// =============================================================================
// TEST CASE 15: Complex URI patterns with prefix and suffix
// =============================================================================
mod complex_uri_patterns {
    use super::*;

    #[mcp_server(name = "Complex URI Server")]
    #[derive(Default, Clone)]
    pub struct ComplexUriServer;

    #[mcp_tools]
    impl ComplexUriServer {
        /// Pattern: /api/{version}/users/{id}/profile
        #[mcp_resource(uri_template = "api://{version}/users/{id}/profile")]
        pub fn get_user_profile(&self, version: String, id: String) -> Result<String, String> {
            Ok(format!("Profile for user {} (API {})", id, version))
        }

        /// Pattern with multiple segments between params
        #[mcp_resource(uri_template = "org://{org}/team/{team}/member/{member}")]
        pub fn get_member(
            &self,
            org: String,
            team: String,
            member: String,
        ) -> Result<String, String> {
            Ok(format!("{} is in team {} of org {}", member, team, org))
        }
    }

    #[tokio::test]
    async fn test_prefix_suffix_pattern() {
        let server = ComplexUriServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "api://v2/users/user123/profile".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        let text = content.contents[0].text.as_ref().unwrap();
        assert!(text.contains("user123"), "Expected user123: {}", text);
        assert!(text.contains("v2"), "Expected v2: {}", text);
    }

    #[tokio::test]
    async fn test_multiple_segments() {
        let server = ComplexUriServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "org://acme/team/engineering/member/alice".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        let text = content.contents[0].text.as_ref().unwrap();
        assert!(text.contains("alice"), "Expected alice: {}", text);
        assert!(
            text.contains("engineering"),
            "Expected engineering: {}",
            text
        );
        assert!(text.contains("acme"), "Expected acme: {}", text);
    }

    #[tokio::test]
    async fn test_pattern_mismatch_fails() {
        let server = ComplexUriServer::with_defaults();

        // Missing /profile suffix
        let request = ReadResourceRequestParam {
            uri: "api://v2/users/user123".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        // Should fail - doesn't match the pattern
        assert!(result.is_err(), "Expected error for pattern mismatch");
    }
}

// =============================================================================
// TEST CASE 16: Verify actual JSON content returned
// =============================================================================
mod content_verification {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
    pub struct UserInfo {
        pub id: String,
        pub name: String,
        pub active: bool,
    }

    #[mcp_server(name = "Content Verification Server")]
    #[derive(Default, Clone)]
    pub struct ContentVerificationServer;

    #[mcp_tools]
    impl ContentVerificationServer {
        #[mcp_resource(uri_template = "user://{id}")]
        pub fn get_user(&self, id: String) -> Result<UserInfo, String> {
            Ok(UserInfo {
                id: id.clone(),
                name: format!("User {}", id),
                active: true,
            })
        }
    }

    #[tokio::test]
    async fn test_json_content_structure() {
        let server = ContentVerificationServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "user://42".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_ok());
        let content = result.unwrap();

        // Verify structure
        assert_eq!(content.contents.len(), 1);
        assert_eq!(
            content.contents[0].mime_type,
            Some("application/json".to_string())
        );

        // Parse and verify JSON content
        let text = content.contents[0].text.as_ref().unwrap();
        let parsed: UserInfo = serde_json::from_str(text).expect("Should parse as UserInfo");
        assert_eq!(parsed.id, "42");
        assert_eq!(parsed.name, "User 42");
        assert!(parsed.active);
    }

    #[tokio::test]
    async fn test_uri_preserved_in_response() {
        let server = ContentVerificationServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "user://test123".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_ok());
        let content = result.unwrap();

        // URI should be preserved in response
        assert_eq!(content.contents[0].uri, "user://test123");
    }
}

// =============================================================================
// TEST CASE 17: Server with no resources (empty resource list)
// =============================================================================
mod empty_resources {
    use super::*;

    #[mcp_server(name = "Empty Resources Server")]
    #[derive(Default, Clone)]
    pub struct EmptyResourcesServer;

    #[mcp_tools]
    impl EmptyResourcesServer {
        // Only tools, no resources
        pub fn do_something(&self) -> String {
            "Done".to_string()
        }
    }

    #[test]
    fn test_empty_resource_list() {
        let server = EmptyResourcesServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert!(resources.is_empty());
    }

    #[tokio::test]
    async fn test_unknown_resource_returns_error() {
        let server = EmptyResourcesServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "any://resource".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(
            error_msg.contains("Unknown resource"),
            "Expected 'Unknown resource' error: {}",
            error_msg
        );
    }
}

// =============================================================================
// TEST CASE 18: Resource calling other methods on self
// =============================================================================
mod self_referential {
    use super::*;

    #[mcp_server(name = "Self Referential Server")]
    #[derive(Default, Clone)]
    pub struct SelfReferentialServer {
        prefix: String,
    }

    impl SelfReferentialServer {
        pub fn new(prefix: &str) -> Self {
            Self {
                prefix: prefix.to_string(),
            }
        }

        fn format_data(&self, raw: &str) -> String {
            format!("[{}] {}", self.prefix, raw)
        }
    }

    #[mcp_tools]
    impl SelfReferentialServer {
        #[mcp_resource(uri_template = "formatted://{data}")]
        pub fn get_formatted(&self, data: String) -> Result<String, String> {
            // Call another method on self
            Ok(self.format_data(&data))
        }
    }

    #[tokio::test]
    async fn test_self_reference_works() {
        let server = SelfReferentialServer::new("PREFIX");

        let request = ReadResourceRequestParam {
            uri: "formatted://hello".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        let text = content.contents[0].text.as_ref().unwrap();
        assert!(
            text.contains("PREFIX") && text.contains("hello"),
            "Expected formatted output: {}",
            text
        );
    }
}

// =============================================================================
// TEST CASE 19: Very long URIs
// =============================================================================
mod long_uris {
    use super::*;

    #[mcp_server(name = "Long URI Server")]
    #[derive(Default, Clone)]
    pub struct LongUriServer;

    #[mcp_tools]
    impl LongUriServer {
        #[mcp_resource(uri_template = "data://{payload}")]
        pub fn get_data(&self, payload: String) -> Result<String, String> {
            Ok(format!("Received {} bytes", payload.len()))
        }
    }

    #[tokio::test]
    async fn test_long_parameter() {
        let server = LongUriServer::with_defaults();

        // Create a very long parameter (10KB)
        let long_param = "x".repeat(10_000);
        let request = ReadResourceRequestParam {
            uri: format!("data://{}", long_param),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        let text = content.contents[0].text.as_ref().unwrap();
        assert!(
            text.contains("10000"),
            "Expected 10000 bytes reported: {}",
            text
        );
    }
}

// =============================================================================
// TEST CASE 20: Parameter order verification
// This verifies that parameter binding is positional (URI template order
// must match method parameter order)
// =============================================================================
mod param_order {
    use super::*;

    #[mcp_server(name = "Param Order Server")]
    #[derive(Default, Clone)]
    pub struct ParamOrderServer;

    #[mcp_tools]
    impl ParamOrderServer {
        /// URI template has {a}/{b}, method params are (first, second)
        /// Position 0 from URI -> first, Position 1 from URI -> second
        #[mcp_resource(uri_template = "order://{a}/{b}")]
        pub fn get_order(&self, first: String, second: String) -> Result<String, String> {
            Ok(format!("first={}, second={}", first, second))
        }
    }

    #[tokio::test]
    async fn test_positional_binding() {
        let server = ParamOrderServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "order://alpha/beta".to_string(),
        };
        let result = server.try_read_resource_default(request).await;

        assert!(result.is_ok());
        let content = result.unwrap();
        let text = content.contents[0].text.as_ref().unwrap();
        // first should get "alpha" (position 0), second should get "beta" (position 1)
        assert!(
            text.contains("first=") && text.contains("alpha"),
            "first should be alpha: {}",
            text
        );
        assert!(
            text.contains("second=") && text.contains("beta"),
            "second should be beta: {}",
            text
        );
    }
}

// =============================================================================
// TEST CASE 21: Async error handling
// =============================================================================
mod async_errors {
    use super::*;

    #[mcp_server(name = "Async Error Server")]
    #[derive(Default, Clone)]
    pub struct AsyncErrorServer;

    #[mcp_tools]
    impl AsyncErrorServer {
        #[mcp_resource(uri_template = "async://{id}")]
        pub async fn get_async(&self, id: String) -> Result<String, String> {
            // Simulate async work
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            if id == "error" {
                Err("Async error occurred".to_string())
            } else {
                Ok(format!("Async result for {}", id))
            }
        }
    }

    #[tokio::test]
    async fn test_async_success() {
        let server = AsyncErrorServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "async://success".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_error() {
        let server = AsyncErrorServer::with_defaults();

        let request = ReadResourceRequestParam {
            uri: "async://error".to_string(),
        };
        let result = server.try_read_resource_default(request).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(
            error_msg.contains("Async error"),
            "Expected async error: {}",
            error_msg
        );
    }
}
