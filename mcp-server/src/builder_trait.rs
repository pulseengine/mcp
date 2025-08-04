//! Builder trait for reducing generated code in server implementations
//!
//! This module provides a trait-based builder pattern that eliminates the need
//! to generate repetitive builder methods for each server.

use crate::{McpServer, ServerConfig, McpBackend};
use pulseengine_mcp_transport::{StdioTransport, Transport};
use std::future::Future;

/// Builder trait that provides common serve methods for all MCP servers
pub trait McpServerBuilder: McpBackend + Sized {
    /// Create server with default configuration
    fn with_defaults() -> ServerBuilder<Self> {
        ServerBuilder {
            config: ServerConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Configure stdio logging to redirect logs to stderr
    fn configure_stdio_logging() {
        #[cfg(feature = "stdio-logging")]
        {
            use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
            
            let filter_layer = EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap();
                
            let fmt_layer = fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_thread_ids(false)
                .with_thread_names(false);
                
            tracing_subscriber::registry()
                .with(filter_layer)
                .with(fmt_layer)
                .init();
        }
        
        #[cfg(not(feature = "stdio-logging"))]
        {
            eprintln!("Warning: stdio logging configuration requires 'stdio-logging' feature");
            eprintln!("Add to Cargo.toml: pulseengine-mcp-macros = {{ version = \"*\", features = [\"stdio-logging\"] }}");
            eprintln!("And: tracing-subscriber = \"0.3\"");
        }
    }
}

/// Builder for configuring MCP servers
pub struct ServerBuilder<B: McpBackend> {
    config: ServerConfig,
    _phantom: std::marker::PhantomData<B>,
}

impl<B: McpBackend> ServerBuilder<B> {
    /// Serve using STDIO transport
    pub async fn serve_stdio(self) -> Result<McpService<B>, B::Error> 
    where 
        B::Error: From<crate::ServerError>
    {
        let backend = B::initialize(B::Config::default()).await?;
        let transport = StdioTransport::new();
        let server = McpServer::new(backend, self.config)?;
        
        Ok(McpService {
            backend: backend.clone(),
            server,
            transport: Box::new(transport),
        })
    }
    
    /// Serve using HTTP transport
    #[cfg(feature = "http")]
    pub async fn serve_http(self, addr: std::net::SocketAddr) -> Result<McpService<B>, B::Error>
    where 
        B::Error: From<crate::ServerError>
    {
        let backend = B::initialize(B::Config::default()).await?;
        let transport = pulseengine_mcp_transport::HttpTransport::new(addr);
        let server = McpServer::new(backend, self.config)?;
        
        Ok(McpService {
            backend: backend.clone(),
            server,
            transport: Box::new(transport),
        })
    }
    
    /// Serve using WebSocket transport
    #[cfg(feature = "websocket")]
    pub async fn serve_websocket(self, addr: std::net::SocketAddr) -> Result<McpService<B>, B::Error>
    where 
        B::Error: From<crate::ServerError>
    {
        let backend = B::initialize(B::Config::default()).await?;
        let transport = pulseengine_mcp_transport::WebSocketTransport::new(addr);
        let server = McpServer::new(backend, self.config)?;
        
        Ok(McpService {
            backend: backend.clone(),
            server,
            transport: Box::new(transport),
        })
    }
    
    /// Serve with custom configuration
    pub async fn serve_with_config(
        self,
        config: B::Config,
        transport: Box<dyn Transport>,
    ) -> Result<McpService<B>, B::Error>
    where 
        B::Error: From<crate::ServerError>
    {
        let backend = B::initialize(config).await?;
        let server = McpServer::new(backend, self.config)?;
        
        Ok(McpService {
            server,
            transport,
        })
    }
}

/// Service wrapper that provides lifecycle methods
pub struct McpService<B: McpBackend> {
    backend: B,
    server: McpServer<B>,
    transport: Box<dyn Transport>,
}

impl<B: McpBackend> McpService<B> {
    /// Run the server
    pub async fn run(self) -> Result<(), B::Error>
    where 
        B::Error: From<crate::ServerError>
    {
        self.server.serve(self.transport).await?;
        Ok(())
    }
    
    /// Run with custom shutdown signal
    pub async fn run_with_shutdown<F>(self, shutdown: F) -> Result<(), B::Error>
    where
        F: Future<Output = ()> + Send + 'static,
        B::Error: From<crate::ServerError>
    {
        self.server.serve_with_shutdown(self.transport, shutdown).await?;
        Ok(())
    }
    
    /// Get a reference to the backend
    pub fn backend(&self) -> &B {
        &self.backend
    }
    
    /// Get server info
    pub fn server_info(&self) -> pulseengine_mcp_protocol::ServerInfo {
        self.backend().get_server_info()
    }
}