//! Permission system for MCP tools and resources
//!
//! This module provides fine-grained permission control for MCP operations,
//! including tools, resources, and custom permission definitions.

pub mod mcp_permissions;

pub use mcp_permissions::{
    McpPermission, McpPermissionChecker, PermissionConfig, PermissionError,
    ToolPermissionConfig, ResourcePermissionConfig, PermissionRule, PermissionAction
};