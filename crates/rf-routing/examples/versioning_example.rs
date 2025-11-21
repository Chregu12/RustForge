//! API Versioning Example
//!
//! Demonstrates multiple ways to version your API:
//! - URL-based versioning (/v1/users, /v2/users)
//! - Header-based versioning (Accept: application/vnd.api.v1+json)
//! - Custom header versioning (API-Version: 1)

use axum::{
    extract::Path,
    response::Json,
    routing::get,
    Router,
};
use rf_routing::{
    versioning::{ApiVersion, extract_from_accept, extract_from_header, extract_from_path},
    versioned_router::VersionedRouterBuilder,
};
use serde::Serialize;

// V1 User representation
#[derive(Serialize)]
struct UserV1 {
    id: i64,
    name: String,
}

// V2 User representation (added email)
#[derive(Serialize)]
struct UserV2 {
    id: i64,
    name: String,
    email: String,
}

// V3 User representation (added timestamps)
#[derive(Serialize)]
struct UserV3 {
    id: i64,
    name: String,
    email: String,
    created_at: String,
    updated_at: String,
}

// Version 1 handlers
async fn get_users_v1() -> Json<Vec<UserV1>> {
    Json(vec![
        UserV1 {
            id: 1,
            name: "John Doe".to_string(),
        },
        UserV1 {
            id: 2,
            name: "Jane Smith".to_string(),
        },
    ])
}

async fn get_user_v1(Path(id): Path<i64>) -> Json<UserV1> {
    Json(UserV1 {
        id,
        name: format!("User {}", id),
    })
}

// Version 2 handlers
async fn get_users_v2() -> Json<Vec<UserV2>> {
    Json(vec![
        UserV2 {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        },
        UserV2 {
            id: 2,
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
        },
    ])
}

async fn get_user_v2(Path(id): Path<i64>) -> Json<UserV2> {
    Json(UserV2 {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
    })
}

// Version 3 handlers
async fn get_users_v3() -> Json<Vec<UserV3>> {
    Json(vec![
        UserV3 {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-15T10:30:00Z".to_string(),
        },
        UserV3 {
            id: 2,
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            created_at: "2024-01-02T00:00:00Z".to_string(),
            updated_at: "2024-01-16T14:20:00Z".to_string(),
        },
    ])
}

async fn get_user_v3(Path(id): Path<i64>) -> Json<UserV3> {
    Json(UserV3 {
        id,
        name: format!("User {}", id),
        email: format!("user{}@example.com", id),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-15T10:30:00Z".to_string(),
    })
}

// Version info endpoint
async fn version_info(version: ApiVersion) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": version.version(),
        "supported_versions": [1, 2, 3],
        "deprecated_versions": [],
    }))
}

#[tokio::main]
async fn main() {
    // Build versioned API
    let app = VersionedRouterBuilder::new()
        // Version 1 routes
        .version(1, |router| {
            router
                .route("/users", get(get_users_v1))
                .route("/users/:id", get(get_user_v1))
        })
        // Version 2 routes
        .version(2, |router| {
            router
                .route("/users", get(get_users_v2))
                .route("/users/:id", get(get_user_v2))
        })
        // Version 3 routes (latest)
        .version(3, |router| {
            router
                .route("/users", get(get_users_v3))
                .route("/users/:id", get(get_user_v3))
        })
        .default_version(3)
        .supported_versions(vec![1, 2, 3])
        .build_with_prefix(); // Use URL-based versioning

    // Add version info endpoint
    let app = Router::new()
        .nest("/api", app)
        .route("/api/version", get(version_info));

    println!("API Versioning Example");
    println!("======================\n");
    println!("Server running on http://localhost:3001\n");

    println!("URL-based versioning:");
    println!("  GET http://localhost:3001/api/v1/users");
    println!("  GET http://localhost:3001/api/v2/users");
    println!("  GET http://localhost:3001/api/v3/users\n");

    println!("Header-based versioning (Accept header):");
    println!("  curl -H 'Accept: application/vnd.api.v1+json' http://localhost:3001/api/users\n");

    println!("Custom header versioning:");
    println!("  curl -H 'API-Version: 2' http://localhost:3001/api/users\n");

    println!("Responses differ by version:");
    println!("  V1: {{id, name}}");
    println!("  V2: {{id, name, email}}");
    println!("  V3: {{id, name, email, created_at, updated_at}}\n");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
