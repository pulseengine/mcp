//! Metrics collector implementation

use crate::{
    config::MonitoringConfig,
    metrics::{ServerMetrics, SystemMetrics},
};
use pulseengine_mcp_protocol::{Error, Request, Response};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use sysinfo::System;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

/// Simple request context for monitoring
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: uuid::Uuid,
}

/// Response time histogram for percentile calculations
#[derive(Clone)]
struct ResponseTimeHistogram {
    values: Arc<Mutex<VecDeque<f64>>>,
    max_size: usize,
}

impl ResponseTimeHistogram {
    fn new(max_size: usize) -> Self {
        Self {
            values: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    fn record(&self, value: f64) {
        let mut values = self.values.lock().unwrap();
        values.push_back(value);
        if values.len() > self.max_size {
            values.pop_front();
        }
    }

    fn get_average(&self) -> f64 {
        let values = self.values.lock().unwrap();
        if values.is_empty() {
            0.0
        } else {
            let sum: f64 = values.iter().sum();
            sum / values.len() as f64
        }
    }
}

/// Metrics collector for MCP server
pub struct MetricsCollector {
    config: MonitoringConfig,
    start_time: Instant,
    request_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    active_connections: Arc<AtomicU64>,
    response_times: ResponseTimeHistogram,
    system: Arc<RwLock<System>>,
    collection_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl MetricsCollector {
    pub fn new(config: MonitoringConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            config,
            start_time: Instant::now(),
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU64::new(0)),
            response_times: ResponseTimeHistogram::new(1000), // Keep last 1000 response times
            system: Arc::new(RwLock::new(system)),
            collection_handle: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start_collection(&self) {
        if self.config.enabled {
            let system = self.system.clone();
            let interval_secs = self.config.collection_interval_secs;

            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

                loop {
                    interval.tick().await;

                    // Refresh system information
                    let mut sys = system.write().await;
                    sys.refresh_all();
                }
            });

            // Store the handle
            let mut handle_guard = self.collection_handle.blocking_write();
            *handle_guard = Some(handle);

            tracing::info!(
                "Started metrics collection with {}s interval",
                interval_secs
            );
        } else {
            tracing::info!("Metrics collection is disabled");
        }
    }

    pub fn stop_collection(&self) {
        let mut handle_guard = self.collection_handle.blocking_write();
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            tracing::info!("Stopped metrics collection");
        }
    }

    /// Increment active connections
    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Process a request and update metrics
    ///
    /// # Errors
    ///
    /// This function currently never returns an error, but the signature allows for
    /// future error handling in metrics processing
    pub fn process_request(
        &self,
        request: Request,
        _context: &RequestContext,
    ) -> Result<Request, Error> {
        if self.config.enabled {
            self.request_count.fetch_add(1, Ordering::Relaxed);
        }
        Ok(request)
    }

    /// Process a response and update error metrics
    ///
    /// # Errors
    ///
    /// This function currently never returns an error, but the signature allows for
    /// future error handling in metrics processing
    pub fn process_response(
        &self,
        response: Response,
        context: &RequestContext,
    ) -> Result<Response, Error> {
        if self.config.enabled {
            if response.error.is_some() {
                self.error_count.fetch_add(1, Ordering::Relaxed);
            }

            // In a real implementation, we'd track request start time in context
            // For now, record a simulated response time
            let simulated_response_time = 10.0 + (context.request_id.as_u128() % 50) as f64;
            self.response_times.record(simulated_response_time);
        }
        Ok(response)
    }

    pub fn get_current_metrics(&self) -> ServerMetrics {
        let uptime_seconds = self.start_time.elapsed().as_secs();
        let requests_total = self.request_count.load(Ordering::Relaxed);
        let errors_total = self.error_count.load(Ordering::Relaxed);
        let active_connections = self.active_connections.load(Ordering::Relaxed);

        // Get system metrics
        let memory_usage_bytes = if self.config.enabled {
            let sys = self.system.blocking_read();
            // Get total system used memory
            sys.used_memory()
        } else {
            0
        };

        ServerMetrics {
            requests_total,
            requests_per_second: if uptime_seconds > 0 {
                #[allow(clippy::cast_precision_loss)]
                {
                    requests_total as f64 / uptime_seconds as f64
                }
            } else {
                0.0
            },
            average_response_time_ms: self.response_times.get_average(),
            error_rate: if requests_total > 0 {
                #[allow(clippy::cast_precision_loss)]
                {
                    errors_total as f64 / requests_total as f64
                }
            } else {
                0.0
            },
            active_connections,
            memory_usage_bytes,
            uptime_seconds,
        }
    }

    /// Get detailed system metrics
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        let sys = self.system.read().await;
        let load_avg = System::load_average();

        SystemMetrics {
            cpu_usage_percent: sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>()
                / sys.cpus().len() as f32,
            memory_total_bytes: sys.total_memory(),
            memory_used_bytes: sys.used_memory(),
            memory_available_bytes: sys.available_memory(),
            swap_total_bytes: sys.total_swap(),
            swap_used_bytes: sys.used_swap(),
            load_average: crate::metrics::LoadAverage {
                one: load_avg.one,
                five: load_avg.five,
                fifteen: load_avg.fifteen,
            },
            process_count: sys.processes().len() as u64,
        }
    }

    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

#[cfg(test)]
#[path = "collector_tests.rs"]
mod collector_tests;
