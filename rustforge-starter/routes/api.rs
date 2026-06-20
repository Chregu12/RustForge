//! API Routes
//!
//! Routes for JSON API endpoints, mounted under `/api/v1`.

use crate::app::Http::Controllers::UserController;
use axum::{routing::get, Router};

pub fn routes() -> Router {
    let users = Router::new()
        .route(
            "/users",
            get(UserController::index).post(UserController::store),
        )
        .route(
            "/users/{id}",
            get(UserController::show)
                .put(UserController::update)
                .delete(UserController::destroy),
        );

    Router::new().nest("/api/v1", users)
}
