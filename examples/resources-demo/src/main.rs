//! # MCP Resources Demo
//!
//! Demonstrates how to use `pulseengine-mcp-resources` with rmcp to build
//! a server that combines tools (via `#[tool]`) with URI-template-based
//! resources (via `ResourceRouter`).
//!
//! ## Key Concepts
//!
//! 1. **Tools** are defined with `#[tool]` inside a `#[tool_router]` impl block.
//! 2. **Resources** are registered on a `ResourceRouter` with URI template patterns.
//! 3. The `ServerHandler` trait overrides `list_resource_templates` and `read_resource`
//!    to wire the router into the MCP protocol.
//! 4. Transport is stdio — connect with MCP Inspector or Claude Desktop.

use std::collections::HashMap;
use std::sync::Arc;

use pulseengine_mcp_resources::ResourceRouter;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    Implementation, ListResourceTemplatesResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

// ── State ─────────────────────────────────────────────────────────

/// Shared server state: an in-memory key/value store.
#[derive(Clone)]
struct State {
    data: HashMap<String, String>,
}

impl State {
    fn new() -> Self {
        let mut data = HashMap::new();
        data.insert(
            "1".to_string(),
            r#"{"id":"1","name":"Alice","role":"admin"}"#.to_string(),
        );
        data.insert(
            "2".to_string(),
            r#"{"id":"2","name":"Bob","role":"user"}"#.to_string(),
        );
        data.insert(
            "app".to_string(),
            r#"{"theme":"dark","language":"en"}"#.to_string(),
        );
        Self { data }
    }
}

// ── Tool params ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListKeysParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct InfoParams {}

// ── Response types (used by resources) ────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    role: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    theme: String,
    language: String,
}

// ── Server ────────────────────────────────────────────────────────

struct ResourcesDemo {
    tool_router: ToolRouter<Self>,
    state: Arc<State>,
    resources: Arc<ResourceRouter<State>>,
}

impl ResourcesDemo {
    fn new() -> Self {
        let state = State::new();

        let mut router = ResourceRouter::<State>::new();

        // user://{user_id} — look up a user by ID
        router.add_resource(
            "/users/{user_id}",
            "user://{user_id}",
            "user",
            "Get user data by ID",
            Some("application/json"),
            |state: &State, uri: &str, params: &matchit::Params| {
                let user_id = params.get("user_id").unwrap_or("unknown");
                match state.data.get(user_id) {
                    Some(json) => ResourceContents::text(json.clone(), uri),
                    None => ResourceContents::text(
                        format!(r#"{{"error":"User not found: {user_id}"}}"#),
                        uri,
                    ),
                }
            },
        );

        // config://{config_name} — look up a config section
        router.add_resource(
            "/config/{config_name}",
            "config://{config_name}",
            "config",
            "Get configuration settings",
            Some("application/json"),
            |state: &State, uri: &str, params: &matchit::Params| {
                let name = params.get("config_name").unwrap_or("unknown");
                match state.data.get(name) {
                    Some(json) => ResourceContents::text(json.clone(), uri),
                    None => ResourceContents::text(
                        format!(r#"{{"error":"Config not found: {name}"}}"#),
                        uri,
                    ),
                }
            },
        );

        // data://{key} — generic key lookup with metadata
        router.add_resource(
            "/data/{key}",
            "data://{key}",
            "data",
            "Get any data by key",
            Some("application/json"),
            |state: &State, uri: &str, params: &matchit::Params| {
                let key = params.get("key").unwrap_or("unknown");
                let exists = state.data.contains_key(key);
                let preview = state.data.get(key).map(|v| {
                    if v.len() > 50 {
                        format!("{}...", &v[..50])
                    } else {
                        v.clone()
                    }
                });
                let json = serde_json::json!({
                    "key": key,
                    "exists": exists,
                    "value_preview": preview,
                });
                ResourceContents::text(json.to_string(), uri)
            },
        );

        Self {
            tool_router: Self::tool_router(),
            state: Arc::new(state),
            resources: Arc::new(router),
        }
    }
}

// ── Tools ─────────────────────────────────────────────────────────

#[tool_router]
impl ResourcesDemo {
    /// List all available data keys in the store
    #[tool]
    fn list_keys(&self, _params: Parameters<ListKeysParams>) -> String {
        let keys: Vec<&String> = self.state.data.keys().collect();
        serde_json::to_string_pretty(&keys).unwrap_or_default()
    }

    /// Get information about how many items are stored
    #[tool]
    fn info(&self, _params: Parameters<InfoParams>) -> String {
        format!("Storing {} items", self.state.data.len())
    }
}

// ── ServerHandler (tools + resources) ─────────────────────────────

#[tool_handler]
impl ServerHandler for ResourcesDemo {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("resources-demo", "0.1.0"))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult {
            resource_templates: self.resources.templates(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = &request.uri;
        match self.resources.resolve(&self.state, uri) {
            Some(contents) => Ok(ReadResourceResult::new(vec![contents])),
            None => Err(ErrorData::resource_not_found(
                format!("No resource matches URI: {uri}"),
                None,
            )),
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let server = ResourcesDemo::new();

    tracing::info!("Resources Demo: starting stdio MCP server");
    tracing::info!(
        templates = server.resources.templates().len(),
        "Registered resource templates"
    );

    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
