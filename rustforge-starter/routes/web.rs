/// Web Routes
///
/// Routes for web interface (HTML responses)
/// These routes use session-based authentication

use rf_web::{Router, Route};
use crate::app::Http::Controllers::{HomeController, UserController};

pub fn routes() -> Router {
    Router::new()
        // Welcome page
        .route("/", Route::get(HomeController::index))

        // User resource routes
        .group("/users", |router| {
            router
                .route("/", Route::get(UserController::index))
                .route("/{id}", Route::get(UserController::show))
                .route("/", Route::post(UserController::store))
                .route("/{id}", Route::put(UserController::update))
                .route("/{id}", Route::delete(UserController::destroy))
        })
}
