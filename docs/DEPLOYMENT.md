# PulseEngine MCP Macros: Deployment Guide

This guide covers production deployment strategies for MCP servers built with PulseEngine macros.

## Deployment Architectures

### Standalone Deployment

Deploy as a single binary with embedded transport:

```rust
use pulseengine_mcp_macros::mcp_server;
use clap::{Arg, Command};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[mcp_server(
    name = "Production Server",
    app_name = "myapp",
    version = "1.0.0"
)]
#[derive(Clone)]
pub struct ProductionServer {
    config: Arc<ServerConfig>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let matches = Command::new("myapp-mcp-server")
        .version("1.0.0")
        .arg(Arg::new("transport")
            .long("transport")
            .value_name("TRANSPORT")
            .help("Transport type: stdio, http, websocket")
            .default_value("stdio"))
        .arg(Arg::new("port")
            .long("port")
            .value_name("PORT")
            .help("Port for HTTP/WebSocket transport")
            .default_value("8080"))
        .arg(Arg::new("config")
            .long("config")
            .value_name("FILE")
            .help("Configuration file path"))
        .get_matches();

    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "myapp=info,pulseengine_mcp=info".into())
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = if let Some(config_path) = matches.get_one::<String>("config") {
        ServerConfig::from_file(config_path).await?
    } else {
        ServerConfig::from_env()?
    };

    // Create server
    let server = ProductionServer::with_config(config);

    // Setup graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        tracing::info!("Shutdown signal received");
    };

    // Start server based on transport
    let transport = matches.get_one::<String>("transport").unwrap();
    let service = match transport.as_str() {
        "stdio" => {
            tracing::info!("Starting MCP server with STDIO transport");
            server.serve_stdio().await?
        }
        "http" => {
            let port: u16 = matches.get_one::<String>("port").unwrap().parse()?;
            tracing::info!("Starting MCP server with HTTP transport on port {}", port);
            server.serve_http(port).await?
        }
        "websocket" => {
            let port: u16 = matches.get_one::<String>("port").unwrap().parse()?;
            let addr = format!("0.0.0.0:{}", port);
            tracing::info!("Starting MCP server with WebSocket transport on {}", addr);
            server.serve_ws(&addr).await?
        }
        _ => return Err(format!("Unknown transport: {}", transport).into()),
    };

    // Run with graceful shutdown
    service.run_with_shutdown(shutdown_signal).await?;
    
    tracing::info!("Server shutdown complete");
    Ok(())
}
```

### Containerized Deployment

Deploy using Docker containers:

```dockerfile
# Dockerfile
FROM rust:1.88-slim as builder

WORKDIR /app
COPY . .

# Build dependencies first for better caching
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Copy binary
COPY --from=builder /app/target/release/myapp-mcp-server /usr/local/bin/

# Create directories for app-specific storage
RUN mkdir -p /app/data /app/config /app/logs && \
    chown -R appuser:appuser /app

USER appuser
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD /usr/local/bin/myapp-mcp-server --transport http --port 8080 || exit 1

EXPOSE 8080
CMD ["/usr/local/bin/myapp-mcp-server", "--transport", "http", "--port", "8080"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  mcp-server:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - DATABASE_URL=postgresql://postgres:password@db:5432/myapp
      - REDIS_URL=redis://redis:6379
    volumes:
      - ./config:/app/config:ro
      - ./data:/app/data
      - ./logs:/app/logs
    depends_on:
      - db
      - redis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  db:
    image: postgres:15-alpine
    environment:
      - POSTGRES_DB=myapp
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
```

### Kubernetes Deployment

Deploy on Kubernetes with high availability:

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: mcp-system
---
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mcp-server-config
  namespace: mcp-system
data:
  config.toml: |
    [server]
    name = "Production MCP Server"
    version = "1.0.0"
    max_connections = 1000
    
    [database]
    url = "postgresql://postgres:password@postgres:5432/myapp"
    max_connections = 20
    
    [redis]
    url = "redis://redis:6379"
    max_connections = 10
    
    [logging]
    level = "info"
    format = "json"
---
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mcp-server
  namespace: mcp-system
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mcp-server
  template:
    metadata:
      labels:
        app: mcp-server
    spec:
      containers:
      - name: mcp-server
        image: myapp/mcp-server:1.0.0
        ports:
        - containerPort: 8080
        env:
        - name: RUST_LOG
          value: "info"
        - name: CONFIG_PATH
          value: "/etc/config/config.toml"
        volumeMounts:
        - name: config
          mountPath: /etc/config
          readOnly: true
        - name: data
          mountPath: /app/data
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: mcp-server-config
      - name: data
        persistentVolumeClaim:
          claimName: mcp-server-data
---
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: mcp-server
  namespace: mcp-system
spec:
  selector:
    app: mcp-server
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: ClusterIP
---
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: mcp-server
  namespace: mcp-system
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - mcp.example.com
    secretName: mcp-server-tls
  rules:
  - host: mcp.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: mcp-server
            port:
              number: 80
```

## Configuration Management

### Configuration Structure

```toml
# config/production.toml
[server]
name = "Production MCP Server"
version = "1.0.0"
description = "Production deployment of MyApp MCP server"
app_name = "myapp"
bind_address = "0.0.0.0:8080"
max_connections = 1000
request_timeout = 30
shutdown_timeout = 10

[database]
url = "postgresql://user:pass@localhost:5432/myapp"
max_connections = 20
min_connections = 5
connection_timeout = 30
idle_timeout = 600

[redis]
url = "redis://localhost:6379"
max_connections = 10
connection_timeout = 5

[auth]
enabled = true
api_key_header = "X-API-Key"
jwt_secret = "${JWT_SECRET}"
token_expiry = 3600

[logging]
level = "info"
format = "json"
file_path = "/app/logs/server.log"
max_file_size = "100MB"
max_files = 10

[metrics]
enabled = true
prometheus_endpoint = "/metrics"
namespace = "myapp_mcp"

[security]
cors_enabled = true
cors_origins = ["https://app.example.com"]
rate_limit = 100
rate_limit_window = 60

[features]
cache_enabled = true
batch_processing = true
streaming = true
```

### Environment-Based Configuration

```rust
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub auth: AuthSettings,
    pub logging: LoggingSettings,
    pub metrics: MetricsSettings,
    pub security: SecuritySettings,
    pub features: FeatureFlags,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let env = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into());
        
        let config = Config::builder()
            // Default configuration
            .add_source(File::with_name("config/default"))
            // Environment-specific configuration
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            // Local overrides
            .add_source(File::with_name("config/local").required(false))
            // Environment variables
            .add_source(Environment::with_prefix("MYAPP").separator("_"))
            .build()?;
        
        config.try_deserialize()
    }

    pub async fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::from(path.as_ref()))
            .add_source(Environment::with_prefix("MYAPP").separator("_"))
            .build()?;
        
        config.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate database URL
        if self.database.url.is_empty() {
            return Err(ConfigError::Message("Database URL is required".into()));
        }

        // Validate connection limits
        if self.database.max_connections == 0 {
            return Err(ConfigError::Message("Database max_connections must be > 0".into()));
        }

        // Validate auth settings if enabled
        if self.auth.enabled && self.auth.jwt_secret.is_empty() {
            return Err(ConfigError::Message("JWT secret is required when auth is enabled".into()));
        }

        Ok(())
    }
}
```

## Monitoring and Observability

### Metrics Collection

```rust
use prometheus::{Counter, Histogram, IntGauge, Registry};
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    request_count: Counter,
    request_duration: Histogram,
    active_connections: IntGauge,
    tool_calls: Counter,
    resource_reads: Counter,
    prompt_generations: Counter,
    errors: Counter,
}

impl Metrics {
    pub fn new(namespace: &str) -> Result<Self, prometheus::Error> {
        let registry = Arc::new(Registry::new());
        
        let request_count = Counter::new(
            format!("{}_requests_total", namespace),
            "Total number of requests processed"
        )?;
        
        let request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                format!("{}_request_duration_seconds", namespace),
                "Request duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0])
        )?;
        
        let active_connections = IntGauge::new(
            format!("{}_active_connections", namespace),
            "Number of active connections"
        )?;
        
        let tool_calls = Counter::new(
            format!("{}_tool_calls_total", namespace),
            "Total number of tool calls"
        )?;
        
        let resource_reads = Counter::new(
            format!("{}_resource_reads_total", namespace),
            "Total number of resource reads"
        )?;
        
        let prompt_generations = Counter::new(
            format!("{}_prompt_generations_total", namespace),
            "Total number of prompt generations"
        )?;
        
        let errors = Counter::new(
            format!("{}_errors_total", namespace),
            "Total number of errors"
        )?;
        
        // Register metrics
        registry.register(Box::new(request_count.clone()))?;
        registry.register(Box::new(request_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(tool_calls.clone()))?;
        registry.register(Box::new(resource_reads.clone()))?;
        registry.register(Box::new(prompt_generations.clone()))?;
        registry.register(Box::new(errors.clone()))?;
        
        Ok(Self {
            registry,
            request_count,
            request_duration,
            active_connections,
            tool_calls,
            resource_reads,
            prompt_generations,
            errors,
        })
    }
    
    pub fn record_request(&self, duration: f64) {
        self.request_count.inc();
        self.request_duration.observe(duration);
    }
    
    pub fn increment_active_connections(&self) {
        self.active_connections.inc();
    }
    
    pub fn decrement_active_connections(&self) {
        self.active_connections.dec();
    }
    
    pub fn record_tool_call(&self) {
        self.tool_calls.inc();
    }
    
    pub fn record_resource_read(&self) {
        self.resource_reads.inc();
    }
    
    pub fn record_prompt_generation(&self) {
        self.prompt_generations.inc();
    }
    
    pub fn record_error(&self) {
        self.errors.inc();
    }
    
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
}

// Integrate metrics into server
#[mcp_server(name = "Monitored Server")]
#[derive(Clone)]
pub struct MonitoredServer {
    metrics: Metrics,
    inner: Arc<RwLock<ServerState>>,
}

impl MonitoredServer {
    pub async fn serve_with_metrics(&self, port: u16) -> Result<(), ServerError> {
        let metrics = self.metrics.clone();
        
        // Start metrics endpoint
        let metrics_handler = {
            let registry = metrics.registry();
            move || {
                let encoder = prometheus::TextEncoder::new();
                let metric_families = registry.gather();
                encoder.encode_to_string(&metric_families).unwrap_or_default()
            }
        };
        
        // Serve metrics on /metrics endpoint
        let metrics_route = warp::path("metrics")
            .and(warp::get())
            .map(metrics_handler);
        
        // Serve main MCP endpoints with metrics middleware
        let mcp_routes = self.create_mcp_routes()
            .with(warp::filters::trace::trace(|info| {
                let start = std::time::Instant::now();
                tracing::info_span!("request", method = %info.method(), path = %info.path())
            }))
            .with(warp::wrap_fn(move |req, next| {
                let metrics = metrics.clone();
                async move {
                    metrics.increment_active_connections();
                    let start = std::time::Instant::now();
                    
                    let result = next.run(req).await;
                    
                    let duration = start.elapsed().as_secs_f64();
                    metrics.record_request(duration);
                    metrics.decrement_active_connections();
                    
                    result
                }
            }));
        
        let routes = metrics_route.or(mcp_routes);
        
        warp::serve(routes)
            .run(([0, 0, 0, 0], port))
            .await;
            
        Ok(())
    }
}
```

### Distributed Tracing

```rust
use opentelemetry::{
    trace::{TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_jaeger::new_agent_pipeline;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Jaeger tracer
    let tracer = new_agent_pipeline()
        .with_service_name(service_name)
        .with_auto_split_batch(true)
        .install_batch(opentelemetry::runtime::Tokio)?;

    // Initialize tracing subscriber with OpenTelemetry layer
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .try_init()?;

    Ok(())
}

#[mcp_tool]
impl MonitoredServer {
    /// Tool with distributed tracing
    #[tracing::instrument(skip(self), fields(tool_name = "traced_operation"))]
    async fn traced_operation(&self, input: String) -> Result<String, OperationError> {
        let span = tracing::Span::current();
        span.record("input_length", input.len());
        
        // Child span for database operation
        let db_result = {
            let _db_span = tracing::info_span!("database_query").entered();
            self.query_database(&input).await?
        };
        
        // Child span for processing
        let processed = {
            let _process_span = tracing::info_span!("data_processing").entered();
            self.process_data(db_result).await?
        };
        
        span.record("output_length", processed.len());
        Ok(processed)
    }
}
```

## Security Hardening

### TLS Configuration

```rust
use rustls::{Certificate, PrivateKey, ServerConfig as TlsConfig};
use std::io::BufReader;

pub struct TlsManager {
    config: Arc<TlsConfig>,
}

impl TlsManager {
    pub fn new(cert_path: &str, key_path: &str) -> Result<Self, TlsError> {
        // Load certificates
        let cert_file = std::fs::File::open(cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs = rustls_pemfile::certs(&mut cert_reader)?
            .into_iter()
            .map(Certificate)
            .collect();

        // Load private key
        let key_file = std::fs::File::open(key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)?;
        
        if keys.is_empty() {
            return Err(TlsError::NoPrivateKey);
        }
        
        let key = PrivateKey(keys[0].clone());

        // Configure TLS
        let config = TlsConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(Self {
            config: Arc::new(config),
        })
    }
    
    pub fn config(&self) -> Arc<TlsConfig> {
        self.config.clone()
    }
}

// Use TLS in server
impl ProductionServer {
    pub async fn serve_https(&self, port: u16, tls_manager: TlsManager) -> Result<impl McpService, ServerError> {
        use warp::Filter;
        
        let routes = self.create_routes();
        
        warp::serve(routes)
            .tls()
            .cert_path("path/to/cert.pem")
            .key_path("path/to/key.pem")
            .run(([0, 0, 0, 0], port))
            .await;
            
        Ok(())
    }
}
```

### Rate Limiting and DDoS Protection

```rust
use governor::{Quota, RateLimiter};
use std::net::IpAddr;
use std::collections::HashMap;

#[derive(Clone)]
pub struct RateLimitManager {
    global_limiter: Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    per_ip_limiters: Arc<RwLock<HashMap<IpAddr, Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>>>>,
    quota: Quota,
}

impl RateLimitManager {
    pub fn new(requests_per_minute: u32, burst_size: u32) -> Self {
        let quota = Quota::per_minute(nonzero::NonZeroU32::new(requests_per_minute).unwrap())
            .allow_burst(nonzero::NonZeroU32::new(burst_size).unwrap());
        
        let global_limiter = Arc::new(RateLimiter::direct(quota));
        
        Self {
            global_limiter,
            per_ip_limiters: Arc::new(RwLock::new(HashMap::new())),
            quota,
        }
    }
    
    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        // Check global rate limit
        self.global_limiter.check().map_err(|_| RateLimitError::GlobalLimitExceeded)?;
        
        // Check per-IP rate limit
        let limiters = self.per_ip_limiters.read().await;
        let limiter = if let Some(limiter) = limiters.get(&ip) {
            limiter.clone()
        } else {
            drop(limiters);
            let mut limiters = self.per_ip_limiters.write().await;
            let limiter = Arc::new(RateLimiter::direct(self.quota));
            limiters.insert(ip, limiter.clone());
            limiter
        };
        
        limiter.check().map_err(|_| RateLimitError::IpLimitExceeded { ip })
    }
}

// Integrate rate limiting
impl ProductionServer {
    pub async fn serve_with_rate_limiting(&self, port: u16) -> Result<(), ServerError> {
        let rate_limiter = RateLimitManager::new(100, 10); // 100 requests per minute, burst of 10
        
        let routes = self.create_routes()
            .with(warp::wrap_fn(move |req, next| {
                let rate_limiter = rate_limiter.clone();
                async move {
                    let ip = req.remote_addr()
                        .map(|addr| addr.ip())
                        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
                    
                    if let Err(e) = rate_limiter.check_rate_limit(ip).await {
                        return Ok(warp::reply::with_status(
                            warp::reply::json(&serde_json::json!({
                                "error": "Rate limit exceeded",
                                "details": e.to_string()
                            })),
                            warp::http::StatusCode::TOO_MANY_REQUESTS
                        ).into_response());
                    }
                    
                    next.run(req).await
                }
            }));
        
        warp::serve(routes).run(([0, 0, 0, 0], port)).await;
        Ok(())
    }
}
```

## High Availability and Load Balancing

### Health Checks

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime: Duration,
    pub checks: HashMap<String, ComponentHealth>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: String,
    pub response_time_ms: u64,
    pub details: Option<String>,
}

impl ProductionServer {
    pub async fn health_check(&self) -> HealthStatus {
        let start_time = self.start_time;
        let uptime = Utc::now().signed_duration_since(start_time);
        
        let mut checks = HashMap::new();
        
        // Database health check
        let db_start = std::time::Instant::now();
        let db_health = match self.check_database_health().await {
            Ok(_) => ComponentHealth {
                status: "healthy".to_string(),
                response_time_ms: db_start.elapsed().as_millis() as u64,
                details: None,
            },
            Err(e) => ComponentHealth {
                status: "unhealthy".to_string(),
                response_time_ms: db_start.elapsed().as_millis() as u64,
                details: Some(e.to_string()),
            },
        };
        checks.insert("database".to_string(), db_health);
        
        // Redis health check
        let redis_start = std::time::Instant::now();
        let redis_health = match self.check_redis_health().await {
            Ok(_) => ComponentHealth {
                status: "healthy".to_string(),
                response_time_ms: redis_start.elapsed().as_millis() as u64,
                details: None,
            },
            Err(e) => ComponentHealth {
                status: "unhealthy".to_string(),
                response_time_ms: redis_start.elapsed().as_millis() as u64,
                details: Some(e.to_string()),
            },
        };
        checks.insert("redis".to_string(), redis_health);
        
        // Overall status
        let overall_status = if checks.values().all(|h| h.status == "healthy") {
            "healthy"
        } else {
            "unhealthy"
        };
        
        HealthStatus {
            status: overall_status.to_string(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: uptime.to_std().unwrap_or_default(),
            checks,
        }
    }
    
    async fn check_database_health(&self) -> Result<(), DatabaseError> {
        // Simple query to check database connectivity
        let _result = self.database_pool.get().await?
            .query_one("SELECT 1", &[]).await?;
        Ok(())
    }
    
    async fn check_redis_health(&self) -> Result<(), RedisError> {
        let mut conn = self.redis_pool.get().await?;
        let _result: String = redis::cmd("PING").query_async(&mut *conn).await?;
        Ok(())
    }
}
```

### Load Balancer Configuration

```nginx
# nginx.conf
upstream mcp_servers {
    least_conn;
    server mcp-server-1:8080 max_fails=3 fail_timeout=30s;
    server mcp-server-2:8080 max_fails=3 fail_timeout=30s;
    server mcp-server-3:8080 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    listen 443 ssl http2;
    server_name mcp.example.com;
    
    ssl_certificate /etc/ssl/certs/mcp.example.com.crt;
    ssl_certificate_key /etc/ssl/private/mcp.example.com.key;
    
    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    
    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    limit_req zone=api burst=20 nodelay;
    
    location /health {
        proxy_pass http://mcp_servers/health;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Health check specific settings
        proxy_connect_timeout 5s;
        proxy_send_timeout 5s;
        proxy_read_timeout 5s;
    }
    
    location / {
        proxy_pass http://mcp_servers;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        
        # Timeouts
        proxy_connect_timeout 10s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        
        # Buffer settings
        proxy_buffering on;
        proxy_buffer_size 4k;
        proxy_buffers 8 4k;
    }
}
```

This deployment guide provides comprehensive strategies for production deployment of MCP servers built with PulseEngine macros, covering containerization, orchestration, monitoring, security, and high availability patterns.