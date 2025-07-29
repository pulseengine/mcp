//! Tests for type system integration and complex type handling

use pulseengine_mcp_macros::{mcp_prompt, mcp_resource, mcp_server, mcp_tools};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod custom_types {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct User {
        pub id: u64,
        pub name: String,
        pub email: String,
        pub active: bool,
        pub metadata: HashMap<String, String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateUserRequest {
        pub name: String,
        pub email: String,
        pub initial_metadata: Option<HashMap<String, String>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UpdateUserRequest {
        pub name: Option<String>,
        pub email: Option<String>,
        pub active: Option<bool>,
        pub metadata_updates: Option<HashMap<String, String>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum UserRole {
        Admin,
        Moderator,
        User,
        Guest,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PaginationParams {
        pub limit: Option<u32>,
        pub offset: Option<u32>,
        pub sort_by: Option<String>,
        pub order: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PaginatedResponse<T> {
        pub items: Vec<T>,
        pub total: u64,
        pub limit: u32,
        pub offset: u32,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum UserError {
        #[error("User not found: {id}")]
        NotFound { id: u64 },
        #[error("Invalid email format: {email}")]
        InvalidEmail { email: String },
        #[error("Duplicate user: {field}")]
        Duplicate { field: String },
        #[error("Validation error: {message}")]
        Validation { message: String },
    }
}

mod type_system_server {
    use super::*;
    use custom_types::*;

    #[mcp_server(name = "Type System Test Server")]
    #[derive(Clone)]
    pub struct TypeSystemServer {
        users: std::sync::Arc<std::sync::RwLock<HashMap<u64, User>>>,
        next_id: std::sync::Arc<std::sync::atomic::AtomicU64>,
    }

    impl Default for TypeSystemServer {
        fn default() -> Self {
            let mut users = HashMap::new();
            users.insert(
                1,
                User {
                    id: 1,
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                    active: true,
                    metadata: [("role".to_string(), "admin".to_string())]
                        .into_iter()
                        .collect(),
                },
            );
            users.insert(
                2,
                User {
                    id: 2,
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                    active: true,
                    metadata: HashMap::new(),
                },
            );

            Self {
                users: std::sync::Arc::new(std::sync::RwLock::new(users)),
                next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(3)),
            }
        }
    }

    #[mcp_tools]
    impl TypeSystemServer {
        /// Create a new user with complex type handling
        async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserError> {
            // Validate email format
            if !request.email.contains('@') {
                return Err(UserError::InvalidEmail {
                    email: request.email,
                });
            }

            // Check for duplicates
            let users = self.users.read().unwrap();
            for user in users.values() {
                if user.email == request.email {
                    return Err(UserError::Duplicate {
                        field: "email".to_string(),
                    });
                }
                if user.name == request.name {
                    return Err(UserError::Duplicate {
                        field: "name".to_string(),
                    });
                }
            }
            drop(users);

            // Create new user
            let id = self
                .next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let user = User {
                id,
                name: request.name,
                email: request.email,
                active: true,
                metadata: request.initial_metadata.unwrap_or_default(),
            };

            // Store user
            let mut users = self.users.write().unwrap();
            users.insert(id, user.clone());

            Ok(user)
        }

        /// Get user by ID with optional field selection
        async fn get_user(
            &self,
            id: u64,
            include_metadata: Option<bool>,
        ) -> Result<User, UserError> {
            let users = self.users.read().unwrap();
            let mut user = users.get(&id).cloned().ok_or(UserError::NotFound { id })?;

            // Optionally exclude metadata
            if !include_metadata.unwrap_or(true) {
                user.metadata.clear();
            }

            Ok(user)
        }

        /// Update user with partial update pattern
        async fn update_user(
            &self,
            id: u64,
            request: UpdateUserRequest,
        ) -> Result<User, UserError> {
            let mut users = self.users.write().unwrap();
            let user = users.get_mut(&id).ok_or(UserError::NotFound { id })?;

            // Apply updates
            if let Some(name) = request.name {
                if name.is_empty() {
                    return Err(UserError::Validation {
                        message: "Name cannot be empty".to_string(),
                    });
                }
                user.name = name;
            }

            if let Some(email) = request.email {
                if !email.contains('@') {
                    return Err(UserError::InvalidEmail { email });
                }
                user.email = email;
            }

            if let Some(active) = request.active {
                user.active = active;
            }

            if let Some(metadata_updates) = request.metadata_updates {
                user.metadata.extend(metadata_updates);
            }

            Ok(user.clone())
        }

        /// List users with pagination and complex return types
        async fn list_users(&self, params: PaginationParams) -> PaginatedResponse<User> {
            let users = self.users.read().unwrap();
            let mut user_list: Vec<User> = users.values().cloned().collect();

            // Sort if requested
            if let Some(sort_by) = &params.sort_by {
                match sort_by.as_str() {
                    "name" => user_list.sort_by(|a, b| a.name.cmp(&b.name)),
                    "email" => user_list.sort_by(|a, b| a.email.cmp(&b.email)),
                    "id" => user_list.sort_by(|a, b| a.id.cmp(&b.id)),
                    _ => {} // Invalid sort field, ignore
                }

                // Apply order
                if params.order.as_deref() == Some("desc") {
                    user_list.reverse();
                }
            }

            let total = user_list.len() as u64;
            let offset = params.offset.unwrap_or(0) as usize;
            let limit = params.limit.unwrap_or(10) as usize;

            // Apply pagination
            let items = user_list.into_iter().skip(offset).take(limit).collect();

            PaginatedResponse {
                items,
                total,
                limit: limit as u32,
                offset: offset as u32,
            }
        }

        /// Delete user and return the deleted user
        async fn delete_user(&self, id: u64) -> Result<User, UserError> {
            let mut users = self.users.write().unwrap();
            users.remove(&id).ok_or(UserError::NotFound { id })
        }

        /// Work with enums and complex matching
        async fn set_user_role(&self, id: u64, role: UserRole) -> Result<String, UserError> {
            let mut users = self.users.write().unwrap();
            let user = users.get_mut(&id).ok_or(UserError::NotFound { id })?;

            let role_string = match role {
                UserRole::Admin => "admin",
                UserRole::Moderator => "moderator",
                UserRole::User => "user",
                UserRole::Guest => "guest",
            };

            user.metadata
                .insert("role".to_string(), role_string.to_string());

            Ok(format!("User {} role set to {}", user.name, role_string))
        }

        /// Generic type handling with vectors and maps
        async fn batch_update_metadata(
            &self,
            updates: HashMap<u64, HashMap<String, String>>,
        ) -> Result<Vec<u64>, UserError> {
            let mut users = self.users.write().unwrap();
            let mut updated_ids = Vec::new();

            for (user_id, metadata_updates) in updates {
                if let Some(user) = users.get_mut(&user_id) {
                    user.metadata.extend(metadata_updates);
                    updated_ids.push(user_id);
                }
            }

            Ok(updated_ids)
        }

        /// Complex nested types with Options and Results
        async fn search_users(
            &self,
            query: Option<String>,
            filters: Option<HashMap<String, String>>,
            limit: Option<u32>,
        ) -> Result<Vec<User>, UserError> {
            let users = self.users.read().unwrap();
            let mut results: Vec<User> = users.values().cloned().collect();

            // Apply query filter
            if let Some(q) = query {
                let query_lower = q.to_lowercase();
                results.retain(|user| {
                    user.name.to_lowercase().contains(&query_lower)
                        || user.email.to_lowercase().contains(&query_lower)
                });
            }

            // Apply metadata filters
            if let Some(filters) = filters {
                results.retain(|user| {
                    filters.iter().all(|(key, value)| {
                        user.metadata.get(key).map(|v| v == value).unwrap_or(false)
                    })
                });
            }

            // Apply limit
            if let Some(limit) = limit {
                results.truncate(limit as usize);
            }

            Ok(results)
        }
    }

    #[mcp_resource(uri_template = "user://{id}/profile")]
    impl TypeSystemServer {
        /// Resource with complex type serialization
        async fn user_profile_resource(
            &self,
            id: String,
        ) -> Result<serde_json::Value, std::io::Error> {
            let user_id: u64 = id.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid user ID")
            })?;

            let users = self.users.read().unwrap();
            let user = users.get(&user_id).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "User not found")
            })?;

            // Serialize to JSON
            serde_json::to_value(user)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        }
    }

    #[mcp_prompt(name = "user_prompt")]
    impl TypeSystemServer {
        /// Prompt with complex type handling in parameters
        async fn user_prompt(
            &self,
            user_data: serde_json::Value,
            template_type: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            // Parse user data
            let user: User = serde_json::from_value(user_data).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            })?;

            let prompt_text = match template_type.as_str() {
                "welcome" => format!(
                    "Welcome {}! We're glad to have you at {}.",
                    user.name, user.email
                ),
                "profile" => format!(
                    "User Profile:\nName: {}\nEmail: {}\nActive: {}\nMetadata: {:?}",
                    user.name, user.email, user.active, user.metadata
                ),
                "admin" => {
                    if user.metadata.get("role") == Some(&"admin".to_string()) {
                        format!("Admin user {} has full system access.", user.name)
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "Not an admin user",
                        ));
                    }
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Unknown template type",
                    ));
                }
            };

            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptMessageContent::Text { text: prompt_text },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custom_types::*;
    use type_system_server::*;

    #[test]
    fn test_custom_types_serialize() {
        let user = User {
            id: 1,
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
            active: true,
            metadata: [("key".to_string(), "value".to_string())]
                .into_iter()
                .collect(),
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(user, deserialized);
    }

    #[test]
    fn test_server_compiles() {
        let _server = TypeSystemServer::with_defaults();
    }

    #[tokio::test]
    async fn test_create_user() {
        let server = TypeSystemServer::with_defaults();

        let request = CreateUserRequest {
            name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
            initial_metadata: Some(
                [("department".to_string(), "engineering".to_string())]
                    .into_iter()
                    .collect(),
            ),
        };

        let result = server.create_user(request).await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.name, "Charlie");
        assert_eq!(user.email, "charlie@example.com");
        assert_eq!(
            user.metadata.get("department"),
            Some(&"engineering".to_string())
        );
        assert!(user.active);
    }

    #[tokio::test]
    async fn test_create_user_validation() {
        let server = TypeSystemServer::with_defaults();

        // Test invalid email
        let invalid_email_request = CreateUserRequest {
            name: "Invalid".to_string(),
            email: "not-an-email".to_string(),
            initial_metadata: None,
        };

        let result = server.create_user(invalid_email_request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            UserError::InvalidEmail { email } => assert_eq!(email, "not-an-email"),
            _ => panic!("Expected InvalidEmail error"),
        }

        // Test duplicate email
        let duplicate_request = CreateUserRequest {
            name: "Duplicate".to_string(),
            email: "alice@example.com".to_string(), // Already exists
            initial_metadata: None,
        };

        let result = server.create_user(duplicate_request).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            UserError::Duplicate { field } => assert_eq!(field, "email"),
            _ => panic!("Expected Duplicate error"),
        }
    }

    #[tokio::test]
    async fn test_get_user() {
        let server = TypeSystemServer::with_defaults();

        // Test existing user
        let result = server.get_user(1, Some(true)).await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.name, "Alice");
        assert!(!user.metadata.is_empty());

        // Test without metadata
        let result = server.get_user(1, Some(false)).await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert!(user.metadata.is_empty());

        // Test non-existent user
        let result = server.get_user(999, None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            UserError::NotFound { id } => assert_eq!(id, 999),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_update_user() {
        let server = TypeSystemServer::with_defaults();

        let update_request = UpdateUserRequest {
            name: Some("Alice Updated".to_string()),
            email: None,
            active: Some(false),
            metadata_updates: Some(
                [("status".to_string(), "updated".to_string())]
                    .into_iter()
                    .collect(),
            ),
        };

        let result = server.update_user(1, update_request).await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.name, "Alice Updated");
        assert!(!user.active);
        assert_eq!(user.metadata.get("status"), Some(&"updated".to_string()));
        assert_eq!(user.metadata.get("role"), Some(&"admin".to_string())); // Should preserve existing
    }

    #[tokio::test]
    async fn test_list_users_pagination() {
        let server = TypeSystemServer::with_defaults();

        let params = PaginationParams {
            limit: Some(1),
            offset: Some(0),
            sort_by: Some("name".to_string()),
            order: Some("asc".to_string()),
        };

        let result = server.list_users(params).await;
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 2);
        assert_eq!(result.limit, 1);
        assert_eq!(result.offset, 0);
        assert_eq!(result.items[0].name, "Alice"); // Should be first alphabetically
    }

    #[tokio::test]
    async fn test_user_role_enum() {
        let server = TypeSystemServer::with_defaults();

        let result = server.set_user_role(1, UserRole::Moderator).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("moderator"));

        // Verify the role was set
        let user = server.get_user(1, Some(true)).await.unwrap();
        assert_eq!(user.metadata.get("role"), Some(&"moderator".to_string()));
    }

    #[tokio::test]
    async fn test_batch_update_metadata() {
        let server = TypeSystemServer::with_defaults();

        let mut updates = HashMap::new();
        updates.insert(
            1,
            [("batch_key".to_string(), "batch_value".to_string())]
                .into_iter()
                .collect(),
        );
        updates.insert(
            2,
            [("another_key".to_string(), "another_value".to_string())]
                .into_iter()
                .collect(),
        );
        updates.insert(
            999,
            [("nonexistent".to_string(), "value".to_string())]
                .into_iter()
                .collect(),
        ); // Should be ignored

        let result = server.batch_update_metadata(updates).await;
        assert!(result.is_ok());

        let updated_ids = result.unwrap();
        assert_eq!(updated_ids.len(), 2);
        assert!(updated_ids.contains(&1));
        assert!(updated_ids.contains(&2));
        assert!(!updated_ids.contains(&999));

        // Verify updates were applied
        let user1 = server.get_user(1, Some(true)).await.unwrap();
        assert_eq!(
            user1.metadata.get("batch_key"),
            Some(&"batch_value".to_string())
        );
    }

    #[tokio::test]
    async fn test_search_users_complex() {
        let server = TypeSystemServer::with_defaults();

        // Search by query
        let result = server
            .search_users(Some("alice".to_string()), None, None)
            .await;
        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Alice");

        // Search by metadata filter
        let mut filters = HashMap::new();
        filters.insert("role".to_string(), "admin".to_string());
        let result = server.search_users(None, Some(filters), None).await;
        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "Alice");

        // Search with limit
        let result = server.search_users(None, None, Some(1)).await;
        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 1);
    }

    #[tokio::test]
    async fn test_user_profile_resource() {
        let server = TypeSystemServer::with_defaults();

        let result = server.user_profile_resource("1".to_string()).await;
        assert!(result.is_ok());

        let json_value = result.unwrap();
        assert_eq!(json_value["name"], "Alice");
        assert_eq!(json_value["email"], "alice@example.com");
        assert_eq!(json_value["active"], true);

        // Test invalid ID
        let result = server.user_profile_resource("invalid".to_string()).await;
        assert!(result.is_err());

        // Test non-existent user
        let result = server.user_profile_resource("999".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_user_prompt_complex_types() {
        let server = TypeSystemServer::with_defaults();

        let user_data = serde_json::json!({
            "id": 1,
            "name": "Test User",
            "email": "test@example.com",
            "active": true,
            "metadata": {"role": "admin"}
        });

        // Test welcome template
        let result = server
            .user_prompt(user_data.clone(), "welcome".to_string())
            .await;
        assert!(result.is_ok());
        let message = result.unwrap();
        if let pulseengine_mcp_protocol::PromptMessageContent::Text { text } = message.content {
            assert!(text.contains("Test User"));
            assert!(text.contains("test@example.com"));
        }

        // Test admin template
        let result = server
            .user_prompt(user_data.clone(), "admin".to_string())
            .await;
        assert!(result.is_ok());

        // Test non-admin user with admin template
        let mut non_admin_data = user_data.clone();
        non_admin_data["metadata"]["role"] = serde_json::Value::String("user".to_string());
        let result = server
            .user_prompt(non_admin_data, "admin".to_string())
            .await;
        assert!(result.is_err());

        // Test invalid user data
        let invalid_data = serde_json::json!({"invalid": "data"});
        let result = server
            .user_prompt(invalid_data, "welcome".to_string())
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_error_types() {
        let error1 = UserError::NotFound { id: 123 };
        assert_eq!(error1.to_string(), "User not found: 123");

        let error2 = UserError::InvalidEmail {
            email: "bad@".to_string(),
        };
        assert_eq!(error2.to_string(), "Invalid email format: bad@");

        let error3 = UserError::Duplicate {
            field: "email".to_string(),
        };
        assert_eq!(error3.to_string(), "Duplicate user: email");

        let error4 = UserError::Validation {
            message: "test error".to_string(),
        };
        assert_eq!(error4.to_string(), "Validation error: test error");
    }

    #[test]
    fn test_complex_type_serialization_round_trip() {
        let pagination = PaginationParams {
            limit: Some(50),
            offset: Some(100),
            sort_by: Some("name".to_string()),
            order: Some("desc".to_string()),
        };

        let json = serde_json::to_string(&pagination).unwrap();
        let deserialized: PaginationParams = serde_json::from_str(&json).unwrap();

        assert_eq!(pagination.limit, deserialized.limit);
        assert_eq!(pagination.offset, deserialized.offset);
        assert_eq!(pagination.sort_by, deserialized.sort_by);
        assert_eq!(pagination.order, deserialized.order);
    }

    #[test]
    fn test_generic_types() {
        let response = PaginatedResponse {
            items: vec!["item1".to_string(), "item2".to_string()],
            total: 100,
            limit: 10,
            offset: 20,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: PaginatedResponse<String> = serde_json::from_str(&json).unwrap();

        assert_eq!(response.items, deserialized.items);
        assert_eq!(response.total, deserialized.total);
    }
}
