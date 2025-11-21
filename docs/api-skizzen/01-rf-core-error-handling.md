# API-Skizze: rf-core Error Handling (RFC 7807)

**Crate:** `rf-core`
**Module:** `rf_core::error`

## Ziel

Type-safe, RFC 7807-konformes Error-Handling mit:
- Klare Error-Hierarchie
- HTTP-Status-Mapping
- Trace-ID-Injection
- Development vs Production Modus

---

## Core Types

### AppError (Main Error Type)

```rust
// rf-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    /// Validation errors (422 Unprocessable Entity)
    #[error("Validation failed")]
    Validation(#[from] ValidationErrors),

    /// Resource not found (404)
    #[error("Resource not found: {resource}")]
    NotFound {
        resource: String,
    },

    /// Unauthorized access (401)
    #[error("Unauthorized")]
    Unauthorized,

    /// Forbidden access (403)
    #[error("Forbidden: {reason}")]
    Forbidden {
        reason: String,
    },

    /// Bad request (400)
    #[error("Bad request: {message}")]
    BadRequest {
        message: String,
    },

    /// Conflict (409)
    #[error("Conflict: {message}")]
    Conflict {
        message: String,
    },

    /// Too many requests (429)
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Internal server error (500)
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    /// Service unavailable (503)
    #[error("Service unavailable: {service}")]
    ServiceUnavailable {
        service: String,
    },

    /// Database error (500)
    #[error("Database error")]
    Database(#[from] sea_orm::DbErr),
}

impl AppError {
    /// Convert to RFC 7807 Problem Details
    pub fn to_problem_details(&self, ctx: &RequestContext) -> ProblemDetails {
        match self {
            Self::Validation(errors) => ProblemDetails {
                type_uri: "https://api.example.com/errors/validation-failed".to_string(),
                title: "Validation Failed".to_string(),
                status: 422,
                detail: "One or more fields failed validation".to_string(),
                instance: ctx.path().to_string(),
                trace_id: ctx.trace_id().to_string(),
                extensions: {
                    let mut map = serde_json::Map::new();
                    map.insert("errors".to_string(), serde_json::to_value(errors).unwrap());
                    map
                },
            },

            Self::NotFound { resource } => ProblemDetails {
                type_uri: "https://api.example.com/errors/not-found".to_string(),
                title: "Not Found".to_string(),
                status: 404,
                detail: format!("{} not found", resource),
                instance: ctx.path().to_string(),
                trace_id: ctx.trace_id().to_string(),
                extensions: Default::default(),
            },

            Self::Unauthorized => ProblemDetails {
                type_uri: "https://api.example.com/errors/unauthorized".to_string(),
                title: "Unauthorized".to_string(),
                status: 401,
                detail: "Authentication required".to_string(),
                instance: ctx.path().to_string(),
                trace_id: ctx.trace_id().to_string(),
                extensions: Default::default(),
            },

            Self::Forbidden { reason } => ProblemDetails {
                type_uri: "https://api.example.com/errors/forbidden".to_string(),
                title: "Forbidden".to_string(),
                status: 403,
                detail: reason.clone(),
                instance: ctx.path().to_string(),
                trace_id: ctx.trace_id().to_string(),
                extensions: Default::default(),
            },

            Self::Internal(err) => {
                // Log full error for debugging
                tracing::error!(error = ?err, "Internal server error");

                ProblemDetails {
                    type_uri: "https://api.example.com/errors/internal-error".to_string(),
                    title: "Internal Server Error".to_string(),
                    status: 500,
                    detail: if ctx.is_development() {
                        format!("{:?}", err)
                    } else {
                        "An internal error occurred".to_string()
                    },
                    instance: ctx.path().to_string(),
                    trace_id: ctx.trace_id().to_string(),
                    extensions: if ctx.is_development() {
                        let mut map = serde_json::Map::new();
                        map.insert("backtrace".to_string(), format!("{:?}", err).into());
                        map
                    } else {
                        Default::default()
                    },
                }
            }

            _ => self.default_problem_details(ctx),
        }
    }

    fn default_problem_details(&self, ctx: &RequestContext) -> ProblemDetails {
        ProblemDetails {
            type_uri: "https://api.example.com/errors/unknown".to_string(),
            title: "Error".to_string(),
            status: 500,
            detail: self.to_string(),
            instance: ctx.path().to_string(),
            trace_id: ctx.trace_id().to_string(),
            extensions: Default::default(),
        }
    }

    /// Get HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Validation(_) => 422,
            Self::NotFound { .. } => 404,
            Self::Unauthorized => 401,
            Self::Forbidden { .. } => 403,
            Self::BadRequest { .. } => 400,
            Self::Conflict { .. } => 409,
            Self::RateLimitExceeded => 429,
            Self::ServiceUnavailable { .. } => 503,
            Self::Internal(_) | Self::Database(_) => 500,
        }
    }
}
```

---

### ProblemDetails (RFC 7807)

```rust
// rf-core/src/error/problem_details.rs
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type
    #[serde(rename = "type")]
    pub type_uri: String,

    /// A short, human-readable summary
    pub title: String,

    /// HTTP status code
    pub status: u16,

    /// Human-readable explanation specific to this occurrence
    pub detail: String,

    /// URI reference identifying the specific occurrence
    pub instance: String,

    /// Trace ID for log correlation
    pub trace_id: String,

    /// Additional problem-specific fields
    #[serde(flatten)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl ProblemDetails {
    pub fn new(status: u16, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            type_uri: format!("https://api.example.com/errors/{}", status),
            title: title.into(),
            status,
            detail: detail.into(),
            instance: "/".to_string(),
            trace_id: "".to_string(),
            extensions: Default::default(),
        }
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = trace_id.into();
        self
    }

    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = instance.into();
        self
    }

    pub fn with_extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }
}
```

---

### AppResult (Convenience Type Alias)

```rust
// rf-core/src/error/result.rs
pub type AppResult<T> = Result<T, AppError>;
```

---

### RequestContext (for trace_id, path, env)

```rust
// rf-core/src/context.rs
use uuid::Uuid;

#[derive(Clone)]
pub struct RequestContext {
    trace_id: String,
    path: String,
    method: String,
    environment: Environment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl RequestContext {
    pub fn new(path: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            path: path.into(),
            method: method.into(),
            environment: Self::detect_environment(),
        }
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn is_development(&self) -> bool {
        self.environment == Environment::Development
    }

    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }

    fn detect_environment() -> Environment {
        match std::env::var("APP_ENV").as_deref() {
            Ok("production") => Environment::Production,
            Ok("staging") => Environment::Staging,
            _ => Environment::Development,
        }
    }
}
```

---

## Usage Examples

### Example 1: Handler mit AppResult

```rust
use rf_core::error::{AppError, AppResult};
use rf_core::context::RequestContext;
use axum::{extract::Path, Json};

pub async fn get_user(
    ctx: RequestContext,
    Path(id): Path<i32>,
) -> AppResult<Json<User>> {
    let user = User::find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            resource: format!("User {}", id),
        })?;

    Ok(Json(user))
}
```

### Example 2: Validation Error

```rust
use rf_core::error::{AppError, AppResult};
use validator::Validate;

#[derive(Validate)]
pub struct CreateUserRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8))]
    pub password: String,
}

pub async fn create_user(
    Json(req): Json<CreateUserRequest>,
) -> AppResult<Json<User>> {
    req.validate()
        .map_err(AppError::from)?;

    // ...
}
```

### Example 3: Custom Error Response

```rust
if user.is_banned() {
    return Err(AppError::Forbidden {
        reason: "User account is banned".to_string(),
    });
}

if !user.has_permission("posts:create") {
    return Err(AppError::Forbidden {
        reason: "Insufficient permissions".to_string(),
    });
}
```

---

## HTTP Response Format

### Success Response (200 OK)

```json
{
  "id": 1,
  "email": "user@example.com",
  "created_at": "2024-01-01T00:00:00Z"
}
```

### Error Response (422 Unprocessable Entity)

```json
{
  "type": "https://api.example.com/errors/validation-failed",
  "title": "Validation Failed",
  "status": 422,
  "detail": "One or more fields failed validation",
  "instance": "/api/users",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "errors": {
    "email": ["must be a valid email address"],
    "password": ["must be at least 8 characters"]
  }
}
```

### Error Response (404 Not Found)

```json
{
  "type": "https://api.example.com/errors/not-found",
  "title": "Not Found",
  "status": 404,
  "detail": "User 123 not found",
  "instance": "/api/users/123",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### Error Response (500 Internal Server Error - Development)

```json
{
  "type": "https://api.example.com/errors/internal-error",
  "title": "Internal Server Error",
  "status": 500,
  "detail": "Database connection failed: connection timeout",
  "instance": "/api/users",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "backtrace": "Error: Database connection failed\n  at ..."
}
```

### Error Response (500 Internal Server Error - Production)

```json
{
  "type": "https://api.example.com/errors/internal-error",
  "title": "Internal Server Error",
  "status": 500,
  "detail": "An internal error occurred",
  "instance": "/api/users",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

---

## Integration with Axum

### IntoResponse Implementation (in rf-web)

```rust
// rf-web/src/response.rs
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use axum::Json;
use rf_core::error::AppError;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Extract RequestContext from request extensions
        let ctx = /* get from extensions or create default */;

        let problem = self.to_problem_details(&ctx);
        let status = StatusCode::from_u16(problem.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Json(problem)).into_response()
    }
}
```

---

## Testing

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let ctx = RequestContext::new("/api/users/123", "GET");
        let error = AppError::NotFound {
            resource: "User 123".to_string(),
        };

        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert!(problem.detail.contains("User 123"));
        assert_eq!(problem.instance, "/api/users/123");
        assert!(!problem.trace_id.is_empty());
    }

    #[test]
    fn test_internal_error_hides_details_in_production() {
        let mut ctx = RequestContext::new("/api/users", "GET");
        ctx.environment = Environment::Production;

        let error = AppError::Internal(anyhow::anyhow!("Database password: secret123"));
        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 500);
        assert_eq!(problem.detail, "An internal error occurred");
        assert!(!problem.detail.contains("secret123"));
        assert!(problem.extensions.is_empty());
    }

    #[test]
    fn test_internal_error_shows_details_in_development() {
        let ctx = RequestContext::new("/api/users", "GET");
        // Default is Development

        let error = AppError::Internal(anyhow::anyhow!("Connection timeout"));
        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 500);
        assert!(problem.detail.contains("Connection timeout"));
        assert!(problem.extensions.contains_key("backtrace"));
    }
}
```

---

## Dependencies

```toml
[dependencies]
thiserror = "1.0"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
tracing = "0.1"
validator = { version = "0.16", optional = true }
```
