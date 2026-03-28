// PoC 1: Tower Auth Middleware with rmcp 1.3
//
// ADJUSTMENTS from the original starting code:
//
// 1. No `#[tool_handler]` on the `impl ServerHandler` block — it was missing in the
//    initial scaffold. Without it, `call_tool`/`list_tools`/`get_tool` get default
//    implementations that return "method not found" or empty lists.
//
// 2. `AuthContext` must derive `Clone` **and** be `Send + Sync + 'static` so it can
//    be stored in both `http::Extensions` and rmcp's `Extensions` type map.
//
// 3. The whoami tool uses `Extension<http::request::Parts>` as a parameter extractor
//    (documented in rmcp's `StreamableHttpService` docs) rather than manually reading
//    from `RequestContext.extensions`. The HTTP transport injects `Parts` automatically.
//    Inside `Parts.extensions` we find our `AuthContext` that the Tower middleware inserted.
//
// 4. The `AuthService` response type must be generic over the inner service's `Response`,
//    not hardcoded to `axum::body::Body`. When wrapping an axum `Router`, the inner
//    service returns `Response<axum::body::Body>`, so we produce 401 responses with the
//    same body type via `axum::body::Body::from(...)`.
//
// 5. Import paths: `rmcp::handler::server::tool::Extension` is the extractor type for
//    pulling values from `RequestContext.extensions`.
//
// 6. `ServerInfo::new(caps)` and `ServerCapabilities::builder().enable_tools().build()`
//    are correct as-is.
//
// 7. `CallToolResult::success(vec![Content::text(...)])` is correct as-is.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::Router;
use http::{Request, Response, StatusCode};
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Extension, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
        session::local::LocalSessionManager,
    },
    ServerHandler,
};
use tower::{Layer, Service};
use tracing_subscriber::EnvFilter;

// --- Auth types ---

#[derive(Clone, Debug)]
struct AuthContext {
    user: String,
    role: String,
}

// --- Tower middleware ---

#[derive(Clone)]
struct AuthLayer {
    token: String,
}

impl AuthLayer {
    fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            token: self.token.clone(),
        }
    }
}

#[derive(Clone)]
struct AuthService<S> {
    inner: S,
    token: String,
}

impl<S, B> Service<Request<B>> for AuthService<S>
where
    S: Service<Request<B>, Response = Response<axum::body::Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let expected = format!("Bearer {}", self.token);
        let auth_header = req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        if auth_header.as_deref() != Some(&expected) {
            return Box::pin(async {
                Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(axum::body::Body::from("Unauthorized"))
                    .unwrap())
            });
        }

        // Inject AuthContext into http::Extensions so it propagates through
        // rmcp's StreamableHttpService into the tool handler via Parts.extensions.
        req.extensions_mut().insert(AuthContext {
            user: "admin".to_string(),
            role: "operator".to_string(),
        });

        let mut svc = self.inner.clone();
        Box::pin(async move { svc.call(req).await })
    }
}

// --- MCP tool handler ---

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct WhoamiParams {}

#[derive(Debug, Clone)]
struct AuthDemo {
    tool_router: ToolRouter<Self>,
}

impl AuthDemo {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl AuthDemo {
    /// Returns the authenticated user and role from the Tower auth middleware.
    #[tool(description = "Returns the authenticated user and role")]
    fn whoami(
        &self,
        _params: Parameters<WhoamiParams>,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let auth = parts.extensions.get::<AuthContext>().cloned();
        match auth {
            Some(ctx) => Ok(CallToolResult::success(vec![Content::text(format!(
                "user={}, role={}",
                ctx.user, ctx.role
            ))])),
            None => Ok(CallToolResult::success(vec![Content::text(
                "no auth context available",
            )])),
        }
    }
}

#[tool_handler]
impl ServerHandler for AuthDemo {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

// --- main ---

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let ct = tokio_util::sync::CancellationToken::new();

    let mcp_service = StreamableHttpService::new(
        || Ok(AuthDemo::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    let app = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(AuthLayer::new("secret-token"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("PoC 1: Tower Auth — listening on http://127.0.0.1:8080/mcp");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;

    Ok(())
}
