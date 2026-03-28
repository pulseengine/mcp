//! # Hello World MCP Server with Authentication
//!
//! Demonstrates an rmcp MCP server wrapped in `pulseengine-security` middleware
//! via Axum's Streamable HTTP transport.
//!
//! ## Running
//!
//! ```bash
//! cargo run -p hello-world-with-auth
//! ```
//!
//! ## Testing
//!
//! ```bash
//! # The dev profile generates an API key — check the logs for it, then:
//! curl -H "Authorization: ApiKey <key>" http://127.0.0.1:8080/mcp
//! ```

use std::sync::Arc;

use axum::middleware::from_fn;
use axum::Router;
use pulseengine_security::SecurityConfig;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::info;
use tracing_subscriber::EnvFilter;

// ── Tool parameters ───────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct SayHelloParams {
    /// The name to greet (optional)
    name: Option<String>,
}

// ── MCP server ────────────────────────────────────────────────────

struct HelloWorldAuth {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl HelloWorldAuth {
    /// Say hello to someone (secured by pulseengine-security)
    #[tool]
    fn say_hello(&self, Parameters(params): Parameters<SayHelloParams>) -> String {
        let name = params.name.as_deref().unwrap_or("Authenticated World");
        format!("Hello, {name}!")
    }
}

#[tool_handler]
impl ServerHandler for HelloWorldAuth {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("hello-world-with-auth", "0.1.0"))
    }
}

// ── Main ──────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,hello_world_with_auth=debug")),
        )
        .init();

    // 1. Security setup — zero-config development profile
    let security = SecurityConfig::development();
    info!("Security profile: {:?}", security.profile);

    if let Some(ref key) = security.api_key {
        info!("Development API key: {key}");
        info!("Test with: curl -H 'Authorization: ApiKey {key}' http://127.0.0.1:8080/mcp");
    }

    let middleware = security.create_middleware().await?;

    // 2. Build the rmcp Streamable HTTP service
    let ct = tokio_util::sync::CancellationToken::new();

    let mcp_service = StreamableHttpService::new(
        || {
            Ok(HelloWorldAuth {
                tool_router: HelloWorldAuth::tool_router(),
            })
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    // 3. Mount MCP under /mcp, wrap the whole router with security middleware
    let app = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(from_fn(move |req, next| {
            let mw = middleware.clone();
            async move { mw.process(req, next).await }
        }));

    // 4. Serve
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("Listening on http://127.0.0.1:8080/mcp");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;

    Ok(())
}
