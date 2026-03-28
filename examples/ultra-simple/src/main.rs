use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::schemars;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct AddParams {
    /// First number
    a: i32,
    /// Second number
    b: i32,
}

struct UltraSimple {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl UltraSimple {
    /// Add two numbers
    #[tool]
    fn add(&self, Parameters(params): Parameters<AddParams>) -> String {
        format!("{}", params.a + params.b)
    }
}

#[tool_handler]
impl ServerHandler for UltraSimple {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("ultra-simple", "0.1.0"))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = UltraSimple {
        tool_router: UltraSimple::tool_router(),
    };
    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
