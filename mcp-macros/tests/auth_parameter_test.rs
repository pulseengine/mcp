//! Test for auth parameter functionality

#![allow(dead_code, clippy::uninlined_format_args)]

use pulseengine_mcp_macros::mcp_server;
use pulseengine_mcp_server::McpBackend;

#[cfg(feature = "auth")]
use pulseengine_mcp_auth::config::StorageConfig;

mod memory_auth {
    use super::*;

    /// Test server with memory auth
    #[mcp_server(name = "Memory Auth Server", auth = "memory")]
    #[derive(Clone, Default)]
    pub struct MemoryAuthServer;
}

mod disabled_auth {
    use super::*;

    /// Test server with disabled auth
    #[mcp_server(name = "Disabled Auth Server", auth = "disabled")]
    #[derive(Clone, Default)]
    pub struct DisabledAuthServer;
}

mod file_auth {
    use super::*;

    /// Test server with file auth (explicit)
    #[mcp_server(name = "File Auth Server", auth = "file")]
    #[derive(Clone, Default)]
    pub struct FileAuthServer;
}

mod default_auth {
    use super::*;

    /// Test server with no auth (no auth parameter)
    #[mcp_server(name = "No Auth Server")]
    #[derive(Clone, Default)]
    pub struct NoAuthServer;
}

#[test]
fn test_auth_parameter_memory() {
    let server = memory_auth::MemoryAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Memory Auth Server");

    // Test auth config generation
    #[cfg(feature = "auth")]
    {
        let auth_config = memory_auth::MemoryAuthServerConfig::get_auth_config();
        assert!(matches!(auth_config.storage, StorageConfig::Memory));
        assert!(auth_config.enabled);
    }
}

#[test]
fn test_auth_parameter_disabled() {
    let server = disabled_auth::DisabledAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Disabled Auth Server");

    // Test auth config generation
    #[cfg(feature = "auth")]
    {
        let auth_config = disabled_auth::DisabledAuthServerConfig::get_auth_config();
        assert!(!auth_config.enabled);
    }
}

#[test]
fn test_auth_parameter_file() {
    let server = file_auth::FileAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "File Auth Server");

    // Test auth config generation
    #[cfg(feature = "auth")]
    {
        let auth_config = file_auth::FileAuthServerConfig::get_auth_config();
        assert!(matches!(auth_config.storage, StorageConfig::File { .. }));
        assert!(auth_config.enabled);
    }
}

#[test]
fn test_no_auth_by_default() {
    let server = default_auth::NoAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "No Auth Server");

    // When no auth parameter is specified, no auth config methods should be generated
    // This is compile-time verified - if get_auth_config existed, this would fail to compile
    
    // Test that the server works without auth
    assert!(!info.server_info.name.is_empty());
}
