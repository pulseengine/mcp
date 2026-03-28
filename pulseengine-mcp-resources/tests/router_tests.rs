use pulseengine_mcp_resources::{ResourceRouter, strip_uri_scheme};
use rmcp::model::ResourceContents;

// ---------------------------------------------------------------------------
// Helper: extract text content from ResourceContents
// ---------------------------------------------------------------------------

fn extract_text(contents: &ResourceContents) -> &str {
    match contents {
        ResourceContents::TextResourceContents { text, .. } => text,
        ResourceContents::BlobResourceContents { .. } => {
            panic!("Expected text contents, got blob")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn register_and_list_templates() {
    let mut router = ResourceRouter::<()>::new();

    router.add_resource(
        "/files/{path}",
        "file:///{path}",
        "file",
        "Read a file by path",
        Some("text/plain"),
        |_state: &(), uri: &str, params: &matchit::Params| {
            let path = params.get("path").unwrap_or("unknown");
            ResourceContents::text(format!("Contents of {path}"), uri)
        },
    );

    router.add_resource(
        "/config/{section}/{key}",
        "config://{section}/{key}",
        "config",
        "Read a config value",
        None,
        |_state: &(), uri: &str, params: &matchit::Params| {
            let section = params.get("section").unwrap_or("?");
            let key = params.get("key").unwrap_or("?");
            ResourceContents::text(format!("[{section}] {key} = value"), uri)
        },
    );

    let templates = router.templates();
    assert_eq!(templates.len(), 2);
    assert_eq!(templates[0].raw.name, "file");
    assert_eq!(templates[0].raw.uri_template, "file:///{path}");
    assert_eq!(templates[1].raw.name, "config");
    assert_eq!(templates[1].raw.uri_template, "config://{section}/{key}");
}

#[test]
fn resolve_matching_uri() {
    let mut router = ResourceRouter::<()>::new();
    router.add_resource(
        "/files/{path}",
        "file:///{path}",
        "file",
        "Read a file",
        None,
        |_state: &(), uri: &str, params: &matchit::Params| {
            let path = params.get("path").unwrap_or("unknown");
            ResourceContents::text(format!("Mock contents of: {path}"), uri)
        },
    );

    let result = router.resolve(&(), "file:///README.md");
    assert!(result.is_some());

    let contents = result.unwrap();
    let text = extract_text(&contents);
    assert_eq!(text, "Mock contents of: README.md");
}

#[test]
fn resolve_non_matching_uri_returns_none() {
    let mut router = ResourceRouter::<()>::new();
    router.add_resource(
        "/files/{path}",
        "file:///{path}",
        "file",
        "Read a file",
        None,
        |_state: &(), uri: &str, _params: &matchit::Params| {
            ResourceContents::text("should not be called".to_string(), uri)
        },
    );

    let result = router.resolve(&(), "unknown://foo/bar");
    assert!(result.is_none());
}

#[test]
fn multiple_schemes() {
    let mut router = ResourceRouter::<()>::new();

    router.add_resource(
        "/files/{path}",
        "file:///{path}",
        "file",
        "Read a file",
        Some("text/plain"),
        |_state: &(), uri: &str, params: &matchit::Params| {
            let path = params.get("path").unwrap_or("?");
            ResourceContents::text(format!("file:{path}"), uri)
        },
    );

    router.add_resource(
        "/config/{section}/{key}",
        "config://{section}/{key}",
        "config",
        "Read config",
        None,
        |_state: &(), uri: &str, params: &matchit::Params| {
            let section = params.get("section").unwrap_or("?");
            let key = params.get("key").unwrap_or("?");
            ResourceContents::text(format!("config:{section}/{key}"), uri)
        },
    );

    // Resolve file URI
    let file_result = router.resolve(&(), "file:///main.rs");
    assert!(file_result.is_some());
    assert_eq!(extract_text(&file_result.unwrap()), "file:main.rs");

    // Resolve config URI
    let config_result = router.resolve(&(), "config://database/host");
    assert!(config_result.is_some());
    assert_eq!(
        extract_text(&config_result.unwrap()),
        "config:database/host"
    );
}

#[test]
fn template_with_multiple_params() {
    let mut router = ResourceRouter::<()>::new();
    router.add_resource(
        "/config/{section}/{key}",
        "config://{section}/{key}",
        "config",
        "Read a config value by section and key",
        None,
        |_state: &(), uri: &str, params: &matchit::Params| {
            let section = params.get("section").unwrap_or("?");
            let key = params.get("key").unwrap_or("?");
            ResourceContents::text(
                format!("Config [{section}] {key} = mock_value"),
                uri,
            )
        },
    );

    let result = router.resolve(&(), "config://database/host");
    assert!(result.is_some());
    assert_eq!(
        extract_text(&result.unwrap()),
        "Config [database] host = mock_value"
    );
}

#[test]
fn handler_receives_original_uri() {
    let mut router = ResourceRouter::<()>::new();
    router.add_resource(
        "/files/{path}",
        "file:///{path}",
        "file",
        "Read a file",
        None,
        |_state: &(), uri: &str, _params: &matchit::Params| {
            // Return the URI itself so we can verify it was passed correctly
            ResourceContents::text(uri.to_string(), uri)
        },
    );

    let result = router.resolve(&(), "file:///path.txt");
    assert!(result.is_some());
    assert_eq!(
        extract_text(&result.unwrap()),
        "file:///path.txt"
    );
}

#[test]
fn handler_with_state() {
    struct AppState {
        prefix: String,
    }

    let mut router = ResourceRouter::<AppState>::new();
    router.add_resource(
        "/files/{path}",
        "file:///{path}",
        "file",
        "Read a file",
        None,
        |state: &AppState, uri: &str, params: &matchit::Params| {
            let path = params.get("path").unwrap_or("?");
            ResourceContents::text(
                format!("{}: {path}", state.prefix),
                uri,
            )
        },
    );

    let state = AppState {
        prefix: "STATE".to_string(),
    };
    let result = router.resolve(&state, "file:///test.rs");
    assert!(result.is_some());
    assert_eq!(extract_text(&result.unwrap()), "STATE: test.rs");
}

#[test]
fn strip_uri_scheme_helper() {
    // file:/// URIs: the third slash is part of the absolute path
    assert_eq!(strip_uri_scheme("file:///README.md"), "/README.md");
    assert_eq!(strip_uri_scheme("config://database/host"), "database/host");
    assert_eq!(strip_uri_scheme("custom://some/path"), "some/path");
    assert_eq!(strip_uri_scheme("no-scheme"), "no-scheme");
}

#[test]
fn template_metadata() {
    let mut router = ResourceRouter::<()>::new();
    router.add_resource(
        "/files/{path}",
        "file:///{path}",
        "file",
        "Read a file by path",
        Some("text/plain"),
        |_state: &(), uri: &str, _params: &matchit::Params| {
            ResourceContents::text("".to_string(), uri)
        },
    );

    let templates = router.templates();
    assert_eq!(templates.len(), 1);

    let t = &templates[0];
    assert_eq!(t.raw.name, "file");
    assert_eq!(t.raw.uri_template, "file:///{path}");
    assert_eq!(
        t.raw.description.as_deref(),
        Some("Read a file by path")
    );
    assert_eq!(t.raw.mime_type.as_deref(), Some("text/plain"));
}

#[test]
fn chained_add_resource() {
    let mut router = ResourceRouter::<()>::new();
    let handler = |_state: &(), uri: &str, _params: &matchit::Params| {
        ResourceContents::text("ok".to_string(), uri)
    };

    // add_resource returns &mut Self, so we can chain
    router
        .add_resource("/a/{x}", "a:///{x}", "a", "A", None, handler)
        .add_resource("/b/{y}", "b:///{y}", "b", "B", None, handler);

    assert_eq!(router.templates().len(), 2);
}

#[test]
fn catch_all_route_for_deep_paths() {
    // matchit's {*path} catch-all syntax supports multi-segment paths
    let mut router = ResourceRouter::<()>::new();
    router.add_resource(
        "/files/{*path}",
        "file:///{*path}",
        "file",
        "Read a file by path (deep)",
        None,
        |_state: &(), uri: &str, params: &matchit::Params| {
            let path = params.get("path").unwrap_or("?");
            ResourceContents::text(format!("deep:{path}"), uri)
        },
    );

    let result = router.resolve(&(), "file:///src/main.rs");
    assert!(result.is_some());
    assert_eq!(extract_text(&result.unwrap()), "deep:src/main.rs");

    let result = router.resolve(&(), "file:///a/b/c/d.txt");
    assert!(result.is_some());
    assert_eq!(extract_text(&result.unwrap()), "deep:a/b/c/d.txt");
}
