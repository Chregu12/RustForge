# API-Skizze: rf-web Axum Integration & Middleware

**Crate:** `rf-web`
**Module:** `rf_web`

## Ziel

Production-ready Web-Layer mit Axum + Tower:
- IntoResponse für AppError (RFC 7807 HTTP responses)
- Middleware-Stack (Tracing, RequestId, CORS, Timeout, Compression)
- RouterBuilder für ergonomisches Setup
- Extractor für RequestContext

---

## Core Types

### 1. IntoResponse für AppError

```rust
// rf-web/src/response.rs
use axum::response::{IntoResponse, Response};
use axum::http::{StatusCode, HeaderValue};
use axum::Json;
use rf_core::{AppError, RequestContext};

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Extract RequestContext from request extensions
        // Fallback to default if not available
        let ctx = RequestContext::new("/unknown", "UNKNOWN");

        let problem = self.to_problem_details(&ctx);
        let status = StatusCode::from_u16(problem.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // Return RFC 7807 JSON response
        (
            status,
            [(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/problem+json"),
            )],
            Json(problem),
        )
            .into_response()
    }
}
```

### 2. RequestContext Extractor

```rust
// rf-web/src/extractors/request_context.rs
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rf_core::RequestContext;

#[async_trait]
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to get from extensions (set by middleware)
        if let Some(ctx) = parts.extensions.get::<RequestContext>() {
            return Ok(ctx.clone());
        }

        // Fallback: create new context
        let path = parts.uri.path().to_string();
        let method = parts.method.as_str().to_string();

        Ok(RequestContext::new(path, method))
    }
}
```

---

## Middleware Stack

### 1. RequestIdMiddleware (Trace ID Injection)

```rust
// rf-web/src/middleware/request_id.rs
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use rf_core::RequestContext;

/// Middleware to inject trace ID into request context
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    // Create RequestContext with unique trace ID
    let ctx = RequestContext::new(
        request.uri().path(),
        request.method().as_str(),
    );

    // Store in extensions for handlers to use
    request.extensions_mut().insert(ctx.clone());

    // Add trace ID to response headers
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-trace-id",
        ctx.trace_id().parse().unwrap(),
    );

    response
}

/// Layer wrapper
pub fn request_id_layer() -> axum::middleware::FromFnLayer<
    impl Fn(Request, Next) -> impl Future<Output = Response> + Clone,
> {
    axum::middleware::from_fn(request_id_middleware)
}
```

### 2. TracingMiddleware (OpenTelemetry)

```rust
// rf-web/src/middleware/tracing.rs
use tower_http::trace::{TraceLayer, DefaultMakeSpan, DefaultOnResponse};
use tracing::Level;

/// Create tracing middleware layer
pub fn tracing_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
> {
    TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(true)
        )
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .include_headers(true)
        )
}
```

### 3. CorsMiddleware

```rust
// rf-web/src/middleware/cors.rs
use tower_http::cors::{CorsLayer, Any};
use axum::http::Method;

/// CORS configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<Method>,
    pub allowed_headers: Vec<String>,
    pub max_age: Option<std::time::Duration>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "x-trace-id".to_string(),
            ],
            max_age: Some(std::time::Duration::from_secs(3600)),
        }
    }
}

/// Create CORS layer
pub fn cors_layer(config: CorsConfig) -> CorsLayer {
    let mut layer = CorsLayer::new();

    // Origins
    if config.allowed_origins.contains(&"*".to_string()) {
        layer = layer.allow_origin(Any);
    } else {
        let origins: Vec<_> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer = layer.allow_origin(origins);
    }

    // Methods
    layer = layer.allow_methods(config.allowed_methods);

    // Headers
    let headers: Vec<_> = config
        .allowed_headers
        .iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    layer = layer.allow_headers(headers);

    // Max age
    if let Some(max_age) = config.max_age {
        layer = layer.max_age(max_age);
    }

    layer
}
```

### 4. TimeoutMiddleware

```rust
// rf-web/src/middleware/timeout.rs
use tower::timeout::TimeoutLayer;
use std::time::Duration;

/// Create timeout layer
///
/// Returns 408 Request Timeout if handler takes longer than duration
pub fn timeout_layer(duration: Duration) -> TimeoutLayer {
    TimeoutLayer::new(duration)
}

/// Default timeout (30 seconds)
pub fn default_timeout_layer() -> TimeoutLayer {
    timeout_layer(Duration::from_secs(30))
}
```

### 5. CompressionMiddleware

```rust
// rf-web/src/middleware/compression.rs
use tower_http::compression::{CompressionLayer, predicate::SizeAbove};

/// Create compression layer
///
/// Compresses responses larger than 1KB using gzip/brotli
pub fn compression_layer() -> CompressionLayer {
    CompressionLayer::new()
        .gzip(true)
        .br(true)
        .deflate(true)
        .compress_when(SizeAbove::new(1024)) // Only compress > 1KB
}
```

---

## RouterBuilder (Ergonomic Setup)

```rust
// rf-web/src/router.rs
use axum::{Router, routing};
use tower::ServiceBuilder;
use std::time::Duration;

/// Builder for creating routers with standard middleware
pub struct RouterBuilder {
    router: Router,
    enable_tracing: bool,
    enable_cors: bool,
    enable_compression: bool,
    enable_timeout: bool,
    timeout_duration: Duration,
    cors_config: CorsConfig,
}

impl RouterBuilder {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            enable_tracing: true,
            enable_cors: true,
            enable_compression: true,
            enable_timeout: true,
            timeout_duration: Duration::from_secs(30),
            cors_config: CorsConfig::default(),
        }
    }

    /// Add a route
    pub fn route(mut self, path: &str, method_router: routing::MethodRouter) -> Self {
        self.router = self.router.route(path, method_router);
        self
    }

    /// Nest routes under a prefix
    pub fn nest(mut self, path: &str, router: Router) -> Self {
        self.router = self.router.nest(path, router);
        self
    }

    /// Enable/disable tracing
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    /// Enable/disable CORS
    pub fn with_cors(mut self, enable: bool) -> Self {
        self.enable_cors = enable;
        self
    }

    /// Configure CORS
    pub fn cors_config(mut self, config: CorsConfig) -> Self {
        self.cors_config = config;
        self
    }

    /// Enable/disable compression
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// Enable/disable timeout
    pub fn with_timeout(mut self, enable: bool) -> Self {
        self.enable_timeout = enable;
        self
    }

    /// Set timeout duration
    pub fn timeout_duration(mut self, duration: Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    /// Build the router with configured middleware
    pub fn build(self) -> Router {
        let mut router = self.router;

        // Layer order matters: first added = outermost layer
        let mut layers = ServiceBuilder::new();

        // RequestId (outermost - so it's available to all layers)
        layers = layers.layer(request_id_layer());

        // Tracing
        if self.enable_tracing {
            layers = layers.layer(tracing_layer());
        }

        // Timeout
        if self.enable_timeout {
            layers = layers.layer(timeout_layer(self.timeout_duration));
        }

        // CORS
        if self.enable_cors {
            layers = layers.layer(cors_layer(self.cors_config));
        }

        // Compression (innermost - so it compresses after handler)
        if self.enable_compression {
            layers = layers.layer(compression_layer());
        }

        router.layer(layers)
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## Usage Examples

### Example 1: Basic Handler with Error Handling

```rust
use axum::{extract::Path, Json};
use rf_core::{AppError, AppResult, RequestContext};
use rf_web::RouterBuilder;

#[derive(serde::Deserialize, serde::Serialize)]
struct User {
    id: i32,
    name: String,
}

async fn get_user(
    ctx: RequestContext,
    Path(id): Path<i32>,
) -> AppResult<Json<User>> {
    tracing::info!(trace_id = %ctx.trace_id(), user_id = id, "Fetching user");

    if id <= 0 {
        return Err(AppError::BadRequest {
            message: "ID must be positive".to_string(),
        });
    }

    // Simulate database lookup
    let user = User {
        id,
        name: "John Doe".to_string(),
    };

    Ok(Json(user))
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Build router with all middleware
    let app = RouterBuilder::new()
        .route("/users/:id", axum::routing::get(get_user))
        .with_tracing(true)
        .with_cors(true)
        .with_compression(true)
        .with_timeout(true)
        .build();

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Example 2: Custom CORS Configuration

```rust
use rf_web::{RouterBuilder, CorsConfig};
use axum::http::Method;
use std::time::Duration;

let cors_config = CorsConfig {
    allowed_origins: vec![
        "https://app.example.com".to_string(),
        "https://admin.example.com".to_string(),
    ],
    allowed_methods: vec![Method::GET, Method::POST],
    allowed_headers: vec!["content-type".to_string(), "authorization".to_string()],
    max_age: Some(Duration::from_secs(7200)),
};

let app = RouterBuilder::new()
    .route("/api/users", axum::routing::get(list_users))
    .cors_config(cors_config)
    .build();
```

### Example 3: Minimal Router (No Middleware)

```rust
let app = RouterBuilder::new()
    .route("/health", axum::routing::get(health_check))
    .with_tracing(false)
    .with_cors(false)
    .with_compression(false)
    .with_timeout(false)
    .build();
```

### Example 4: Nested Routes

```rust
let api_routes = RouterBuilder::new()
    .route("/users", axum::routing::get(list_users))
    .route("/users/:id", axum::routing::get(get_user))
    .build();

let app = RouterBuilder::new()
    .nest("/api/v1", api_routes)
    .route("/health", axum::routing::get(health_check))
    .build();
```

---

## HTTP Response Examples

### Success Response (200 OK)

**Request:**
```http
GET /users/123 HTTP/1.1
Host: api.example.com
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json
X-Trace-Id: 550e8400-e29b-41d4-a716-446655440000
Content-Encoding: gzip

{
  "id": 123,
  "name": "John Doe"
}
```

### Error Response (404 Not Found)

**Request:**
```http
GET /users/999 HTTP/1.1
Host: api.example.com
```

**Response:**
```http
HTTP/1.1 404 Not Found
Content-Type: application/problem+json
X-Trace-Id: 550e8400-e29b-41d4-a716-446655440000

{
  "type": "https://api.example.com/errors/not-found",
  "title": "Not Found",
  "status": 404,
  "detail": "User 999 not found",
  "instance": "/users/999",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Error Response (408 Request Timeout)

**Request:**
```http
GET /slow-endpoint HTTP/1.1
Host: api.example.com
```

**Response:**
```http
HTTP/1.1 408 Request Timeout
Content-Type: application/problem+json
X-Trace-Id: 550e8400-e29b-41d4-a716-446655440000

{
  "type": "https://api.example.com/errors/timeout",
  "title": "Request Timeout",
  "status": 408,
  "detail": "Request exceeded 30 second timeout",
  "instance": "/slow-endpoint",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

---

## Testing

### Unit Test: IntoResponse

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::body::Body;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_not_found_into_response() {
        let error = AppError::NotFound {
            resource: "User 123".to_string(),
        };

        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(content_type, "application/problem+json");
    }
}
```

### Integration Test: Full Request

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_get_user_not_found() {
        let app = RouterBuilder::new()
            .route("/users/:id", axum::routing::get(get_user))
            .build();

        let request = Request::builder()
            .uri("/users/999")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let problem: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(problem["status"], 404);
        assert_eq!(problem["title"], "Not Found");
    }

    #[tokio::test]
    async fn test_trace_id_header() {
        let app = RouterBuilder::new()
            .route("/health", axum::routing::get(|| async { "OK" }))
            .build();

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let trace_id = response.headers().get("x-trace-id");
        assert!(trace_id.is_some());
        assert!(!trace_id.unwrap().to_str().unwrap().is_empty());
    }
}
```

---

## Dependencies

```toml
[dependencies]
rf-core = { path = "../rf-core" }

# Axum & Tower
axum = "0.7"
tower = { version = "0.4", features = ["full"] }
tower-http = { version = "0.5", features = ["full"] }
hyper = { version = "1.0", features = ["full"] }

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"

[dev-dependencies]
tokio-test = "0.4"
```

---

## Performance Considerations

### Middleware Order
```
Request → RequestId → Tracing → Timeout → CORS → Compression → Handler
Handler → Compression → CORS → Timeout → Tracing → RequestId → Response
```

**Why this order:**
1. **RequestId first**: So trace ID is available to all downstream layers
2. **Tracing second**: Captures all middleware timing
3. **Timeout third**: Applies to handler + compression
4. **CORS fourth**: Headers added before compression
5. **Compression last**: Compress final response

### Compression Threshold
- Only compress responses > 1KB
- Avoids overhead for small responses
- Automatic content-type detection

### Timeout Defaults
- Default: 30 seconds
- Health checks: No timeout (or very high)
- Long operations: Custom timeout per route

---

## Security Considerations

### CORS
- **Production**: Whitelist specific origins
- **Development**: Allow all (`*`)
- Always validate `allowed_methods` and `allowed_headers`

### Headers
- `X-Trace-Id`: Safe to expose (no sensitive data)
- `X-Request-Id`: Alias for trace ID
- Never expose internal error details in production

### Timeout
- Prevents resource exhaustion from slow clients
- Returns 408 (not 500) for timeout errors
- Configurable per-route for long operations
