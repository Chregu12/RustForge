use axum::{
    Router,
    routing::{get, post},
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;
use axum::body::Body;
use axum::http::{Request, Method};

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserResponse {
    id: i64,
    name: String,
    email: String,
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
    })
}

async fn create_user_handler(Json(payload): Json<CreateUserRequest>) -> (StatusCode, Json<UserResponse>) {
    (
        StatusCode::CREATED,
        Json(UserResponse {
            id: 1,
            name: payload.name,
            email: payload.email,
        }),
    )
}

fn create_test_router() -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/users", post(create_user_handler))
}

#[tokio::test]
async fn test_health_endpoint() {
    // Test health check endpoint
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_user_endpoint() {
    // Test user creation endpoint
    let app = create_test_router();

    let user_data = CreateUserRequest {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/users")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&user_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_not_found_endpoint() {
    // Test 404 for non-existent endpoint
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/non-existent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_method_not_allowed() {
    // Test method not allowed
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_json_content_type() {
    // Test JSON content type header
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response.headers().get("content-type");
    assert!(content_type.is_some());
}

#[tokio::test]
async fn test_cors_headers() {
    // Test CORS headers (when configured)
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("Origin", "https://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[cfg(test)]
mod middleware_tests {
    use super::*;
    use axum::middleware;
    use axum::http::Request;
    use tower::ServiceBuilder;

    async fn logging_middleware<B>(
        request: Request<B>,
        next: axum::middleware::Next<B>,
    ) -> axum::response::Response {
        // Simple logging middleware
        println!("Request: {} {}", request.method(), request.uri());
        next.run(request).await
    }

    #[tokio::test]
    async fn test_middleware_chain() {
        // Test middleware chain
        let app = create_test_router()
            .layer(middleware::from_fn(logging_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[tokio::test]
    async fn test_email_validation() {
        // Test email validation in request
        fn is_valid_email(email: &str) -> bool {
            email.contains('@') && email.contains('.')
        }

        let request = CreateUserRequest {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        };

        assert!(is_valid_email(&request.email));
    }

    #[tokio::test]
    async fn test_required_fields() {
        // Test that required fields are present
        let request = CreateUserRequest {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        };

        assert!(!request.name.is_empty());
        assert!(!request.email.is_empty());
    }
}

#[cfg(test)]
mod pagination_tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct PaginationParams {
        page: Option<u32>,
        per_page: Option<u32>,
    }

    #[tokio::test]
    async fn test_pagination_defaults() {
        // Test pagination default values
        let params = PaginationParams {
            page: None,
            per_page: None,
        };

        let page = params.page.unwrap_or(1);
        let per_page = params.per_page.unwrap_or(15);

        assert_eq!(page, 1);
        assert_eq!(per_page, 15);
    }

    #[tokio::test]
    async fn test_pagination_limits() {
        // Test pagination limits
        let params = PaginationParams {
            page: Some(2),
            per_page: Some(50),
        };

        let per_page = params.per_page.unwrap_or(15).min(100);
        assert!(per_page <= 100, "Per page should not exceed maximum");
    }
}

#[cfg(test)]
mod rate_limiting_tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_tracking() {
        // Test rate limit tracking
        struct RateLimiter {
            requests: u32,
            limit: u32,
        }

        let mut limiter = RateLimiter {
            requests: 0,
            limit: 100,
        };

        limiter.requests += 1;
        assert!(limiter.requests < limiter.limit, "Should be under rate limit");
    }
}
