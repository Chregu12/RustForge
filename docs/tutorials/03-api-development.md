# API Development with RustForge

**Time:** 2 hours
**Difficulty:** Intermediate
**Prerequisites:** Complete Getting Started Tutorial, basic understanding of REST

---

## Table of Contents

1. [Introduction](#introduction)
2. [RESTful API Design](#restful-api-design)
3. [Creating API Routes](#creating-api-routes)
4. [API Resources](#api-resources)
5. [Authentication with Tokens](#authentication-with-tokens)
6. [Rate Limiting](#rate-limiting)
7. [Validation](#validation)
8. [Error Handling](#error-handling)
9. [API Documentation](#api-documentation)
10. [Testing APIs](#testing-apis)

---

## Introduction

This tutorial teaches you how to build production-ready RESTful APIs with RustForge. You'll learn API best practices, authentication, rate limiting, and testing.

### What You'll Build

A complete Task API with:
- CRUD operations for tasks
- Token-based authentication
- Rate limiting
- Request validation
- Structured error responses
- API documentation
- Comprehensive tests

---

## RESTful API Design

### REST Principles

**RESTful APIs use HTTP methods semantically:**

| Method | Purpose | Example |
|--------|---------|---------|
| GET | Retrieve resources | `GET /api/tasks` |
| POST | Create resource | `POST /api/tasks` |
| PUT/PATCH | Update resource | `PUT /api/tasks/1` |
| DELETE | Delete resource | `DELETE /api/tasks/1` |

### API Structure

```
/api/v1/
  /tasks
    GET    /           - List all tasks
    POST   /           - Create new task
    GET    /:id        - Get specific task
    PUT    /:id        - Update task
    DELETE /:id        - Delete task
  /users
    POST   /register   - Register user
    POST   /login      - Login
    GET    /me         - Current user
```

---

## Creating API Routes

### Project Setup

```bash
forge new task-api
cd task-api
```

### Define API Routes

Create `src/api/routes.rs`:

```rust
use axum::{Router, routing::{get, post, put, delete}};
use crate::api::controllers::task_controller;
use crate::api::middleware::auth;

pub fn api_routes() -> Router {
    Router::new()
        .route("/tasks", get(task_controller::index))
        .route("/tasks", post(task_controller::store))
        .route("/tasks/:id", get(task_controller::show))
        .route("/tasks/:id", put(task_controller::update))
        .route("/tasks/:id", delete(task_controller::destroy))
        .layer(axum::middleware::from_fn(auth::require_auth))
}
```

### Register in Main Application

Update `src/main.rs`:

```rust
mod api;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest("/api/v1", api::routes::api_routes());

    // Start server...
}
```

---

## API Resources

API Resources transform models into JSON responses with consistent structure.

### Create Task Resource

Create `src/api/resources/task_resource.rs`:

```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::models::Task;

#[derive(Serialize, Deserialize)]
pub struct TaskResource {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: i32,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Task> for TaskResource {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            title: task.title,
            description: task.description,
            status: task.status,
            priority: task.priority,
            due_date: task.due_date,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct TaskCollection {
    pub data: Vec<TaskResource>,
    pub meta: Meta,
}

#[derive(Serialize)]
pub struct Meta {
    pub total: i64,
    pub per_page: i64,
    pub current_page: i64,
    pub last_page: i64,
}
```

### Use Resources in Controller

Create `src/api/controllers/task_controller.rs`:

```rust
use axum::{Json, extract::{Path, State}};
use crate::api::resources::task_resource::{TaskResource, TaskCollection};
use crate::models::Task;
use crate::AppState;

/// GET /api/v1/tasks
pub async fn index(
    State(state): State<AppState>,
) -> Json<TaskCollection> {
    let tasks = Task::all(&state.db).await.unwrap();
    let total = tasks.len() as i64;

    let resources: Vec<TaskResource> = tasks
        .into_iter()
        .map(TaskResource::from)
        .collect();

    Json(TaskCollection {
        data: resources,
        meta: Meta {
            total,
            per_page: 15,
            current_page: 1,
            last_page: (total as f64 / 15.0).ceil() as i64,
        },
    })
}

/// GET /api/v1/tasks/:id
pub async fn show(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<TaskResource>, ApiError> {
    let task = Task::find(id, &state.db)
        .await
        .map_err(|_| ApiError::NotFound)?;

    Ok(Json(TaskResource::from(task)))
}

/// POST /api/v1/tasks
pub async fn store(
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<TaskResource>, ApiError> {
    // Validate
    payload.validate()?;

    // Create task
    let task = Task::create(&state.db, payload).await?;

    Ok(Json(TaskResource::from(task)))
}

/// PUT /api/v1/tasks/:id
pub async fn update(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResource>, ApiError> {
    payload.validate()?;

    let task = Task::find(id, &state.db)
        .await
        .map_err(|_| ApiError::NotFound)?;

    let updated = task.update(&state.db, payload).await?;

    Ok(Json(TaskResource::from(updated)))
}

/// DELETE /api/v1/tasks/:id
pub async fn destroy(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<SuccessResponse>, ApiError> {
    let task = Task::find(id, &state.db)
        .await
        .map_err(|_| ApiError::NotFound)?;

    task.delete(&state.db).await?;

    Ok(Json(SuccessResponse {
        message: "Task deleted successfully".to_string(),
    }))
}
```

---

## Authentication with Tokens

### Bearer Token Authentication

Create `src/api/middleware/auth.rs`:

```rust
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::StatusCode,
};
use crate::AppState;
use crate::models::User;

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token and get user
    let user = User::find_by_token(token, &state.db)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add user to request extensions
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}
```

### Login Endpoint

Create `src/api/controllers/auth_controller.rs`:

```rust
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::models::User;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResource,
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    // Find user
    let user = User::find_by_email(&payload.email, &state.db)
        .await
        .map_err(|_| ApiError::Unauthorized("Invalid credentials"))?;

    // Verify password
    if !user.verify_password(&payload.password) {
        return Err(ApiError::Unauthorized("Invalid credentials"));
    }

    // Generate token
    let token = user.generate_token()?;

    Ok(Json(LoginResponse {
        token,
        user: UserResource::from(user),
    }))
}
```

### Using Authentication

In your controller, access the authenticated user:

```rust
use axum::Extension;

pub async fn index(
    Extension(user): Extension<User>, // Injected by auth middleware
    State(state): State<AppState>,
) -> Json<TaskCollection> {
    // Only show tasks owned by authenticated user
    let tasks = Task::where_user_id(user.id, &state.db).await.unwrap();
    // ...
}
```

---

## Rate Limiting

Protect your API from abuse with rate limiting.

### Install Rate Limiter

Add to `Cargo.toml`:

```toml
tower-governor = "0.3"
```

### Configure Rate Limiting

Create `src/api/middleware/rate_limit.rs`:

```rust
use tower_governor::{
    governor::GovernorConfigBuilder,
    GovernorLayer,
};
use std::time::Duration;

pub fn rate_limiter() -> GovernorLayer {
    let config = GovernorConfigBuilder::default()
        .per_second(10) // 10 requests per second
        .burst_size(20) // Allow bursts up to 20
        .finish()
        .unwrap();

    GovernorLayer {
        config: Arc::new(config),
    }
}
```

### Apply to Routes

```rust
pub fn api_routes() -> Router {
    Router::new()
        .route("/tasks", get(task_controller::index))
        // ... other routes
        .layer(rate_limiter()) // Apply rate limiting
}
```

---

## Validation

### Create Request Validators

Create `src/api/requests/task_request.rs`:

```rust
use serde::Deserialize;
use validator::Validate;
use chrono::{DateTime, Utc};

#[derive(Deserialize, Validate)]
pub struct CreateTaskRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    #[validate(custom = "validate_status")]
    pub status: String,

    #[validate(range(min = 1, max = 5))]
    pub priority: i32,

    pub due_date: Option<DateTime<Utc>>,
}

fn validate_status(status: &str) -> Result<(), ValidationError> {
    match status {
        "pending" | "in_progress" | "completed" => Ok(()),
        _ => Err(ValidationError::new("Invalid status")),
    }
}

impl CreateTaskRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        use validator::Validate;
        self.validate()
            .map_err(|e| ApiError::ValidationError(e.to_string()))
    }
}
```

---

## Error Handling

### Standardized Error Responses

Create `src/api/errors.rs`:

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    Unauthorized(&'static str),
    ValidationError(String),
    DatabaseError(String),
    InternalError,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error, message, details) = match self {
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "Resource not found",
                None,
            ),
            ApiError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                msg,
                None,
            ),
            ApiError::ValidationError(details) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_error",
                "Validation failed",
                Some(details),
            ),
            ApiError::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "Database error",
                Some(msg),
            ),
            ApiError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Internal server error",
                None,
            ),
        };

        let body = Json(ErrorResponse {
            error: error.to_string(),
            message: message.to_string(),
            details,
        });

        (status, body).into_response()
    }
}
```

---

## API Documentation

### Using OpenAPI/Swagger

Add to `Cargo.toml`:

```toml
utoipa = { version = "4.0", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "4.0", features = ["axum"] }
```

### Document Your API

```rust
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    paths(
        task_controller::index,
        task_controller::show,
        task_controller::store,
        task_controller::update,
        task_controller::destroy,
    ),
    components(schemas(TaskResource, CreateTaskRequest))
)]
struct ApiDoc;

/// GET /api/v1/tasks - List all tasks
#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    responses(
        (status = 200, description = "List of tasks", body = TaskCollection),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_token" = []))
)]
pub async fn index() -> Json<TaskCollection> {
    // ...
}
```

### Serve Documentation

```rust
use utoipa_swagger_ui::SwaggerUi;

let app = Router::new()
    .nest("/api/v1", api_routes())
    .merge(SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi()));
```

Visit `http://localhost:8000/swagger-ui` to see interactive API docs!

---

## Testing APIs

### Integration Tests

Create `tests/api/task_api_test.rs`:

```rust
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use serde_json::json;

#[tokio::test]
async fn test_create_task() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "Test Task",
                        "description": "Test Description",
                        "status": "pending",
                        "priority": 1
                    }).to_string()
                ))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let task: TaskResource = serde_json::from_slice(&body).unwrap();

    assert_eq!(task.title, "Test Task");
    assert_eq!(task.status, "pending");
}

#[tokio::test]
async fn test_list_tasks() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let collection: TaskCollection = serde_json::from_slice(&body).unwrap();

    assert!(collection.data.len() > 0);
}

#[tokio::test]
async fn test_unauthorized_access() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
```

---

## Complete Example

Here's the full structure of a production-ready API:

```
task-api/
├── src/
│   ├── main.rs
│   └── api/
│       ├── mod.rs
│       ├── routes.rs
│       ├── controllers/
│       │   ├── mod.rs
│       │   ├── task_controller.rs
│       │   └── auth_controller.rs
│       ├── resources/
│       │   ├── mod.rs
│       │   └── task_resource.rs
│       ├── requests/
│       │   ├── mod.rs
│       │   └── task_request.rs
│       ├── middleware/
│       │   ├── mod.rs
│       │   ├── auth.rs
│       │   └── rate_limit.rs
│       └── errors.rs
└── tests/
    └── api/
        └── task_api_test.rs
```

---

## Best Practices

1. **Versioning:** Always version your API (`/api/v1`)
2. **Pagination:** Paginate large collections
3. **Filtering:** Support query parameters for filtering
4. **CORS:** Configure CORS for browser clients
5. **Rate Limiting:** Protect against abuse
6. **Authentication:** Use tokens, not sessions
7. **Validation:** Validate all inputs
8. **Error Handling:** Return structured errors
9. **Documentation:** Keep docs up to date
10. **Testing:** Test all endpoints

---

## Summary

You learned how to:

- ✅ Design RESTful APIs
- ✅ Create API routes and controllers
- ✅ Use API resources for responses
- ✅ Implement token authentication
- ✅ Add rate limiting
- ✅ Validate requests
- ✅ Handle errors gracefully
- ✅ Document with OpenAPI
- ✅ Write integration tests

**Next Steps:**

- [Advanced Features Tutorial](./04-advanced-features.md)
- [Testing Tutorial](./05-testing.md)
- [API Resources Guide](../guides/api-resources.md)

---

**Time to complete:** ~2 hours ✅
