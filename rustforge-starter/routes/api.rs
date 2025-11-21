/// API Routes
///
/// Routes for API endpoints (JSON responses)
/// These routes use token-based authentication

use rf_web::{Router, Route};
use crate::app::Http::Controllers::UserController;

pub fn routes() -> Router {
    Router::new()
        .prefix("/api/v1")
        // .middleware(Middleware::auth_api())

        // API User endpoints
        .group("/users", |router| {
            router
                .route("/", Route::get(UserController::index))
                .route("/{id}", Route::get(UserController::show))
                .route("/", Route::post(UserController::store))
                .route("/{id}", Route::put(UserController::update))
                .route("/{id}", Route::delete(UserController::destroy))
        })
}
