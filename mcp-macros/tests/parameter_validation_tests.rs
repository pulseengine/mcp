//! Tests for parameter validation and edge cases

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use serde_json::json;

mod parameter_types {
    use super::*;

    #[mcp_server(name = "Parameter Test Server")]
    #[derive(Default, Clone)]
    pub struct ParameterServer;

    #[mcp_tools]
    impl ParameterServer {
        /// Tool with various primitive types
        pub async fn primitive_types(
            &self,
            string_param: String,
            int_param: i32,
            uint_param: u64,
            float_param: f64,
            bool_param: bool,
        ) -> String {
            format!(
                "String: {}, Int: {}, UInt: {}, Float: {}, Bool: {}",
                string_param, int_param, uint_param, float_param, bool_param
            )
        }

        /// Tool with optional parameters
        pub async fn optional_params(
            &self,
            required: String,
            optional_string: Option<String>,
            optional_int: Option<i32>,
        ) -> String {
            format!(
                "Required: {}, OptStr: {:?}, OptInt: {:?}",
                required, optional_string, optional_int
            )
        }

        /// Tool with collection parameters
        pub async fn collection_params(
            &self,
            string_vec: Vec<String>,
            number_vec: Vec<i32>,
        ) -> String {
            format!(
                "Strings: {:?}, Numbers: {:?}",
                string_vec, number_vec
            )
        }

        /// Tool with JSON parameter
        pub async fn json_param(&self, data: serde_json::Value) -> String {
            format!("JSON data: {}", data.to_string())
        }

        /// Resource access with parameter validation
        pub async fn access_resource(
            &self,
            resource_type: String,
            resource_id: String,
        ) -> Result<String, std::io::Error> {
            if resource_type.is_empty() || resource_id.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Resource type and ID cannot be empty"
                ));
            }
            Ok(format!("Resource: {}/{}", resource_type, resource_id))
        }

        /// Complex resource with multiple parameters
        pub async fn complex_resource(
            &self,
            database: String,
            schema: String,
            table: String,
            action: String,
        ) -> Result<String, std::io::Error> {
            if database.is_empty() || schema.is_empty() || table.is_empty() || action.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "All parameters must be provided"
                ));
            }
            Ok(format!("Complex resource: {}.{}.{} action={}", database, schema, table, action))
        }

        /// Generate prompt with parameters
        pub async fn generate_prompt(&self, context: String, query: String) -> String {
            format!("Context: {} | Query: {}", context, query)
        }
    }
}

mod edge_cases {
    use super::*;

    #[mcp_server(name = "Edge Case Server")]
    #[derive(Default, Clone)]
    pub struct EdgeCaseServer;

    #[mcp_tools]
    impl EdgeCaseServer {
        /// Tool with very long parameter names
        pub async fn very_long_parameter_names(
            &self,
            this_is_a_very_long_parameter_name_that_tests_edge_cases: String,
            another_extremely_long_parameter_name_for_comprehensive_testing: String,
        ) -> String {
            format!("Long params: {} and {}", 
                this_is_a_very_long_parameter_name_that_tests_edge_cases,
                another_extremely_long_parameter_name_for_comprehensive_testing
            )
        }

        /// Tool with many parameters
        pub async fn many_parameters(
            &self,
            p1: String, p2: String, p3: String, p4: String, p5: String,
            p6: i32, p7: i32, p8: i32, p9: i32, p10: i32,
        ) -> String {
            format!("Many params: {},{},{},{},{},{},{},{},{},{}", 
                p1, p2, p3, p4, p5, p6, p7, p8, p9, p10)
        }

        /// Edge case resource access
        pub async fn edge_resource(&self, param: String) -> Result<String, std::io::Error> {
            if param.len() > 100 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Parameter too long"
                ));
            }
            Ok(format!("Edge resource: {}", param))
        }
    }
}

mod validation_server {
    use super::*;

    #[mcp_server(name = "Validation Server")]
    #[derive(Default, Clone)]
    pub struct ValidationServer;

    #[mcp_tools]
    impl ValidationServer {
        /// Strict validation tool
        pub async fn strict_validation(
            &self,
            email: String,
            age: u32,
        ) -> Result<String, std::io::Error> {
            // Email validation
            if !email.contains('@') {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid email format"
                ));
            }

            // Age validation
            if age < 18 || age > 120 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Age must be between 18 and 120"
                ));
            }

            Ok(format!("Valid user: {} (age {})", email, age))
        }

        /// Numeric boundary testing
        pub async fn numeric_boundaries(
            &self,
            min_int: i32,
            max_int: i32,
            small_float: f32,
            large_float: f64,
        ) -> String {
            format!("Boundaries: int={}-{}, float={}-{}", 
                min_int, max_int, small_float, large_float)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parameter_types::*;
    use edge_cases::*;
    use validation_server::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_parameter_servers_compile() {
        let param_server = ParameterServer::with_defaults();
        let edge_server = EdgeCaseServer::with_defaults();
        let validation_server = ValidationServer::with_defaults();

        let param_info = param_server.get_server_info();
        let edge_info = edge_server.get_server_info();
        let validation_info = validation_server.get_server_info();

        assert_eq!(param_info.server_info.name, "Parameter Test Server");
        assert_eq!(edge_info.server_info.name, "Edge Case Server");
        assert_eq!(validation_info.server_info.name, "Validation Server");
    }

    #[tokio::test]
    async fn test_primitive_types() {
        let server = ParameterServer::with_defaults();
        let result = server.primitive_types(
            "test".to_string(),
            42,
            100u64,
            3.14,
            true
        ).await;
        
        assert!(result.contains("test"));
        assert!(result.contains("42"));
        assert!(result.contains("100"));
        assert!(result.contains("3.14"));
        assert!(result.contains("true"));
    }

    #[tokio::test]
    async fn test_optional_parameters() {
        let server = ParameterServer::with_defaults();
        
        // With all parameters
        let result = server.optional_params(
            "required".to_string(),
            Some("optional".to_string()),
            Some(123)
        ).await;
        assert!(result.contains("required"));
        assert!(result.contains("optional"));
        assert!(result.contains("123"));

        // With only required parameter
        let result = server.optional_params(
            "required_only".to_string(),
            None,
            None
        ).await;
        assert!(result.contains("required_only"));
        assert!(result.contains("None"));
    }

    #[tokio::test]
    async fn test_validation_functionality() {
        let server = ValidationServer::with_defaults();

        // Valid input
        let result = server.strict_validation("test@example.com".to_string(), 25).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test@example.com"));

        // Invalid email
        let result = server.strict_validation("invalid_email".to_string(), 25).await;
        assert!(result.is_err());

        // Invalid age
        let result = server.strict_validation("test@example.com".to_string(), 15).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_edge_cases() {
        let server = EdgeCaseServer::with_defaults();

        let result = server.very_long_parameter_names(
            "test1".to_string(),
            "test2".to_string()
        ).await;
        assert!(result.contains("test1"));
        assert!(result.contains("test2"));

        // Test many parameters
        let result = server.many_parameters(
            "a".to_string(), "b".to_string(), "c".to_string(), "d".to_string(), "e".to_string(),
            1, 2, 3, 4, 5
        ).await;
        assert!(result.contains("a,b,c,d,e,1,2,3,4,5"));
    }
}