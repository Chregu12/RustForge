# RustForge Deployment Guide

Complete guide for deploying RustForge applications to production.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Environment Configuration](#environment-configuration)
- [Redis Setup](#redis-setup)
- [Database Setup](#database-setup)
- [Health Checks](#health-checks)
- [CORS Configuration](#cors-configuration)
- [Compression](#compression)
- [Rate Limiting](#rate-limiting)
- [Broadcasting](#broadcasting)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Monitoring](#monitoring)
- [Performance Tuning](#performance-tuning)

## Prerequisites

- Rust 1.75+
- PostgreSQL 14+ (or SQLite for development)
- Redis 7+ (for distributed deployments)
- Docker (optional, for containerized deployment)
- Kubernetes (optional, for orchestrated deployment)

## Environment Configuration

Create a `.env` file in your project root:

```env
# Application
APP_NAME=my-app
APP_ENV=production
APP_PORT=8080
APP_HOST=0.0.0.0

# Database
DATABASE_URL=postgres://user:password@localhost/mydb
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5

# Redis (for distributed deployments)
REDIS_URL=redis://localhost:6379

# Rate Limiting
RATE_LIMIT_ENABLED=true
RATE_LIMIT_PER_MINUTE=60
RATE_LIMIT_BACKEND=redis  # or "memory" for single-server

# Broadcasting
BROADCAST_ENABLED=true
BROADCAST_BACKEND=redis  # or "memory" for single-server

# CORS
CORS_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com
CORS_ALLOWED_METHODS=GET,POST,PUT,DELETE,PATCH,OPTIONS
CORS_ALLOWED_HEADERS=content-type,authorization,x-trace-id
CORS_MAX_AGE=3600

# Compression
COMPRESSION_ENABLED=true

# Timeouts
REQUEST_TIMEOUT_SECONDS=30

# Logging
RUST_LOG=info,my_app=debug
RUST_BACKTRACE=1
```

## Redis Setup

### Using Docker

```bash
docker run -d \
  --name redis \
  -p 6379:6379 \
  -v redis-data:/data \
  redis:7-alpine redis-server --appendonly yes
```

### Using Docker Compose

```yaml
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3

volumes:
  redis-data:
```

### Production Redis Configuration

For production, use Redis Cluster or Redis Sentinel for high availability:

```yaml
# Redis Sentinel (3 nodes minimum)
services:
  redis-master:
    image: redis:7-alpine
    command: redis-server --appendonly yes

  redis-replica-1:
    image: redis:7-alpine
    command: redis-server --slaveof redis-master 6379

  redis-replica-2:
    image: redis:7-alpine
    command: redis-server --slaveof redis-master 6379

  sentinel-1:
    image: redis:7-alpine
    command: redis-sentinel /etc/redis/sentinel.conf

  sentinel-2:
    image: redis:7-alpine
    command: redis-sentinel /etc/redis/sentinel.conf

  sentinel-3:
    image: redis:7-alpine
    command: redis-sentinel /etc/redis/sentinel.conf
```

## Database Setup

### PostgreSQL Configuration

```toml
# Cargo.toml
[dependencies]
rf-orm = "0.1"
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls"] }
```

```rust
use rf_orm::{DatabaseConfig, DatabasePool};

async fn setup_database() -> Result<DatabasePool, Box<dyn std::error::Error>> {
    let config = DatabaseConfig {
        url: std::env::var("DATABASE_URL")?,
        max_connections: 20,
        min_connections: 5,
        connect_timeout: std::time::Duration::from_secs(30),
        idle_timeout: Some(std::time::Duration::from_secs(600)),
    };

    let pool = DatabasePool::connect(config).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
```

## Health Checks

### Basic Setup

```toml
# Cargo.toml
[dependencies]
rf-health = "0.1"
```

```rust
use rf_health::{health_router, HealthChecker};
use rf_health::checks::{MemoryCheck, DiskCheck};
use axum::Router;

fn create_health_checker() -> HealthChecker {
    HealthChecker::new()
        .add_check(MemoryCheck::new(0.8, 0.95))  // 80% warning, 95% critical
        .add_check(DiskCheck::new("/", 0.8, 0.95))
}

async fn main() {
    let checker = create_health_checker();

    let app = Router::new()
        .merge(health_router(checker));

    // Health endpoints:
    // GET /health - All checks
    // GET /health/live - Liveness probe
    // GET /health/ready - Readiness probe
}
```

### With Database and Redis Checks

```rust
use rf_health::checks::{DatabaseCheck, RedisCheck};

async fn create_production_health_checker(
    db_pool: sqlx::PgPool,
    redis_url: &str,
) -> Result<HealthChecker, Box<dyn std::error::Error>> {
    let redis_check = RedisCheck::from_url(redis_url).await?;

    let checker = HealthChecker::new()
        .add_check(MemoryCheck::default())
        .add_check(DiskCheck::default())
        .add_check(DatabaseCheck::new(db_pool))
        .add_check(redis_check);

    Ok(checker)
}
```

## CORS Configuration

### Development Setup

```rust
use rf_web::{cors_layer, CorsConfig};

let cors_config = CorsConfig::default(); // Allows all origins
let app = Router::new()
    .layer(cors_layer(cors_config));
```

### Production Setup

```rust
use rf_web::{cors_layer, CorsConfig};
use axum::http::Method;
use std::time::Duration;

let cors_config = CorsConfig {
    allowed_origins: vec![
        "https://app.example.com".to_string(),
        "https://admin.example.com".to_string(),
    ],
    allowed_methods: vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
    ],
    allowed_headers: vec![
        "content-type".to_string(),
        "authorization".to_string(),
        "x-trace-id".to_string(),
    ],
    max_age: Some(Duration::from_secs(3600)),
};

let app = Router::new()
    .layer(cors_layer(cors_config));
```

### Environment-Based Configuration

```rust
use std::env;

fn load_cors_config() -> CorsConfig {
    let origins = env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "*".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    CorsConfig {
        allowed_origins: origins,
        ..Default::default()
    }
}
```

## Compression

### Enable Compression

```rust
use rf_web::compression_layer;

let app = Router::new()
    .layer(compression_layer()); // Enables gzip, brotli, deflate
```

Compression automatically applies to responses larger than 1KB.

## Rate Limiting

### Memory Backend (Single Server)

```rust
use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig};
use std::sync::Arc;

let config = RateLimitConfig::per_minute(60);
let limiter = Arc::new(MemoryRateLimiter::new(config));

// Use in middleware
async fn rate_limit_middleware(
    limiter: Arc<MemoryRateLimiter>,
    user_id: String,
    next: Next,
) -> Result<Response, StatusCode> {
    match limiter.check(&user_id).await {
        Ok(result) if result.allowed => Ok(next.run(request).await),
        Ok(result) => {
            // Rate limit exceeded
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

### Redis Backend (Distributed)

```toml
# Cargo.toml
[dependencies]
rf-ratelimit = { version = "0.1", features = ["redis-backend"] }
```

```rust
use rf_ratelimit::{RedisRateLimiter, RateLimitConfig};
use std::sync::Arc;

let config = RateLimitConfig::per_minute(60);
let limiter = Arc::new(
    RedisRateLimiter::new("redis://localhost", config).await?
);

// Use same middleware as above
```

## Broadcasting

### Memory Backend (Single Server)

```rust
use rf_broadcast::{MemoryBroadcaster, Channel, SimpleEvent};
use serde_json::json;
use std::sync::Arc;

let broadcaster = Arc::new(MemoryBroadcaster::new());

// Subscribe
broadcaster.subscribe(
    &Channel::public("users"),
    "conn-123".to_string(),
    None,
).await?;

// Broadcast
let event = SimpleEvent::new(
    "user.created",
    json!({"id": 123, "name": "John"}),
    vec![Channel::public("users")],
);

broadcaster.broadcast(&Channel::public("users"), &event).await?;
```

### Redis Backend (Distributed)

```toml
# Cargo.toml
[dependencies]
rf-broadcast = { version = "0.1", features = ["redis-backend"] }
```

```rust
use rf_broadcast::{RedisBroadcaster, Channel, SimpleEvent};
use serde_json::json;
use std::sync::Arc;

let broadcaster = Arc::new(
    RedisBroadcaster::new("redis://localhost").await?
);

// Use same API as memory backend
```

## Docker Deployment

### Dockerfile

```dockerfile
# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build for release
RUN cargo build --release -p my-app

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/my-app /app/my-app

# Copy migrations (if using sqlx)
COPY migrations ./migrations

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["./my-app"]
```

### Docker Compose (Full Stack)

```yaml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://user:password@postgres:5432/mydb
      - REDIS_URL=redis://redis:6379
      - APP_ENV=production
      - RUST_LOG=info
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 3s
      retries: 3

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: password
      POSTGRES_DB: mydb
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U user"]
      interval: 10s
      timeout: 3s
      retries: 3

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3

volumes:
  postgres-data:
  redis-data:
```

## Kubernetes Deployment

### Deployment Manifest

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
  labels:
    app: my-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
      - name: my-app
        image: my-app:latest
        ports:
        - containerPort: 8080
          name: http
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: app-secrets
              key: database-url
        - name: REDIS_URL
          value: "redis://redis-service:6379"
        - name: APP_ENV
          value: "production"
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
          timeoutSeconds: 3
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3
---
apiVersion: v1
kind: Service
metadata:
  name: my-app-service
spec:
  selector:
    app: my-app
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

### ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
data:
  RUST_LOG: "info,my_app=debug"
  APP_ENV: "production"
  CORS_ALLOWED_ORIGINS: "https://app.example.com,https://admin.example.com"
```

### Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: app-secrets
type: Opaque
stringData:
  database-url: "postgres://user:password@postgres:5432/mydb"
```

### Redis Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: redis
spec:
  replicas: 1
  selector:
    matchLabels:
      app: redis
  template:
    metadata:
      labels:
        app: redis
    spec:
      containers:
      - name: redis
        image: redis:7-alpine
        ports:
        - containerPort: 6379
        command: ["redis-server", "--appendonly", "yes"]
        volumeMounts:
        - name: redis-storage
          mountPath: /data
      volumes:
      - name: redis-storage
        persistentVolumeClaim:
          claimName: redis-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: redis-service
spec:
  selector:
    app: redis
  ports:
  - port: 6379
    targetPort: 6379
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: redis-pvc
spec:
  accessModes:
  - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
```

## Monitoring

### Prometheus Metrics (Future)

```rust
// Will be added in future phase
use prometheus::{Counter, Histogram, Registry};

let registry = Registry::new();
let request_counter = Counter::new("http_requests_total", "Total HTTP requests")?;
let request_duration = Histogram::new("http_request_duration_seconds", "HTTP request duration")?;

registry.register(Box::new(request_counter.clone()))?;
registry.register(Box::new(request_duration.clone()))?;
```

### Structured Logging

```rust
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// In handlers
info!(user_id = %user_id, "User logged in");
error!(error = %err, "Database query failed");
warn!(count = requests, "Rate limit approaching");
```

## Performance Tuning

### Database Connection Pool

```rust
let config = DatabaseConfig {
    max_connections: num_cpus::get() as u32 * 2,
    min_connections: 5,
    connect_timeout: Duration::from_secs(30),
    idle_timeout: Some(Duration::from_secs(600)),
};
```

### Redis Connection Pool

```rust
use deadpool_redis::{Config, Runtime};

let config = Config::from_url("redis://localhost");
config.pool_size = 20;
let pool = config.create_pool(Some(Runtime::Tokio1))?;
```

### Tokio Runtime Configuration

```rust
#[tokio::main(worker_threads = 8)]
async fn main() {
    // Application code
}
```

### Cargo.toml Optimizations

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

## Security Checklist

- [ ] Enable HTTPS (use reverse proxy like nginx/Caddy)
- [ ] Configure CORS with specific origins
- [ ] Enable rate limiting
- [ ] Use environment variables for secrets
- [ ] Enable request timeout
- [ ] Set up proper logging (no sensitive data)
- [ ] Use database connection pooling
- [ ] Enable compression for bandwidth savings
- [ ] Set up health checks for monitoring
- [ ] Use Redis Sentinel/Cluster for high availability
- [ ] Configure proper resource limits in Kubernetes
- [ ] Enable database query logging in non-production
- [ ] Set up automated backups

## Troubleshooting

### High Memory Usage

Check memory health endpoint:
```bash
curl http://localhost:8080/health
```

Adjust connection pool sizes if necessary.

### Rate Limiter Not Working

Verify Redis connection:
```bash
redis-cli ping
```

Check rate limiter configuration in logs.

### CORS Errors

Verify allowed origins in configuration:
```bash
echo $CORS_ALLOWED_ORIGINS
```

Ensure preflight requests are handled correctly.

### Database Connection Issues

Test database connectivity:
```bash
psql $DATABASE_URL -c "SELECT 1"
```

Check connection pool exhaustion in logs.

## Next Steps

- Set up CI/CD pipeline
- Configure monitoring and alerting
- Set up log aggregation (ELK, Loki)
- Configure automated backups
- Set up disaster recovery plan
- Load testing and performance benchmarking
- Security audit and penetration testing
