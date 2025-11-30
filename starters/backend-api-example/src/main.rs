//! RustForge API Example
//!
//! This example demonstrates how to set up authentication endpoints
//! that work with the RustForge starter kits (React, Vue, Angular).

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

// =============================================================================
// Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub email_verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: User,
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
}

// =============================================================================
// App State
// =============================================================================

#[derive(Default)]
pub struct AppState {
    users: RwLock<Vec<User>>,
    next_id: RwLock<u64>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(vec![
                User {
                    id: 1,
                    name: "Demo User".to_string(),
                    email: "demo@example.com".to_string(),
                    email_verified_at: Some("2024-01-01T00:00:00Z".to_string()),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                    updated_at: "2024-01-01T00:00:00Z".to_string(),
                },
            ]),
            next_id: RwLock::new(2),
        }
    }
}

// =============================================================================
// Handlers
// =============================================================================

/// POST /api/auth/login
async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let users = state.users.read().await;

    // Find user by email (in real app, verify password hash)
    if let Some(user) = users.iter().find(|u| u.email == req.email) {
        // Demo: accept any password
        let response = AuthResponse {
            user: user.clone(),
            token: format!("token_{}", user.id),
        };
        (StatusCode::OK, Json(response)).into_response()
    } else {
        let error = ErrorResponse {
            message: "Invalid credentials".to_string(),
            errors: None,
        };
        (StatusCode::UNAUTHORIZED, Json(error)).into_response()
    }
}

/// POST /api/auth/register
async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    if req.password != req.password_confirmation {
        let error = ErrorResponse {
            message: "Passwords do not match".to_string(),
            errors: None,
        };
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error)).into_response();
    }

    let mut users = state.users.write().await;
    let mut next_id = state.next_id.write().await;

    // Check if email exists
    if users.iter().any(|u| u.email == req.email) {
        let error = ErrorResponse {
            message: "Email already exists".to_string(),
            errors: None,
        };
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error)).into_response();
    }

    let user = User {
        id: *next_id,
        name: req.name,
        email: req.email,
        email_verified_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    *next_id += 1;
    users.push(user.clone());

    let response = AuthResponse {
        token: format!("token_{}", user.id),
        user,
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

/// POST /api/auth/logout
async fn logout() -> impl IntoResponse {
    // In real app, invalidate the token
    (StatusCode::NO_CONTENT, ()).into_response()
}

/// POST /api/auth/forgot-password
async fn forgot_password(Json(req): Json<ForgotPasswordRequest>) -> impl IntoResponse {
    // In real app, send reset email
    println!("Password reset requested for: {}", req.email);

    let response = MessageResponse {
        message: "We have emailed your password reset link!".to_string(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// POST /api/auth/reset-password
async fn reset_password(Json(req): Json<ResetPasswordRequest>) -> impl IntoResponse {
    if req.password != req.password_confirmation {
        let error = ErrorResponse {
            message: "Passwords do not match".to_string(),
            errors: None,
        };
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error)).into_response();
    }

    // In real app, verify token and update password
    println!("Password reset for: {} with token: {}", req.email, req.token);

    let response = MessageResponse {
        message: "Your password has been reset!".to_string(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// GET /api/user
async fn get_user(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // In real app, get user from auth token
    let users = state.users.read().await;

    if let Some(user) = users.first() {
        (StatusCode::OK, Json(user.clone())).into_response()
    } else {
        let error = ErrorResponse {
            message: "Unauthenticated".to_string(),
            errors: None,
        };
        (StatusCode::UNAUTHORIZED, Json(error)).into_response()
    }
}

/// PUT /api/user/profile
async fn update_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let mut users = state.users.write().await;

    // In real app, get user from auth token
    if let Some(user) = users.first_mut() {
        user.name = req.name;
        user.email = req.email;
        user.updated_at = chrono::Utc::now().to_rfc3339();

        (StatusCode::OK, Json(user.clone())).into_response()
    } else {
        let error = ErrorResponse {
            message: "Unauthenticated".to_string(),
            errors: None,
        };
        (StatusCode::UNAUTHORIZED, Json(error)).into_response()
    }
}

/// PUT /api/user/password
async fn update_password(Json(req): Json<UpdatePasswordRequest>) -> impl IntoResponse {
    if req.password != req.password_confirmation {
        let error = ErrorResponse {
            message: "Passwords do not match".to_string(),
            errors: None,
        };
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(error)).into_response();
    }

    // In real app, verify current password and update
    let response = MessageResponse {
        message: "Password updated successfully".to_string(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// DELETE /api/user
async fn delete_user() -> impl IntoResponse {
    // In real app, delete user account
    (StatusCode::NO_CONTENT, ()).into_response()
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    // CORS configuration for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Auth routes
        .route("/api/auth/login", post(login))
        .route("/api/auth/register", post(register))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/forgot-password", post(forgot_password))
        .route("/api/auth/reset-password", post(reset_password))
        // User routes
        .route("/api/user", get(get_user))
        .route("/api/user/profile", put(update_profile))
        .route("/api/user/password", put(update_password))
        .route("/api/user", delete(delete_user))
        // Health check
        .route("/health", get(|| async { "OK" }))
        .layer(cors)
        .with_state(state);

    println!("RustForge API running on http://localhost:3000");
    println!();
    println!("Demo credentials:");
    println!("  Email: demo@example.com");
    println!("  Password: any password works for demo");
    println!();
    println!("Available endpoints:");
    println!("  POST   /api/auth/login");
    println!("  POST   /api/auth/register");
    println!("  POST   /api/auth/logout");
    println!("  POST   /api/auth/forgot-password");
    println!("  POST   /api/auth/reset-password");
    println!("  GET    /api/user");
    println!("  PUT    /api/user/profile");
    println!("  PUT    /api/user/password");
    println!("  DELETE /api/user");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
