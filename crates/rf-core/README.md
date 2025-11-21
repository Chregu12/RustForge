# rf-core

Core foundation for the RustForge framework.

## Features

- **RFC 7807 Problem Details**: Standard error responses for HTTP APIs
- **Request Context**: Trace IDs, path tracking, environment detection
- **Type-Safe Errors**: Compile-time checked error handling
- **Development/Production Modes**: Automatic error detail filtering

## Installation

```toml
[dependencies]
rf-core = "0.3"
```

## Usage

### Basic Error Handling

```rust
use rf_core::{AppError, AppResult, RequestContext};

fn get_user(id: i32) -> AppResult<User> {
    if id <= 0 {
        return Err(AppError::BadRequest {
            message: "ID must be positive".to_string(),
        });
    }

    User::find(id)
        .ok_or_else(|| AppError::NotFound {
            resource: format!("User {}", id),
        })
}
```

### Request Context

```rust
use rf_core::RequestContext;

fn handle_request() {
    let ctx = RequestContext::new("/api/users", "GET");

    println!("Trace ID: {}", ctx.trace_id());
    println!("Path: {}", ctx.path());

    if ctx.is_production() {
        // Hide sensitive error details
    }
}
```

### RFC 7807 Problem Details

```rust
use rf_core::{AppError, RequestContext};

let ctx = RequestContext::new("/api/users/123", "GET");
let error = AppError::NotFound {
    resource: "User 123".to_string(),
};

let problem = error.to_problem_details(&ctx);

// Serialize to JSON
let json = serde_json::to_string_pretty(&problem)?;
```

JSON output:
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

### Error Types

| Error Variant | HTTP Status | Use Case |
|--------------|-------------|----------|
| `Validation` | 422 | Form validation failures |
| `NotFound` | 404 | Resource doesn't exist |
| `Unauthorized` | 401 | Authentication required |
| `Forbidden` | 403 | Insufficient permissions |
| `BadRequest` | 400 | Invalid input |
| `Conflict` | 409 | Resource conflict (e.g., duplicate email) |
| `RateLimitExceeded` | 429 | Too many requests |
| `Internal` | 500 | Unexpected errors |
| `ServiceUnavailable` | 503 | Service down (e.g., database) |

### Environment Detection

The framework automatically detects the environment from the `APP_ENV` variable:

```bash
# Development (default) - shows full error details
APP_ENV=development cargo run

# Production - hides sensitive information
APP_ENV=production cargo run

# Staging
APP_ENV=staging cargo run
```

### Integration with Axum (in rf-web)

```rust
use axum::{extract::Path, Json};
use rf_core::{AppError, AppResult, RequestContext};

async fn get_user(
    ctx: RequestContext,
    Path(id): Path<i32>,
) -> AppResult<Json<User>> {
    let user = User::find(id).await?
        .ok_or_else(|| AppError::NotFound {
            resource: format!("User {}", id),
        })?;

    Ok(Json(user))
}
```

## Features

### `validation`

Enables integration with the `validator` crate for automatic validation error handling.

```toml
[dependencies]
rf-core = { version = "0.3", features = ["validation"] }
```

```rust
use rf_core::{AppError, AppResult};
use validator::Validate;

#[derive(Validate)]
struct CreateUser {
    #[validate(email)]
    email: String,

    #[validate(length(min = 8))]
    password: String,
}

fn create_user(input: CreateUser) -> AppResult<User> {
    input.validate()?; // Automatically converts to AppError::Validation
    // ...
}
```

## Testing

Run tests:

```bash
cargo test -p rf-core
```

Run with all features:

```bash
cargo test -p rf-core --all-features
```

## Documentation

Build and open documentation:

```bash
cargo doc -p rf-core --open
```

## License

MIT
