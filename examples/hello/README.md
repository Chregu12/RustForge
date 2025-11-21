# Hello World - RustForge Phase 2 Example

Minimal application demonstrating integration of Phase 2 framework crates:

- **rf-core**: Error handling with RFC 7807 and request context
- **rf-web**: Axum integration with middleware stack
- **rf-config**: Type-safe hierarchical configuration
- **rf-container**: Dependency injection container

## Quick Start

```bash
# Build and run
cargo run -p hello

# Server will start on http://127.0.0.1:3000
```

## Endpoints

### `GET /`
Hello world message with trace ID and config info

```bash
curl http://localhost:3000/
```

Response:
```json
{
  "message": "Hello from RustForge Phase 2!",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "version": "0.1.0",
  "config": {
    "server": {
      "host": "127.0.0.1",
      "port": 3000
    }
  }
}
```

### `GET /health`
Health check endpoint (Kubernetes liveness probe)

```bash
curl http://localhost:3000/health
```

Response:
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

### `GET /ready`
Readiness probe (Kubernetes readiness probe)

```bash
curl http://localhost:3000/ready
```

Response:
```json
{
  "ready": true,
  "checks": {
    "database": "ok",
    "cache": "ok"
  }
}
```

### `GET /metrics`
Metrics endpoint (Prometheus-compatible placeholder)

```bash
curl http://localhost:3000/metrics
```

Response:
```json
{
  "requests_total": 0,
  "uptime_seconds": 0
}
```

### `POST /echo`
Echo message (demonstrates validation and error handling)

```bash
curl -X POST http://localhost:3000/echo \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello!"}'
```

Response:
```json
{
  "message": "Echo: Hello!"
}
```

Error handling (empty message):
```bash
curl -X POST http://localhost:3000/echo \
  -H "Content-Type: application/json" \
  -d '{"message": ""}'
```

Response (400 Bad Request):
```json
{
  "type": "about:blank",
  "title": "Bad Request",
  "status": 400,
  "detail": "Message cannot be empty",
  "instance": "/unknown",
  "trace_id": "..."
}
```

## Configuration

The application uses hierarchical configuration loading:

1. **Default values** (hardcoded in `rf-config`)
2. **Environment-specific file** (e.g., `config/development.toml`)
3. **Environment variables** (e.g., `APP__SERVER__PORT=8080`)

### Configuration File

Create `config/default.toml`:

```toml
[server]
host = "127.0.0.1"
port = 3000
workers = 4
timeout = 30

[database]
url = "postgres://localhost/hello"
max_connections = 10
min_connections = 2
connect_timeout = 8

[auth]
jwt_secret = "dev-secret-change-in-production"
token_expiry_hours = 24
session_timeout_minutes = 60
```

### Environment Variables

Override config with environment variables:

```bash
APP__SERVER__PORT=8080 \
APP__SERVER__HOST="0.0.0.0" \
cargo run -p hello
```

Format: `APP__SECTION__KEY`

## Middleware Stack

The application uses the following middleware (from outermost to innermost):

1. **Request ID**: Generates unique trace ID per request
2. **Tracing**: Structured logging with trace IDs
3. **Timeout**: Request timeout (30s default)
4. **CORS**: Cross-origin resource sharing
5. **Compression**: Gzip/Brotli/Deflate response compression

## Dependency Injection

Services are registered in the DI container:

```rust
// Register AppConfig as singleton
container.register(Scope::Singleton, || Arc::new(config));

// Resolve in handlers
let config: Arc<AppConfig> = container.resolve()?;
```

## Testing

Run the example and test all endpoints:

```bash
# Terminal 1: Start server
cargo run -p hello

# Terminal 2: Test endpoints
curl http://localhost:3000/
curl http://localhost:3000/health
curl http://localhost:3000/ready
curl -X POST http://localhost:3000/echo \
  -H "Content-Type: application/json" \
  -d '{"message": "test"}'
```

## Architecture

```
┌─────────────────────────────────────────┐
│          Axum HTTP Server               │
└─────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│          Middleware Stack               │
│  • Request ID (trace_id)                │
│  • Tracing (structured logs)            │
│  • Timeout (30s)                        │
│  • CORS (cross-origin)                  │
│  • Compression (gzip/br)                │
└─────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│            Route Handlers               │
│  • / → hello_handler                    │
│  • /health → health_handler             │
│  • /ready → ready_handler               │
│  • /metrics → metrics_handler           │
│  • /echo → echo_handler                 │
└─────────────────────────────────────────┘
                  │
         ┌────────┴────────┐
         ▼                 ▼
┌──────────────┐   ┌──────────────┐
│  rf-config   │   │ rf-container │
│  (AppConfig) │   │ (DI Registry)│
└──────────────┘   └──────────────┘
         │                 │
         └────────┬────────┘
                  ▼
         ┌──────────────┐
         │   rf-core    │
         │ (AppError,   │
         │  Context)    │
         └──────────────┘
```

## Next Steps

This example demonstrates the foundation. Future examples will add:

- **Database integration** (SeaORM)
- **Authentication** (OIDC)
- **Job queues** (Redis/Postgres)
- **GraphQL** (async-graphql)
- **Email** (lettre)
- **Background jobs** (scheduler)
