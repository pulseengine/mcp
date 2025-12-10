//! Tests for request context functionality

use crate::context::RequestContext;
use pulseengine_mcp_protocol::Implementation;
use uuid::Uuid;

#[test]
fn test_request_context_new() {
    let context = RequestContext::new();

    // Should have a valid UUID
    assert_ne!(context.request_id, Uuid::nil());

    // Should be empty by default
    assert!(context.metadata.is_empty());
    assert!(context.client_info.is_none());
    assert!(context.authenticated_user.is_none());
    assert!(context.roles.is_empty());
}

#[test]
fn test_request_context_default() {
    let context = RequestContext::default();

    // Should be equivalent to new()
    assert_ne!(context.request_id, Uuid::nil());
    assert!(context.metadata.is_empty());
    assert!(context.client_info.is_none());
    assert!(context.authenticated_user.is_none());
    assert!(context.roles.is_empty());
}

#[test]
fn test_request_context_with_id() {
    let test_id = Uuid::new_v4();
    let context = RequestContext::with_id(test_id);

    assert_eq!(context.request_id, test_id);
    assert!(context.metadata.is_empty());
    assert!(context.client_info.is_none());
    assert!(context.authenticated_user.is_none());
    assert!(context.roles.is_empty());
}

#[test]
fn test_request_context_with_client_info() {
    let client_info = Implementation::new("Test Client", "1.0.0");

    let context = RequestContext::new().with_client_info(client_info.clone());

    assert!(context.client_info.is_some());
    assert_eq!(context.client_info.unwrap().name, "Test Client");
}

#[test]
fn test_request_context_with_user() {
    let context = RequestContext::new().with_user("test_user");

    assert!(context.authenticated_user.is_some());
    assert_eq!(context.authenticated_user.as_ref().unwrap(), "test_user");
    assert!(context.is_authenticated());
}

#[test]
fn test_request_context_with_user_string() {
    let user = "test_user".to_string();
    let context = RequestContext::new().with_user(user.clone());

    assert!(context.authenticated_user.is_some());
    assert_eq!(context.authenticated_user.unwrap(), user);
}

#[test]
fn test_request_context_with_role() {
    let context = RequestContext::new().with_role("admin");

    assert_eq!(context.roles.len(), 1);
    assert!(context.roles.contains(&"admin".to_string()));
    assert!(context.has_role("admin"));
    assert!(!context.has_role("user"));
}

#[test]
fn test_request_context_with_multiple_roles() {
    let context = RequestContext::new()
        .with_role("admin")
        .with_role("user")
        .with_role("moderator");

    assert_eq!(context.roles.len(), 3);
    assert!(context.has_role("admin"));
    assert!(context.has_role("user"));
    assert!(context.has_role("moderator"));
    assert!(!context.has_role("guest"));
}

#[test]
fn test_request_context_with_metadata() {
    let context = RequestContext::new()
        .with_metadata("key1", "value1")
        .with_metadata("key2", "value2");

    assert_eq!(context.metadata.len(), 2);
    assert_eq!(context.get_metadata("key1"), Some(&"value1".to_string()));
    assert_eq!(context.get_metadata("key2"), Some(&"value2".to_string()));
    assert_eq!(context.get_metadata("nonexistent"), None);
}

#[test]
fn test_request_context_metadata_with_string() {
    let key = "test_key".to_string();
    let value = "test_value".to_string();

    let context = RequestContext::new().with_metadata(key.clone(), value.clone());

    assert_eq!(context.get_metadata(&key), Some(&value));
}

#[test]
fn test_request_context_builder_pattern() {
    let client_info = Implementation::new("Builder Test Client", "2.0.0");

    let context = RequestContext::new()
        .with_client_info(client_info.clone())
        .with_user("builder_user")
        .with_role("admin")
        .with_role("user")
        .with_metadata("session_id", "abc123")
        .with_metadata("ip_address", "192.168.1.1");

    // Verify all fields are set correctly
    assert!(context.client_info.is_some());
    assert_eq!(
        context.client_info.as_ref().unwrap().name,
        "Builder Test Client"
    );

    assert!(context.authenticated_user.is_some());
    assert_eq!(context.authenticated_user.as_ref().unwrap(), "builder_user");
    assert!(context.is_authenticated());

    assert_eq!(context.roles.len(), 2);
    assert!(context.has_role("admin"));
    assert!(context.has_role("user"));

    assert_eq!(context.metadata.len(), 2);
    assert_eq!(
        context.get_metadata("session_id"),
        Some(&"abc123".to_string())
    );
    assert_eq!(
        context.get_metadata("ip_address"),
        Some(&"192.168.1.1".to_string())
    );
}

#[test]
fn test_request_context_is_authenticated() {
    let unauthenticated_context = RequestContext::new();
    assert!(!unauthenticated_context.is_authenticated());

    let authenticated_context = RequestContext::new().with_user("test_user");
    assert!(authenticated_context.is_authenticated());
}

#[test]
fn test_request_context_has_role_empty() {
    let context = RequestContext::new();
    assert!(!context.has_role("any_role"));
}

#[test]
fn test_request_context_has_role_case_sensitivity() {
    let context = RequestContext::new().with_role("Admin");

    assert!(context.has_role("Admin"));
    assert!(!context.has_role("admin")); // Case sensitive
    assert!(!context.has_role("ADMIN"));
}

#[test]
fn test_request_context_metadata_overwrite() {
    let context = RequestContext::new()
        .with_metadata("key", "value1")
        .with_metadata("key", "value2");

    // Should overwrite the previous value
    assert_eq!(context.get_metadata("key"), Some(&"value2".to_string()));
    assert_eq!(context.metadata.len(), 1);
}

#[test]
fn test_request_context_debug() {
    let context = RequestContext::new()
        .with_user("debug_user")
        .with_role("debug_role")
        .with_metadata("debug_key", "debug_value");

    let debug_str = format!("{context:?}");
    assert!(debug_str.contains("RequestContext"));
    assert!(debug_str.contains("debug_user"));
    assert!(debug_str.contains("debug_role"));
    assert!(debug_str.contains("debug_key"));
}

#[test]
fn test_request_context_clone() {
    let original = RequestContext::new()
        .with_user("clone_user")
        .with_role("clone_role")
        .with_metadata("clone_key", "clone_value");

    let cloned = original.clone();

    // Both should have the same values
    assert_eq!(original.request_id, cloned.request_id);
    assert_eq!(original.authenticated_user, cloned.authenticated_user);
    assert_eq!(original.roles, cloned.roles);
    assert_eq!(original.metadata, cloned.metadata);
}

#[test]
fn test_request_context_uuid_uniqueness() {
    let context1 = RequestContext::new();
    let context2 = RequestContext::new();

    // Each context should have a unique request ID
    assert_ne!(context1.request_id, context2.request_id);
}

#[test]
fn test_request_context_roles_order() {
    let context = RequestContext::new()
        .with_role("first")
        .with_role("second")
        .with_role("third");

    // Roles should be stored in the order they were added
    assert_eq!(context.roles[0], "first");
    assert_eq!(context.roles[1], "second");
    assert_eq!(context.roles[2], "third");
}

#[test]
fn test_request_context_roles_duplicates() {
    let context = RequestContext::new()
        .with_role("admin")
        .with_role("admin") // Duplicate
        .with_role("user");

    // Should allow duplicates (no deduplication)
    assert_eq!(context.roles.len(), 3);
    assert_eq!(context.roles[0], "admin");
    assert_eq!(context.roles[1], "admin");
    assert_eq!(context.roles[2], "user");
}

// Test thread safety
#[test]
fn test_request_context_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<RequestContext>();
    assert_sync::<RequestContext>();
}

#[test]
fn test_request_context_with_complex_client_info() {
    let client_info = Implementation::new(
        "Complex Client with Special Characters !@#$%",
        "1.0.0-beta.1+build.123",
    );

    let context = RequestContext::new().with_client_info(client_info.clone());

    assert!(context.client_info.is_some());
    let stored_info = context.client_info.unwrap();
    assert_eq!(
        stored_info.name,
        "Complex Client with Special Characters !@#$%"
    );
    assert_eq!(stored_info.version, "1.0.0-beta.1+build.123");
}

#[test]
fn test_request_context_metadata_empty_values() {
    let context = RequestContext::new()
        .with_metadata("empty_key", "")
        .with_metadata("", "empty_value");

    assert_eq!(context.get_metadata("empty_key"), Some(&"".to_string()));
    assert_eq!(context.get_metadata(""), Some(&"empty_value".to_string()));
}
