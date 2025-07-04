//! MCP Permission System
//!
//! This module provides comprehensive permission management for MCP tools,
//! resources, and custom operations with role-based access control.

use crate::{AuthContext, models::Role};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use tracing::debug;

/// Errors that can occur during permission checking
#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Permission not found: {0}")]
    NotFound(String),
    
    #[error("Invalid permission format: {0}")]
    InvalidFormat(String),
    
    #[error("Role configuration error: {0}")]
    RoleConfig(String),
}

/// MCP-specific permission types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpPermission {
    /// Permission to use a specific tool
    UseTool(String),
    
    /// Permission to access a specific resource
    UseResource(String),
    
    /// Permission to use tools in a category
    UseToolCategory(String),
    
    /// Permission to access resources in a category
    UseResourceCategory(String),
    
    /// Permission to use prompts
    UsePrompt(String),
    
    /// Permission to subscribe to resources
    Subscribe(String),
    
    /// Permission to perform completion operations
    Complete,
    
    /// Permission to change log levels
    SetLogLevel,
    
    /// Administrative permissions
    Admin(String),
    
    /// Custom permission
    Custom(String),
}

impl McpPermission {
    /// Create a tool permission from a tool name
    pub fn tool(name: &str) -> Self {
        Self::UseTool(name.to_string())
    }
    
    /// Create a resource permission from a resource URI
    pub fn resource(uri: &str) -> Self {
        Self::UseResource(uri.to_string())
    }
    
    /// Create a tool category permission
    pub fn tool_category(category: &str) -> Self {
        Self::UseToolCategory(category.to_string())
    }
    
    /// Create a resource category permission
    pub fn resource_category(category: &str) -> Self {
        Self::UseResourceCategory(category.to_string())
    }
    
    /// Get a string representation of the permission
    pub fn to_string(&self) -> String {
        match self {
            Self::UseTool(name) => format!("tool:{}", name),
            Self::UseResource(uri) => format!("resource:{}", uri),
            Self::UseToolCategory(cat) => format!("tool_category:{}", cat),
            Self::UseResourceCategory(cat) => format!("resource_category:{}", cat),
            Self::UsePrompt(name) => format!("prompt:{}", name),
            Self::Subscribe(resource) => format!("subscribe:{}", resource),
            Self::Complete => "complete".to_string(),
            Self::SetLogLevel => "set_log_level".to_string(),
            Self::Admin(action) => format!("admin:{}", action),
            Self::Custom(perm) => format!("custom:{}", perm),
        }
    }
    
    /// Parse a permission from a string
    pub fn from_string(s: &str) -> Result<Self, PermissionError> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        match parts.as_slice() {
            ["tool", name] => Ok(Self::UseTool(name.to_string())),
            ["resource", uri] => Ok(Self::UseResource(uri.to_string())),
            ["tool_category", cat] => Ok(Self::UseToolCategory(cat.to_string())),
            ["resource_category", cat] => Ok(Self::UseResourceCategory(cat.to_string())),
            ["prompt", name] => Ok(Self::UsePrompt(name.to_string())),
            ["subscribe", resource] => Ok(Self::Subscribe(resource.to_string())),
            ["complete"] => Ok(Self::Complete),
            ["set_log_level"] => Ok(Self::SetLogLevel),
            ["admin", action] => Ok(Self::Admin(action.to_string())),
            ["custom", perm] => Ok(Self::Custom(perm.to_string())),
            _ => Err(PermissionError::InvalidFormat(format!("Invalid permission format: {}", s))),
        }
    }
}

/// Permission action (allow or deny)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionAction {
    Allow,
    Deny,
}

impl Default for PermissionAction {
    fn default() -> Self {
        Self::Deny
    }
}

/// Permission rule that defines access for specific roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// The permission this rule applies to
    pub permission: McpPermission,
    
    /// Roles this rule applies to
    pub roles: Vec<Role>,
    
    /// Action to take (allow or deny)
    pub action: PermissionAction,
    
    /// Optional conditions (for future expansion)
    pub conditions: Option<HashMap<String, String>>,
}

impl PermissionRule {
    /// Create a new allow rule
    pub fn allow(permission: McpPermission, roles: Vec<Role>) -> Self {
        Self {
            permission,
            roles,
            action: PermissionAction::Allow,
            conditions: None,
        }
    }
    
    /// Create a new deny rule
    pub fn deny(permission: McpPermission, roles: Vec<Role>) -> Self {
        Self {
            permission,
            roles,
            action: PermissionAction::Deny,
            conditions: None,
        }
    }
    
    /// Check if this rule applies to a given role
    pub fn applies_to_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }
}

/// Configuration for tool permissions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolPermissionConfig {
    /// Default permission for tools (allow or deny)
    pub default_action: PermissionAction,
    
    /// Specific tool permissions
    pub tool_permissions: HashMap<String, Vec<Role>>,
    
    /// Tool category permissions
    pub category_permissions: HashMap<String, Vec<Role>>,
    
    /// Tools that require admin access
    pub admin_only_tools: HashSet<String>,
    
    /// Tools that are read-only (allowed for monitor role)
    pub read_only_tools: HashSet<String>,
}

/// Configuration for resource permissions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcePermissionConfig {
    /// Default permission for resources (allow or deny)
    pub default_action: PermissionAction,
    
    /// Specific resource permissions by URI pattern
    pub resource_permissions: HashMap<String, Vec<Role>>,
    
    /// Resource category permissions
    pub category_permissions: HashMap<String, Vec<Role>>,
    
    /// Resources that require admin access
    pub admin_only_resources: HashSet<String>,
    
    /// Resources that are always public
    pub public_resources: HashSet<String>,
}

/// Main permission configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Tool permission configuration
    pub tools: ToolPermissionConfig,
    
    /// Resource permission configuration
    pub resources: ResourcePermissionConfig,
    
    /// Custom permission rules
    pub custom_rules: Vec<PermissionRule>,
    
    /// Enable strict permission checking
    pub strict_mode: bool,
    
    /// Default action when no rule matches
    pub default_action: PermissionAction,
}

impl PermissionConfig {
    /// Create a permissive configuration (allows most operations)
    pub fn permissive() -> Self {
        Self {
            tools: ToolPermissionConfig {
                default_action: PermissionAction::Allow,
                ..Default::default()
            },
            resources: ResourcePermissionConfig {
                default_action: PermissionAction::Allow,
                ..Default::default()
            },
            strict_mode: false,
            default_action: PermissionAction::Allow,
            ..Default::default()
        }
    }
    
    /// Create a restrictive configuration (denies by default)
    pub fn restrictive() -> Self {
        Self {
            tools: ToolPermissionConfig {
                default_action: PermissionAction::Deny,
                ..Default::default()
            },
            resources: ResourcePermissionConfig {
                default_action: PermissionAction::Deny,
                ..Default::default()
            },
            strict_mode: true,
            default_action: PermissionAction::Deny,
            ..Default::default()
        }
    }
    
    /// Create a standard production configuration
    pub fn production() -> Self {
        let mut config = Self::restrictive();
        
        // Allow common read-only operations for Monitor role
        config.tools.read_only_tools.extend([
            "ping".to_string(),
            "health_check".to_string(),
            "get_status".to_string(),
            "list_devices".to_string(),
        ]);
        
        // Allow public resources
        config.resources.public_resources.extend([
            "system://status".to_string(),
            "system://health".to_string(),
            "system://version".to_string(),
        ]);
        
        config
    }
    
    /// Builder pattern for adding tool permissions
    pub fn allow_role_tool(mut self, role: Role, tool: &str) -> Self {
        self.tools.tool_permissions
            .entry(tool.to_string())
            .or_insert_with(Vec::new)
            .push(role);
        self
    }
    
    /// Builder pattern for adding resource permissions
    pub fn allow_role_resource(mut self, role: Role, resource: &str) -> Self {
        self.resources.resource_permissions
            .entry(resource.to_string())
            .or_insert_with(Vec::new)
            .push(role);
        self
    }
    
    /// Builder pattern for denying resource access
    pub fn deny_role_resource(mut self, role: Role, resource: &str) -> Self {
        let rule = PermissionRule::deny(
            McpPermission::UseResource(resource.to_string()), 
            vec![role]
        );
        self.custom_rules.push(rule);
        self
    }
}

/// MCP Permission Checker
pub struct McpPermissionChecker {
    config: PermissionConfig,
}

impl McpPermissionChecker {
    /// Create a new permission checker
    pub fn new(config: PermissionConfig) -> Self {
        Self { config }
    }
    
    /// Check if a user can use a specific tool
    pub fn can_use_tool(&self, auth_context: &AuthContext, tool_name: &str) -> bool {
        debug!("Checking tool permission: {} for roles: {:?}", tool_name, auth_context.roles);
        
        // Check custom rules first
        for rule in &self.config.custom_rules {
            if let McpPermission::UseTool(rule_tool) = &rule.permission {
                if rule_tool == tool_name {
                    for role in &auth_context.roles {
                        if rule.applies_to_role(role) {
                            match rule.action {
                                PermissionAction::Allow => return true,
                                PermissionAction::Deny => return false,
                            }
                        }
                    }
                }
            }
        }
        
        // Check if tool requires admin access
        if self.config.tools.admin_only_tools.contains(tool_name) {
            return auth_context.roles.contains(&Role::Admin);
        }
        
        // Check if tool is read-only (monitor role allowed)
        if self.config.tools.read_only_tools.contains(tool_name) {
            return auth_context.roles.iter().any(|role| {
                matches!(role, Role::Admin | Role::Operator | Role::Monitor)
            });
        }
        
        // Check specific tool permissions
        if let Some(allowed_roles) = self.config.tools.tool_permissions.get(tool_name) {
            return auth_context.roles.iter().any(|role| allowed_roles.contains(role));
        }
        
        // Check tool category permissions
        if let Some(category) = self.extract_tool_category(tool_name) {
            if let Some(allowed_roles) = self.config.tools.category_permissions.get(&category) {
                return auth_context.roles.iter().any(|role| allowed_roles.contains(role));
            }
        }
        
        // Fall back to default action
        match self.config.tools.default_action {
            PermissionAction::Allow => true,
            PermissionAction::Deny => false,
        }
    }
    
    /// Check if a user can access a specific resource
    pub fn can_access_resource(&self, auth_context: &AuthContext, resource_uri: &str) -> bool {
        debug!("Checking resource permission: {} for roles: {:?}", resource_uri, auth_context.roles);
        
        // Check custom rules first
        for rule in &self.config.custom_rules {
            if let McpPermission::UseResource(rule_resource) = &rule.permission {
                if self.matches_resource_pattern(rule_resource, resource_uri) {
                    for role in &auth_context.roles {
                        if rule.applies_to_role(role) {
                            match rule.action {
                                PermissionAction::Allow => return true,
                                PermissionAction::Deny => return false,
                            }
                        }
                    }
                }
            }
        }
        
        // Check if resource is public
        if self.config.resources.public_resources.contains(resource_uri) {
            return true;
        }
        
        // Check if resource requires admin access
        if self.config.resources.admin_only_resources.contains(resource_uri) {
            return auth_context.roles.contains(&Role::Admin);
        }
        
        // Check specific resource permissions
        for (pattern, allowed_roles) in &self.config.resources.resource_permissions {
            if self.matches_resource_pattern(pattern, resource_uri) {
                return auth_context.roles.iter().any(|role| allowed_roles.contains(role));
            }
        }
        
        // Check resource category permissions
        if let Some(category) = self.extract_resource_category(resource_uri) {
            if let Some(allowed_roles) = self.config.resources.category_permissions.get(&category) {
                return auth_context.roles.iter().any(|role| allowed_roles.contains(role));
            }
        }
        
        // Fall back to default action
        match self.config.resources.default_action {
            PermissionAction::Allow => true,
            PermissionAction::Deny => false,
        }
    }
    
    /// Check if a user can use a specific prompt
    pub fn can_use_prompt(&self, auth_context: &AuthContext, prompt_name: &str) -> bool {
        // For now, prompts follow the same rules as tools
        self.can_use_tool(auth_context, prompt_name)
    }
    
    /// Check if a user can subscribe to a resource
    pub fn can_subscribe(&self, auth_context: &AuthContext, resource_uri: &str) -> bool {
        // Subscription requires both resource access and subscription permission
        if !self.can_access_resource(auth_context, resource_uri) {
            return false;
        }
        
        // Check for subscription-specific rules
        for rule in &self.config.custom_rules {
            if let McpPermission::Subscribe(rule_resource) = &rule.permission {
                if self.matches_resource_pattern(rule_resource, resource_uri) {
                    for role in &auth_context.roles {
                        if rule.applies_to_role(role) {
                            match rule.action {
                                PermissionAction::Allow => return true,
                                PermissionAction::Deny => return false,
                            }
                        }
                    }
                }
            }
        }
        
        // Default: if you can access the resource, you can subscribe
        true
    }
    
    /// Check method-level permissions
    pub fn can_use_method(&self, auth_context: &AuthContext, method: &str) -> bool {
        match method {
            "tools/call" => {
                // Will be checked per-tool in can_use_tool
                true
            }
            "resources/read" | "resources/list" => {
                // Will be checked per-resource in can_access_resource
                true
            }
            "resources/subscribe" | "resources/unsubscribe" => {
                // Subscription requires at least operator role
                auth_context.roles.iter().any(|role| {
                    matches!(role, Role::Admin | Role::Operator)
                })
            }
            "completion/complete" => {
                // Custom rules for completion
                for rule in &self.config.custom_rules {
                    if matches!(rule.permission, McpPermission::Complete) {
                        for role in &auth_context.roles {
                            if rule.applies_to_role(role) {
                                return matches!(rule.action, PermissionAction::Allow);
                            }
                        }
                    }
                }
                // Default: allow for admin and operator
                auth_context.roles.iter().any(|role| {
                    matches!(role, Role::Admin | Role::Operator)
                })
            }
            "logging/setLevel" => {
                // Only admin can change log levels
                auth_context.roles.contains(&Role::Admin)
            }
            "initialize" | "ping" => {
                // Always allowed
                true
            }
            _ => {
                // Unknown method - use default action
                matches!(self.config.default_action, PermissionAction::Allow)
            }
        }
    }
    
    /// Extract tool category from tool name
    fn extract_tool_category(&self, tool_name: &str) -> Option<String> {
        // Common patterns for tool categorization
        if tool_name.starts_with("control_") {
            Some("control".to_string())
        } else if tool_name.starts_with("get_") || tool_name.starts_with("list_") {
            Some("read".to_string())
        } else if tool_name.starts_with("set_") || tool_name.starts_with("update_") {
            Some("write".to_string())
        } else if tool_name.contains("_lights") || tool_name.contains("lighting") {
            Some("lighting".to_string())
        } else if tool_name.contains("_climate") || tool_name.contains("temperature") {
            Some("climate".to_string())
        } else if tool_name.contains("_security") || tool_name.contains("alarm") {
            Some("security".to_string())
        } else if tool_name.contains("_audio") || tool_name.contains("volume") {
            Some("audio".to_string())
        } else {
            None
        }
    }
    
    /// Extract resource category from URI
    fn extract_resource_category(&self, resource_uri: &str) -> Option<String> {
        // Parse scheme://category/... pattern
        if let Some(scheme_pos) = resource_uri.find("://") {
            let after_scheme = &resource_uri[scheme_pos + 3..];
            if let Some(slash_pos) = after_scheme.find('/') {
                Some(after_scheme[..slash_pos].to_string())
            } else {
                Some(after_scheme.to_string())
            }
        } else {
            None
        }
    }
    
    /// Check if a resource pattern matches a URI
    fn matches_resource_pattern(&self, pattern: &str, uri: &str) -> bool {
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            uri.starts_with(prefix)
        } else {
            pattern == uri
        }
    }
    
    /// Validate permission configuration
    pub fn validate_config(&self) -> Result<(), PermissionError> {
        // Check for conflicting rules
        for rule in &self.config.custom_rules {
            if rule.roles.is_empty() {
                return Err(PermissionError::RoleConfig(
                    "Permission rule must specify at least one role".to_string(),
                ));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_permission_string_conversion() {
        let perm = McpPermission::tool("control_device");
        assert_eq!(perm.to_string(), "tool:control_device");
        
        let parsed = McpPermission::from_string("tool:control_device").unwrap();
        assert_eq!(perm, parsed);
    }
    
    #[test]
    fn test_permission_rule_creation() {
        let rule = PermissionRule::allow(
            McpPermission::tool("test_tool"),
            vec![Role::Admin, Role::Operator],
        );
        
        assert!(rule.applies_to_role(&Role::Admin));
        assert!(rule.applies_to_role(&Role::Operator));
        assert!(!rule.applies_to_role(&Role::Monitor));
        assert_eq!(rule.action, PermissionAction::Allow);
    }
    
    #[test]
    fn test_tool_category_extraction() {
        let checker = McpPermissionChecker::new(PermissionConfig::default());
        
        assert_eq!(checker.extract_tool_category("control_lights"), Some("control".to_string()));
        assert_eq!(checker.extract_tool_category("get_status"), Some("read".to_string()));
        assert_eq!(checker.extract_tool_category("set_temperature"), Some("write".to_string()));
        assert_eq!(checker.extract_tool_category("lighting_control"), Some("lighting".to_string()));
    }
    
    #[test]
    fn test_resource_category_extraction() {
        let checker = McpPermissionChecker::new(PermissionConfig::default());
        
        assert_eq!(
            checker.extract_resource_category("loxone://devices/all"),
            Some("devices".to_string())
        );
        assert_eq!(
            checker.extract_resource_category("system://status"),
            Some("status".to_string())
        );
    }
    
    #[test]
    fn test_resource_pattern_matching() {
        let checker = McpPermissionChecker::new(PermissionConfig::default());
        
        assert!(checker.matches_resource_pattern("loxone://admin/*", "loxone://admin/keys"));
        assert!(checker.matches_resource_pattern("system://status", "system://status"));
        assert!(!checker.matches_resource_pattern("loxone://admin/*", "loxone://devices/all"));
    }
    
    #[test]
    fn test_permission_config_builder() {
        let config = PermissionConfig::production()
            .allow_role_tool(Role::Operator, "control_device")
            .allow_role_resource(Role::Monitor, "system://status")
            .deny_role_resource(Role::Monitor, "loxone://admin/*");
        
        assert!(config.tools.tool_permissions.get("control_device").unwrap().contains(&Role::Operator));
        assert!(config.resources.resource_permissions.get("system://status").unwrap().contains(&Role::Monitor));
        assert_eq!(config.custom_rules.len(), 1);
    }
}