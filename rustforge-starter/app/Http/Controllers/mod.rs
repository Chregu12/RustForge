//! HTTP Controllers
//!
//! Controllers handle incoming HTTP requests and return responses. Keep your
//! controllers thin — delegate complex logic to services. Each method is an
//! Axum handler; add extractors (path, JSON body, state, ...) as you need them.

use axum::{response::IntoResponse, Json};
use serde_json::json;

/// Home Controller — handles the welcome page.
pub struct HomeController;

impl HomeController {
    pub async fn index() -> impl IntoResponse {
        Json(json!({
            "title": "Welcome to RustForge",
            "message": "The Laravel experience for Rust"
        }))
    }
}

/// User Controller — example resource controller.
pub struct UserController;

impl UserController {
    /// Display a listing of users.
    pub async fn index() -> impl IntoResponse {
        // Example: let users = User::all().await?;
        Json(json!({ "users": [] }))
    }

    /// Display the specified user.
    pub async fn show() -> impl IntoResponse {
        // Example: let user = User::find(id).await?;
        Json(json!({ "user": null }))
    }

    /// Store a newly created user.
    pub async fn store() -> impl IntoResponse {
        // Example: let user = User::create(req.validated()).await?;
        Json(json!({ "message": "User created successfully" }))
    }

    /// Update the specified user.
    pub async fn update() -> impl IntoResponse {
        // Example: user.update(req.validated()).await?;
        Json(json!({ "message": "User updated successfully" }))
    }

    /// Remove the specified user.
    pub async fn destroy() -> impl IntoResponse {
        // Example: User::destroy(id).await?;
        Json(json!({ "message": "User deleted successfully" }))
    }
}
