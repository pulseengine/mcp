//! Enhanced Hello World MCP Server with Comprehensive Features

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug)]
struct GreetingRecord {
    id: u64,
    name: String,
    greeting: String,
    language: String,
    timestamp: String,
}

/// Enhanced greeting server demonstrating comprehensive macro capabilities
///
/// This server showcases:
/// - #[mcp_server] for automatic server setup with application-specific configuration
/// - #[mcp_tools] for bulk tool registration from impl blocks
/// - Advanced greeting functionality with templates and history
/// - Multi-language support with cultural customization
/// - Comprehensive statistics and search capabilities
#[mcp_server(
    name = "Enhanced Hello World Server",
    app_name = "hello-world-enhanced",
    version = "2.0.0",
    description = "Comprehensive demo of MCP macro capabilities with tools, history, and customization"
)]
#[derive(Clone)]
pub struct EnhancedHelloWorldServer {
    greeting_count: Arc<AtomicU64>,
    greeting_history: Arc<RwLock<Vec<GreetingRecord>>>,
    templates: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for EnhancedHelloWorldServer {
    fn default() -> Self {
        let mut templates = HashMap::new();
        templates.insert(
            "formal".to_string(),
            "Good day, {name}. I hope this message finds you well.".to_string(),
        );
        templates.insert(
            "casual".to_string(),
            "Hey {name}! What's up? 😊".to_string(),
        );
        templates.insert(
            "enthusiastic".to_string(),
            "WOW! Hi there {name}! So excited to meet you! 🎉".to_string(),
        );
        templates.insert(
            "professional".to_string(),
            "Dear {name}, thank you for connecting with our service.".to_string(),
        );
        templates.insert(
            "friendly".to_string(),
            "Hi {name}! Nice to meet you! 🤝".to_string(),
        );

        Self {
            greeting_count: Arc::new(AtomicU64::new(0)),
            greeting_history: Arc::new(RwLock::new(Vec::new())),
            templates: Arc::new(RwLock::new(templates)),
        }
    }
}

/// All tools are automatically registered via the #[mcp_tools] macro
/// This demonstrates the complete tool functionality with comprehensive features
#[mcp_tools]
impl EnhancedHelloWorldServer {
    /// Generate a personalized greeting with extensive customization options
    ///
    /// This tool supports multiple greeting types, languages, and styling options.
    /// It maintains a complete history of all greetings for analytics and personalization.
    ///
    /// # Parameters
    /// - name: The name of the person to greet (required)
    /// - greeting_type: Style of greeting (casual, formal, enthusiastic, professional, friendly)
    /// - language: Language code (en, es, fr, de, ja) - defaults to English
    /// - include_emoji: Whether to include emoji decorations (default: true)
    ///
    /// # Returns
    /// A personalized greeting string with unique numbering
    pub async fn say_hello(
        &self,
        name: String,
        greeting_type: Option<String>,
        language: Option<String>,
        include_emoji: Option<bool>,
    ) -> String {
        let greeting_type = greeting_type.unwrap_or_else(|| "casual".to_string());
        let language = language.unwrap_or_else(|| "en".to_string());
        let include_emoji = include_emoji.unwrap_or(true);

        let count = self.greeting_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Get greeting template
        let templates = self.templates.read().unwrap();
        let template = templates
            .get(&greeting_type)
            .unwrap_or(&"Hello {name}!".to_string())
            .clone();
        drop(templates);

        // Generate greeting based on template
        let mut greeting = template.replace("{name}", &name);

        // Apply language-specific customizations
        match language.as_str() {
            "es" => greeting = format!("¡{}!", greeting.trim_end_matches('!')),
            "fr" => greeting = format!("{}!", greeting.trim_end_matches('!')),
            "de" => greeting = greeting.replace("Hello", "Hallo").replace("Hi", "Hallo"),
            "ja" => greeting = format!("{name}さん、こんにちは！"),
            _ => {} // English default
        }

        // Add emoji decoration if requested
        if include_emoji {
            let emoji = match greeting_type.as_str() {
                "formal" => "🤝",
                "casual" => "👋",
                "enthusiastic" => "🎉",
                "professional" => "💼",
                "friendly" => "😊",
                _ => "👋",
            };
            greeting = format!("{greeting} {emoji}");
        }

        // Record the greeting for history and analytics
        let record = GreetingRecord {
            id: count,
            name: name.clone(),
            greeting: greeting.clone(),
            language,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let mut history = self.greeting_history.write().unwrap();
        history.push(record);

        tracing::info!(
            tool = "say_hello",
            name = %name,
            greeting_type = %greeting_type,
            count = count,
            "Generated personalized greeting"
        );

        format!("{greeting} (Greeting #{count})")
    }

    /// Get comprehensive greeting statistics and analytics
    ///
    /// Returns detailed statistics about greeting usage including:
    /// - Total number of greetings generated
    /// - Language distribution breakdown
    /// - Recent greeting history (last 5)
    /// - Available template options
    pub fn get_greeting_stats(&self) -> serde_json::Value {
        let count = self.greeting_count.load(Ordering::Relaxed);
        let history = self.greeting_history.read().unwrap();

        let mut language_counts = HashMap::new();
        let mut greeting_type_counts = HashMap::new();
        let mut recent_greetings = Vec::new();

        // Analyze recent greetings for patterns
        for record in history.iter().rev().take(5) {
            *language_counts.entry(record.language.clone()).or_insert(0) += 1;
            recent_greetings.push(json!({
                "id": record.id,
                "name": record.name,
                "greeting": record.greeting,
                "language": record.language,
                "timestamp": record.timestamp
            }));
        }

        // Count greeting types based on emoji patterns (simple heuristic)
        for record in history.iter() {
            let greeting_type = if record.greeting.contains("🤝") {
                "formal"
            } else if record.greeting.contains("🎉") {
                "enthusiastic"
            } else if record.greeting.contains("💼") {
                "professional"
            } else if record.greeting.contains("😊") {
                "friendly"
            } else {
                "casual"
            };
            *greeting_type_counts
                .entry(greeting_type.to_string())
                .or_insert(0) += 1;
        }

        tracing::info!(
            tool = "get_greeting_stats",
            total_count = count,
            unique_languages = language_counts.len(),
            "Retrieved comprehensive greeting statistics"
        );

        json!({
            "total_greetings": count,
            "language_distribution": language_counts,
            "greeting_type_distribution": greeting_type_counts,
            "recent_greetings": recent_greetings,
            "available_templates": self.templates.read().unwrap().keys().collect::<Vec<_>>(),
            "statistics_generated_at": chrono::Utc::now().to_rfc3339()
        })
    }

    /// Add a custom greeting template with validation
    ///
    /// Allows users to create personalized greeting templates that can be used
    /// with the say_hello tool. Templates must contain the {name} placeholder.
    ///
    /// # Parameters
    /// - template_name: Unique name for the template
    /// - template_text: Template text with {name} placeholder
    ///
    /// # Returns
    /// Success confirmation message
    pub fn add_greeting_template(
        &self,
        template_name: String,
        template_text: String,
    ) -> Result<String, String> {
        if template_name.is_empty() || template_text.is_empty() {
            return Err("Template name and text cannot be empty".to_string());
        }

        if !template_text.contains("{name}") {
            return Err("Template must contain {name} placeholder".to_string());
        }

        let mut templates = self.templates.write().unwrap();
        let is_update = templates.contains_key(&template_name);
        templates.insert(template_name.clone(), template_text.clone());

        tracing::info!(
            tool = "add_greeting_template",
            template_name = %template_name,
            is_update = is_update,
            "Added/updated custom greeting template"
        );

        if is_update {
            Ok(format!("Successfully updated template: {template_name}"))
        } else {
            Ok(format!("Successfully added new template: {template_name}"))
        }
    }

    /// Search greeting history with advanced filtering
    ///
    /// Provides powerful search capabilities across the greeting history.
    /// Searches through names, greeting text, and languages.
    ///
    /// # Parameters
    /// - query: Search term to look for
    /// - limit: Maximum number of results to return (default: 10)
    ///
    /// # Returns
    /// Array of matching greeting records with full details
    pub fn search_greetings(&self, query: String, limit: Option<u32>) -> Vec<serde_json::Value> {
        let history = self.greeting_history.read().unwrap();
        let limit = limit.unwrap_or(10) as usize;
        let query_lower = query.to_lowercase();

        let results: Vec<serde_json::Value> = history
            .iter()
            .filter(|record| {
                record.name.to_lowercase().contains(&query_lower)
                    || record.greeting.to_lowercase().contains(&query_lower)
                    || record.language.to_lowercase().contains(&query_lower)
            })
            .rev() // Most recent first
            .take(limit)
            .map(|record| {
                let days_ago = {
                    let timestamp = chrono::DateTime::parse_from_rfc3339(&record.timestamp)
                        .unwrap_or_else(|_| chrono::Utc::now().into());
                    let now = chrono::Utc::now();
                    (now - timestamp.with_timezone(&chrono::Utc)).num_days()
                };
                json!({
                    "id": record.id,
                    "name": record.name,
                    "greeting": record.greeting,
                    "language": record.language,
                    "timestamp": record.timestamp,
                    "days_ago": days_ago
                })
            })
            .collect();

        tracing::info!(
            tool = "search_greetings",
            query = %query,
            results_count = results.len(),
            "Searched greeting history with advanced filtering"
        );

        results
    }

    /// Get current server status and performance metrics
    ///
    /// Returns comprehensive information about the server's current state,
    /// including uptime, performance metrics, and operational statistics.
    pub fn get_server_status(&self) -> serde_json::Value {
        let count = self.greeting_count.load(Ordering::Relaxed);
        let history = self.greeting_history.read().unwrap();
        let templates = self.templates.read().unwrap();

        // Calculate some basic metrics
        let avg_greetings_per_minute = if history.len() >= 2 {
            let first = history.first().unwrap();
            let last = history.last().unwrap();

            if let (Ok(first_time), Ok(last_time)) = (
                chrono::DateTime::parse_from_rfc3339(&first.timestamp),
                chrono::DateTime::parse_from_rfc3339(&last.timestamp),
            ) {
                let duration_mins = (last_time - first_time).num_minutes() as f64;
                if duration_mins > 0.0 {
                    history.len() as f64 / duration_mins
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        json!({
            "status": "running",
            "server_name": "Enhanced Hello World Server",
            "version": "2.0.0",
            "app_name": "hello-world-enhanced",
            "current_time": chrono::Utc::now().to_rfc3339(),
            "total_greetings": count,
            "total_history_records": history.len(),
            "available_templates": templates.len(),
            "template_names": templates.keys().collect::<Vec<_>>(),
            "performance_metrics": {
                "average_greetings_per_minute": avg_greetings_per_minute,
                "memory_efficiency": "optimized",
                "concurrent_safety": "thread_safe"
            },
            "features": [
                "multi_language_support",
                "custom_templates",
                "history_tracking",
                "advanced_search",
                "statistics_analytics",
                "emoji_decorations"
            ]
        })
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize comprehensive logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("🚀 Starting Enhanced Hello World MCP Server");
    tracing::info!("📦 App Name: hello-world-enhanced");
    tracing::info!("🔧 Features: Advanced tools with comprehensive functionality");
    tracing::info!("🔐 Authentication: Application-specific configuration");

    // Create and configure the server with application-specific settings
    let server = EnhancedHelloWorldServer::with_defaults()
        .serve_stdio()
        .await?;

    tracing::info!("✅ Enhanced Hello World MCP Server started successfully");
    tracing::info!("🛠️  Available Tools:");
    tracing::info!("   • say_hello - Personalized greetings with multi-language support");
    tracing::info!("   • get_greeting_stats - Comprehensive analytics and statistics");
    tracing::info!("   • add_greeting_template - Custom template management");
    tracing::info!("   • search_greetings - Advanced history search capabilities");
    tracing::info!("   • get_server_status - Server status and performance metrics");
    tracing::info!("🔗 Connect using any MCP client via stdio transport");
    tracing::info!(
        "📚 Documentation: This server demonstrates the full power of PulseEngine MCP macros"
    );

    // Run the server with automatic capability detection
    server
        .run()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    tracing::info!("👋 Enhanced Hello World MCP Server stopped gracefully");
    Ok(())
}
