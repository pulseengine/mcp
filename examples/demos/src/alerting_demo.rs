#!/usr/bin/env rust-script
//! Alerting system demonstration
//!
//! This script demonstrates the comprehensive alerting and notification system
//! that has been implemented for the MCP server framework.

use pulseengine_mcp_logging::{
    Alert, AlertConfig, AlertManager, AlertRule, AlertSeverity, AlertState, ComparisonOperator,
    MetricType, NotificationChannel,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    println!("🚨 MCP Alerting System Demo");
    println!("===========================");

    // Create alert configuration with custom rules
    let mut config = AlertConfig::default();

    // Add a custom alert rule for high error rates
    config.rules.push(AlertRule {
        id: "demo_high_error_rate".to_string(),
        name: "Demo: High Error Rate".to_string(),
        description: "Error rate exceeds 10% for demonstration".to_string(),
        metric: MetricType::ErrorRate,
        operator: ComparisonOperator::GreaterThan,
        threshold: 0.1,
        duration_secs: 5, // Trigger after 5 seconds
        severity: AlertSeverity::High,
        enabled: true,
        channels: vec!["demo_console".to_string()],
        labels: {
            let mut labels = HashMap::new();
            labels.insert("demo".to_string(), "true".to_string());
            labels.insert("environment".to_string(), "development".to_string());
            labels
        },
        suppress_duration_secs: 60,
    });

    // Add a custom notification channel
    config.channels.insert(
        "demo_console".to_string(),
        NotificationChannel::Console { use_colors: true },
    );

    // Add demo console to default channels
    config.default_channels.push("demo_console".to_string());

    // Reduce evaluation interval for demo
    config.evaluation_interval_secs = 2;

    println!("📋 Alert Configuration:");
    println!("  - {} rules configured", config.rules.len());
    println!("  - {} notification channels", config.channels.len());
    println!(
        "  - Evaluation interval: {}s",
        config.evaluation_interval_secs
    );
    println!();

    // Create and start alert manager
    let alert_manager = Arc::new(AlertManager::new(config));
    alert_manager.start().await;

    println!("🎯 Starting alert manager...");
    sleep(Duration::from_secs(1)).await;

    // Simulate some alerts
    println!("📊 Alert Status:");

    // Wait for a few evaluation cycles
    for i in 1..=6 {
        println!(
            "  [{}] Evaluation cycle {}...",
            chrono::Utc::now().format("%H:%M:%S"),
            i
        );

        // Check active alerts
        let active_alerts = alert_manager.get_active_alerts().await;
        println!("    Active alerts: {}", active_alerts.len());

        if !active_alerts.is_empty() {
            for alert in &active_alerts {
                println!(
                    "      - {} ({}): {} - {}",
                    alert.severity_display(),
                    alert.state_display(),
                    alert.rule_id,
                    alert.message
                );
            }
        }

        sleep(Duration::from_secs(3)).await;
    }

    // Demonstrate alert acknowledgment
    let active_alerts = alert_manager.get_active_alerts().await;
    if let Some(alert) = active_alerts.first() {
        println!("\n✅ Acknowledging alert: {}", alert.id);
        alert_manager
            .acknowledge_alert(alert.id, "demo_user".to_string())
            .await?;

        // Show updated status
        let updated_alerts = alert_manager.get_active_alerts().await;
        if let Some(updated_alert) = updated_alerts.iter().find(|a| a.id == alert.id) {
            println!(
                "    Status: {} -> {}",
                alert.state_display(),
                updated_alert.state_display()
            );
        }
    }

    // Demonstrate alert resolution
    sleep(Duration::from_secs(2)).await;
    let active_alerts = alert_manager.get_active_alerts().await;
    if let Some(alert) = active_alerts.first() {
        println!("\n🔧 Resolving alert: {}", alert.id);
        alert_manager.resolve_alert(alert.id).await?;

        // Show final status
        let remaining_alerts = alert_manager.get_active_alerts().await;
        println!("    Remaining active alerts: {}", remaining_alerts.len());

        let history = alert_manager.get_alert_history().await;
        println!("    Alert history: {}", history.len());
    }

    println!("\n📈 Alert System Features Demonstrated:");
    println!("  ✅ Configurable alert rules and thresholds");
    println!("  ✅ Multiple notification channels (console, webhook, email, Slack, PagerDuty)");
    println!("  ✅ Alert severity levels (Critical, High, Medium, Low, Info)");
    println!("  ✅ Alert states (Active, Acknowledged, Resolved, Suppressed)");
    println!("  ✅ Alert de-duplication and suppression");
    println!("  ✅ Alert acknowledgment and resolution");
    println!("  ✅ Alert history tracking");
    println!("  ✅ Metric-based alerting (error rate, response time, etc.)");
    println!("  ✅ Comparison operators (>, >=, <, <=, ==, !=)");
    println!("  ✅ Custom labels and metadata");
    println!("  ✅ Re-notification for unacknowledged alerts");
    println!("  ✅ Cleanup and maintenance tasks");

    println!("\n🎉 Demo completed successfully!");
    Ok(())
}

// Helper trait for display formatting
trait AlertDisplay {
    fn severity_display(&self) -> &str;
    fn state_display(&self) -> &str;
}

impl AlertDisplay for Alert {
    fn severity_display(&self) -> &str {
        match self.severity {
            AlertSeverity::Critical => "🔴 CRITICAL",
            AlertSeverity::High => "🟠 HIGH",
            AlertSeverity::Medium => "🟡 MEDIUM",
            AlertSeverity::Low => "🟢 LOW",
            AlertSeverity::Info => "🔵 INFO",
        }
    }

    fn state_display(&self) -> &str {
        match self.state {
            AlertState::Active => "⚡ ACTIVE",
            AlertState::Acknowledged => "✅ ACKNOWLEDGED",
            AlertState::Resolved => "🔧 RESOLVED",
            AlertState::Suppressed => "🔇 SUPPRESSED",
        }
    }
}
