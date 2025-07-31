//! Example demonstrating different auth configurations with #[mcp_server]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpBackend;

mod dev {
    use super::*;

    /// Development server with in-memory auth (easy setup, no persistence)
    #[mcp_server(name = "Development Server", auth = "memory")]
    #[derive(Clone, Default)]
    pub struct DevServer;

    #[mcp_tools]
    impl DevServer {
        /// A simple test tool
        pub fn hello(&self, name: Option<String>) -> String {
            format!("Hello, {}!", name.unwrap_or_else(|| "World".to_string()))
        }
    }
}

mod test {
    use super::*;

    /// Production server with disabled auth (for testing or low-security environments)
    #[mcp_server(name = "Testing Server", auth = "disabled")]
    #[derive(Clone, Default)]
    pub struct TestServer;

    #[mcp_tools]
    impl TestServer {
        /// Another test tool
        pub fn ping(&self) -> String {
            "pong".to_string()
        }
    }
}

mod prod {
    use super::*;

    /// Production server with file-based auth (secure persistence)
    #[mcp_server(name = "Production Server", auth = "file", app_name = "my-app")]
    #[derive(Clone, Default)]
    pub struct ProdServer;

    #[mcp_tools]
    impl ProdServer {
        /// A secure tool that requires authentication
        pub fn secure_operation(&self, data: String) -> String {
            format!("Processed: {}", data)
        }
    }
}

mod custom {
    use super::*;

    /// Server with custom auth configuration
    #[mcp_server(
        name = "Custom Auth Server",
        auth = "pulseengine_mcp_auth::AuthConfig::with_custom_path(\"my-app\", std::path::PathBuf::from(\"/custom/path\"))"
    )]
    #[derive(Clone, Default)]
    pub struct CustomAuthServer;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Auth Configuration Examples");
    println!("==========================");

    // Development setup - memory auth for easy testing
    println!("\n1. Development Server (memory auth):");
    let dev_server = dev::DevServer::with_defaults();
    println!(
        "   Server: {}",
        dev_server.get_server_info().server_info.name
    );

    #[cfg(feature = "auth")]
    {
        let auth_config = dev::DevServerConfig::get_auth_config();
        println!("   Auth enabled: {}", auth_config.enabled);
        println!("   Storage: Memory (no persistence)");
    }

    // Test server - disabled auth
    println!("\n2. Test Server (disabled auth):");
    let test_server = test::TestServer::with_defaults();
    println!(
        "   Server: {}",
        test_server.get_server_info().server_info.name
    );

    #[cfg(feature = "auth")]
    {
        let auth_config = test::TestServerConfig::get_auth_config();
        println!("   Auth enabled: {}", auth_config.enabled);
    }

    // Production server - file auth with app name
    println!("\n3. Production Server (file auth with app isolation):");
    let prod_server = prod::ProdServer::with_defaults();
    println!(
        "   Server: {}",
        prod_server.get_server_info().server_info.name
    );

    #[cfg(feature = "auth")]
    {
        let auth_config = prod::ProdServerConfig::get_auth_config();
        println!("   Auth enabled: {}", auth_config.enabled);
        if let pulseengine_mcp_auth::StorageConfig::File { path, .. } = &auth_config.storage {
            println!("   Storage: File at {:?}", path);
        }
    }

    // Custom auth server
    println!("\n4. Custom Auth Server (custom path):");
    let custom_server = custom::CustomAuthServer::with_defaults();
    println!(
        "   Server: {}",
        custom_server.get_server_info().server_info.name
    );

    #[cfg(feature = "auth")]
    {
        let auth_config = custom::CustomAuthServerConfig::get_auth_config();
        println!("   Auth enabled: {}", auth_config.enabled);
        if let pulseengine_mcp_auth::StorageConfig::File { path, .. } = &auth_config.storage {
            println!("   Storage: File at {:?}", path);
        }
    }

    println!("\nUsage Examples:");
    println!("==============");
    println!("// Memory auth - perfect for development");
    println!("#[mcp_server(name = \"Dev Server\", auth = \"memory\")]");
    println!("");
    println!("// Disabled auth - for testing or low-security environments");
    println!("#[mcp_server(name = \"Test Server\", auth = \"disabled\")]");
    println!("");
    println!("// File auth - secure persistence (production)");
    println!("#[mcp_server(name = \"Prod Server\", auth = \"file\", app_name = \"my-app\")]");
    println!("");
    println!("// Custom auth configuration - full control");
    println!(
        "#[mcp_server(name = \"Custom\", auth = \"pulseengine_mcp_auth::AuthConfig::memory()\")]"
    );

    Ok(())
}
