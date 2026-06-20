//! Web Routes
//!
//! Routes for the browser-facing interface (HTML/JSON responses).

use crate::app::Http::Controllers::{HomeController, UserController};
use axum::{routing::get, Router};

pub fn routes() -> Router {
    Router::new()
        // Welcome page
        .route("/", get(HomeController::index))
        // User resource routes
        .route(
            "/users",
            get(UserController::index).post(UserController::store),
        )
        .route(
            "/users/{id}",
            get(UserController::show)
                .put(UserController::update)
                .delete(UserController::destroy),
        )
}
