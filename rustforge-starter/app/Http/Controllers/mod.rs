/// HTTP Controllers
///
/// Controllers handle incoming HTTP requests and return responses.
/// Keep your controllers thin - delegate complex logic to services.

use rf_web::{Request, Response, Result};
use serde_json::json;

/// Home Controller - Handles the welcome page
pub struct HomeController;

impl HomeController {
    pub async fn index(_req: Request) -> Result<Response> {
        Ok(Response::view("welcome", json!({
            "title": "Welcome to RustForge",
            "message": "The Laravel experience for Rust"
        })))
    }
}

/// User Controller - Example resource controller
pub struct UserController;

impl UserController {
    /// Display a listing of users
    pub async fn index(_req: Request) -> Result<Response> {
        // Example: let users = User::all().await?;
        Ok(Response::json(json!({
            "users": []
        })))
    }

    /// Display the specified user
    pub async fn show(_req: Request) -> Result<Response> {
        // Example: let id = req.param("id")?;
        // Example: let user = User::find(id).await?;
        Ok(Response::json(json!({
            "user": null
        })))
    }

    /// Store a newly created user
    pub async fn store(_req: Request) -> Result<Response> {
        // Example: let user = User::create(req.validated()).await?;
        Ok(Response::json(json!({
            "message": "User created successfully"
        })))
    }

    /// Update the specified user
    pub async fn update(_req: Request) -> Result<Response> {
        // Example: let id = req.param("id")?;
        // Example: let user = User::find(id).await?;
        // Example: user.update(req.validated()).await?;
        Ok(Response::json(json!({
            "message": "User updated successfully"
        })))
    }

    /// Remove the specified user
    pub async fn destroy(_req: Request) -> Result<Response> {
        // Example: let id = req.param("id")?;
        // Example: User::destroy(id).await?;
        Ok(Response::json(json!({
            "message": "User deleted successfully"
        })))
    }
}
