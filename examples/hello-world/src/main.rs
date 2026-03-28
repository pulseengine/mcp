use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct SayHelloParams {
    /// The name to greet (optional)
    name: Option<String>,
}

struct HelloWorld {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl HelloWorld {
    /// Say hello to someone
    #[tool]
    async fn say_hello(&self, Parameters(params): Parameters<SayHelloParams>) -> String {
        let name = params.name.unwrap_or_else(|| "World".into());
        format!("Hello, {name}!")
    }
}

#[tool_handler]
impl ServerHandler for HelloWorld {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("hello-world", "0.1.0"))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let server = HelloWorld {
        tool_router: HelloWorld::tool_router(),
    };
    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
