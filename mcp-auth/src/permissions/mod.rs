//! Permission system for tools and resources
//!
//! This module provides fine-grained permission control for operations,
//! including tools, resources, and custom permission definitions.

pub mod mcp_permissions;

pub use mcp_permissions::{
    Permission, PermissionAction, PermissionChecker, PermissionConfig, PermissionError,
    PermissionRule, ResourcePermissionConfig, ToolPermissionConfig,
};
