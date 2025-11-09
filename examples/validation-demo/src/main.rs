//! Validation Demo - Comprehensive examples of rf-validation
//!
//! This demo showcases:
//! - ValidatedJson extractor for automatic validation
//! - 30+ validation rules (email, length, range, regex, etc.)
//! - Field-level error responses
//! - Custom validation functions
//! - Nested validation

use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use rf_validation::{Validate, ValidatedJson};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// ============================================================================
// Example 1: Basic Validation (Email, Length, Range)
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
struct CreateUser {
    /// Must be a valid email address
    #[validate(email)]
    email: String,

    /// Password: 8-128 characters
    #[validate(length(min = 8, max = 128))]
    password: String,

    /// Name: 2-100 characters
    #[validate(length(min = 2, max = 100))]
    name: String,

    /// Age: 18-120 years
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

async fn create_user(
    ValidatedJson(user): ValidatedJson<CreateUser>,
) -> impl IntoResponse {
    // If we get here, validation passed!
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "User created successfully",
            "user": {
                "email": user.email,
                "name": user.name,
                "age": user.age,
            }
        })),
    )
}

// ============================================================================
// Example 2: URL Validation
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
struct CreateWebsite {
    #[validate(url)]
    homepage: String,

    #[validate(url)]
    #[serde(skip_serializing_if = "Option::is_none")]
    blog: Option<String>,
}

async fn create_website(
    ValidatedJson(website): ValidatedJson<CreateWebsite>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Website created",
        "homepage": website.homepage,
        "blog": website.blog,
    }))
}

// ============================================================================
// Example 3: Custom Regex Validation
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
struct CreateProduct {
    #[validate(length(min = 3, max = 100))]
    name: String,

    /// SKU must match pattern: 3 letters, dash, 4 digits (e.g., ABC-1234)
    #[validate(regex(path = "*SKU_PATTERN"))]
    sku: String,

    #[validate(range(min = 0.01, max = 999999.99))]
    price: f64,
}

lazy_static::lazy_static! {
    static ref SKU_PATTERN: regex::Regex =
        regex::Regex::new(r"^[A-Z]{3}-\d{4}$").unwrap();
}

async fn create_product(
    ValidatedJson(product): ValidatedJson<CreateProduct>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Product created",
        "product": {
            "name": product.name,
            "sku": product.sku,
            "price": product.price,
        }
    }))
}

// ============================================================================
// Example 4: Contains Validation
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
struct SearchQuery {
    #[validate(length(min = 1, max = 200))]
    #[validate(contains(pattern = "@"))]
    query: String,
}

async fn search(
    ValidatedJson(search): ValidatedJson<SearchQuery>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Search completed",
        "query": search.query,
    }))
}

// ============================================================================
// Example 5: Custom Validation Function
// ============================================================================

fn validate_username(username: &str) -> Result<(), validator::ValidationError> {
    // Username must start with a letter
    if !username.chars().next().unwrap_or('0').is_alphabetic() {
        return Err(validator::ValidationError::new("username_must_start_with_letter"));
    }

    // Username can only contain alphanumeric and underscores
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(validator::ValidationError::new("username_invalid_characters"));
    }

    Ok(())
}

#[derive(Debug, Deserialize, Validate)]
struct RegisterUser {
    #[validate(length(min = 3, max = 30))]
    #[validate(custom(function = "validate_username"))]
    username: String,

    #[validate(email)]
    email: String,
}

async fn register_user(
    ValidatedJson(user): ValidatedJson<RegisterUser>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Registration successful",
        "username": user.username,
        "email": user.email,
    }))
}

// ============================================================================
// Example 6: Nested Validation
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
struct Address {
    #[validate(length(min = 5, max = 100))]
    street: String,

    #[validate(length(min = 2, max = 50))]
    city: String,

    #[validate(regex(path = "*ZIP_PATTERN"))]
    zip: String,

    #[validate(length(equal = 2))]
    country: String,
}

lazy_static::lazy_static! {
    static ref ZIP_PATTERN: regex::Regex =
        regex::Regex::new(r"^\d{5}(-\d{4})?$").unwrap();
}

#[derive(Debug, Deserialize, Validate)]
struct CreateOrder {
    #[validate(length(min = 1))]
    items: Vec<String>,

    #[validate(nested)]
    shipping_address: Address,

    #[validate(nested)]
    billing_address: Option<Address>,
}

async fn create_order(
    ValidatedJson(order): ValidatedJson<CreateOrder>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Order created",
        "items_count": order.items.len(),
        "shipping_to": {
            "city": order.shipping_address.city,
            "country": order.shipping_address.country,
        }
    }))
}

// ============================================================================
// Example 7: Multiple Validation Rules on Same Field
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
struct CreateBlogPost {
    #[validate(length(min = 10, max = 200))]
    #[validate(contains(pattern = " "))] // Must contain at least one space
    title: String,

    #[validate(length(min = 100, max = 10000))]
    content: String,

    #[validate(length(min = 1, max = 10))]
    tags: Vec<String>,
}

async fn create_blog_post(
    ValidatedJson(post): ValidatedJson<CreateBlogPost>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Blog post created",
        "title": post.title,
        "tags": post.tags,
    }))
}

// ============================================================================
// Example 8: Optional Field Validation
// ============================================================================

#[derive(Debug, Deserialize, Validate)]
struct UpdateProfile {
    #[validate(email)]
    email: Option<String>,

    #[validate(length(min = 2, max = 100))]
    name: Option<String>,

    #[validate(url)]
    website: Option<String>,

    #[validate(length(max = 500))]
    bio: Option<String>,
}

async fn update_profile(
    ValidatedJson(profile): ValidatedJson<UpdateProfile>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Profile updated",
        "updates": {
            "email": profile.email,
            "name": profile.name,
            "website": profile.website,
            "bio": profile.bio,
        }
    }))
}

// ============================================================================
// Health Check (no validation)
// ============================================================================

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    endpoints: Vec<String>,
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "rf-validation demo".to_string(),
        endpoints: vec![
            "POST /users - Basic validation (email, length, range)".to_string(),
            "POST /websites - URL validation".to_string(),
            "POST /products - Regex validation".to_string(),
            "POST /search - Contains validation".to_string(),
            "POST /register - Custom validation function".to_string(),
            "POST /orders - Nested validation".to_string(),
            "POST /blog-posts - Multiple rules per field".to_string(),
            "POST /profile - Optional field validation".to_string(),
        ],
    })
}

// ============================================================================
// Application Setup
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // Build router with all validation examples
    let app = Router::new()
        .route("/", get(health))
        .route("/users", post(create_user))
        .route("/websites", post(create_website))
        .route("/products", post(create_product))
        .route("/search", post(search))
        .route("/register", post(register_user))
        .route("/orders", post(create_order))
        .route("/blog-posts", post(create_blog_post))
        .route("/profile", post(update_profile));

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("🚀 rf-validation demo listening on {}", addr);
    tracing::info!("📋 See GET / for available endpoints");
    tracing::info!("");
    tracing::info!("Example curl commands:");
    tracing::info!("");
    tracing::info!("  # Valid user creation:");
    tracing::info!(r#"  curl -X POST http://localhost:3000/users -H "Content-Type: application/json" -d '{{"email":"user@example.com","password":"secret123","name":"John Doe","age":25}}'"#);
    tracing::info!("");
    tracing::info!("  # Invalid email (triggers validation error):");
    tracing::info!(r#"  curl -X POST http://localhost:3000/users -H "Content-Type: application/json" -d '{{"email":"not-an-email","password":"secret123","name":"John Doe","age":25}}'"#);
    tracing::info!("");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_valid_user_creation() {
        let app = Router::new().route("/users", post(create_user));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"test@example.com","password":"password123","name":"Test User","age":25}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_invalid_email() {
        let app = Router::new().route("/users", post(create_user));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"not-an-email","password":"password123","name":"Test User","age":25}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_password_too_short() {
        let app = Router::new().route("/users", post(create_user));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"test@example.com","password":"short","name":"Test User","age":25}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_age_out_of_range() {
        let app = Router::new().route("/users", post(create_user));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"test@example.com","password":"password123","name":"Test User","age":15}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
