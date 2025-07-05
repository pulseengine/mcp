//! Session Management Module
//!
//! This module provides comprehensive session management for MCP authentication
//! including JWT tokens, session storage, and lifecycle management.

pub mod session_manager;

pub use session_manager::{
    MemorySessionStorage, Session, SessionConfig, SessionError, SessionManager, SessionStats,
    SessionStorage,
};
