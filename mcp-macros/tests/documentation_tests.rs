//! Tests for documentation extraction and formatting

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;

#[test]
fn test_documented_server() {
    /// This is a comprehensive server example
    ///
    /// It demonstrates various documentation patterns:
    /// - Multi-line descriptions
    /// - Code examples
    /// - Usage notes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let server = DocumentedServer::with_defaults();
    /// ```
    #[mcp_server(name = "Documented Server")]
    #[derive(Default, Clone)]
    struct DocumentedServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl DocumentedServer {
        /// Process text data with various options
        ///
        /// This tool can:
        /// - Transform text case
        /// - Apply filters
        /// - Generate summaries
        ///
        /// # Parameters
        ///
        /// - `text`: The input text to process
        /// - `operation`: The operation to perform
        /// - `case_sensitive`: Whether to apply case-sensitive operations
        ///
        /// # Returns
        ///
        /// Returns the processed text as a `String`.
        ///
        /// # Examples
        ///
        /// ```ignore
        /// let result = server.process_text_data("Hello World", "uppercase", false);
        /// ```
        pub async fn process_text_data(
            &self,
            text: String,
            operation: String,
            case_sensitive: bool,
        ) -> String {
            match operation.as_str() {
                "uppercase" => {
                    // For this example, both case_sensitive and non-case_sensitive do the same thing
                    let _ = case_sensitive; // Acknowledge the parameter
                    text.to_uppercase()
                }
                "lowercase" => text.to_lowercase(),
                "reverse" => text.chars().rev().collect(),
                _ => text,
            }
        }

        /// Generate comprehensive analytics for data processing
        ///
        /// This tool provides detailed analytics including:
        /// - Processing statistics
        /// - Performance metrics
        /// - Usage patterns
        /// - Error rates
        ///
        /// The analytics are computed in real-time and provide
        /// insights into system behavior and performance.
        pub fn get_analytics(&self) -> serde_json::Value {
            serde_json::json!({
                "total_requests": 100,
                "success_rate": 0.95,
                "avg_response_time_ms": 45.2,
                "peak_requests_per_second": 150,
                "error_breakdown": {
                    "validation_errors": 3,
                    "timeout_errors": 1,
                    "system_errors": 1
                },
                "performance_metrics": {
                    "cpu_usage_percent": 25.3,
                    "memory_usage_mb": 128.7,
                    "disk_io_mb_per_sec": 2.1
                }
            })
        }

        /// Tool with minimal documentation
        pub async fn simple_tool(&self) -> String {
            "Simple result".to_string()
        }

        /// Single line documentation
        pub async fn single_line_doc(&self) -> String {
            "Single line result".to_string()
        }
    }

    let _server = DocumentedServer::with_defaults();
}

#[test]
fn test_parameter_documentation() {
    #[mcp_server(name = "Parameter Doc Server")]
    #[derive(Default, Clone)]
    struct ParameterDocServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl ParameterDocServer {
        /// Tool with extensively documented parameters
        ///
        /// # Parameters
        ///
        /// * `user_id` - Unique identifier for the user (must be positive)
        /// * `action` - The action to perform (supported: "create", "update", "delete")
        /// * `data` - JSON data payload containing the operation details
        /// * `dry_run` - If true, validate operation without executing it
        /// * `options` - Optional configuration parameters
        ///
        /// # Returns
        ///
        /// Returns operation result with status and details
        pub async fn documented_operation(
            &self,
            user_id: u64,
            action: String,
            data: serde_json::Value,
            dry_run: bool,
            options: Option<serde_json::Value>,
        ) -> serde_json::Value {
            serde_json::json!({
                "user_id": user_id,
                "action": action,
                "data": data,
                "dry_run": dry_run,
                "options": options,
                "status": "success",
                "timestamp": "2024-01-01T00:00:00Z"
            })
        }

        /// Tool demonstrating complex return type documentation
        ///
        /// Returns a structured result containing:
        /// - `items`: Array of processed items
        /// - `metadata`: Processing metadata and statistics
        /// - `pagination`: Pagination information if applicable
        /// - `errors`: Any non-fatal errors encountered during processing
        pub fn complex_return_documentation(&self) -> serde_json::Value {
            serde_json::json!({
                "items": [
                    {"id": 1, "name": "Item 1", "processed": true},
                    {"id": 2, "name": "Item 2", "processed": true}
                ],
                "metadata": {
                    "total_items": 2,
                    "processing_time_ms": 150,
                    "version": "1.0.0"
                },
                "pagination": {
                    "page": 1,
                    "per_page": 10,
                    "total_pages": 1
                },
                "errors": []
            })
        }
    }

    let _server = ParameterDocServer::with_defaults();
}

#[test]
fn test_example_documentation() {
    #[mcp_server(name = "Example Doc Server")]
    #[derive(Default, Clone)]
    struct ExampleDocServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl ExampleDocServer {
        /// Mathematical operations with comprehensive examples
        ///
        /// This tool performs various mathematical operations on the input values.
        ///
        /// # Examples
        ///
        /// Basic addition:
        /// ```ignore
        /// let result = server.math_operation(5.0, 3.0, "add").await;
        /// assert_eq!(result, 8.0);
        /// ```
        ///
        /// Division with error handling:
        /// ```ignore
        /// let result = server.math_operation(10.0, 0.0, "divide").await;
        /// // Returns NaN for division by zero
        /// ```
        ///
        /// Supported operations:
        /// - `add`: Addition (a + b)
        /// - `subtract`: Subtraction (a - b)
        /// - `multiply`: Multiplication (a * b)
        /// - `divide`: Division (a / b, returns NaN if b is 0)
        /// - `power`: Exponentiation (a^b)
        pub async fn math_operation(&self, a: f64, b: f64, operation: String) -> f64 {
            match operation.as_str() {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        f64::NAN
                    } else {
                        a / b
                    }
                }
                "power" => a.powf(b),
                _ => f64::NAN,
            }
        }

        /// String manipulation with usage examples
        ///
        /// # Usage Examples
        ///
        /// Transform text to title case:
        /// ```ignore
        /// let result = server.string_transform("hello world", "title").await;
        /// // Returns "Hello World"
        /// ```
        ///
        /// Reverse a string:
        /// ```ignore
        /// let result = server.string_transform("hello", "reverse").await;
        /// // Returns "olleh"
        /// ```
        pub async fn string_transform(&self, input: String, transform: String) -> String {
            match transform.as_str() {
                "title" => input
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().collect::<String>()
                                    + &chars.as_str().to_lowercase()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                "reverse" => input.chars().rev().collect(),
                "snake_case" => input.to_lowercase().replace(' ', "_"),
                _ => input,
            }
        }
    }

    let _server = ExampleDocServer::with_defaults();
}
