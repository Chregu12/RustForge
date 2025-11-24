//! # rf-route-facade
//!
//! Laravel-style Route facade for the RustForge framework.
//!
//! This crate provides a static, fluent API for defining routes similar to Laravel's routing,
//! making it easy to define routes with middleware, names, and groups.
//!
//! ## Features
//!
//! - **Static Route API**: Define routes using `Route::get()`, `Route::post()`, etc.
//! - **Fluent Builder**: Chain methods like `.middleware()`, `.name()`
//! - **Global Router Registry**: Thread-safe global router accessible from anywhere
//! - **Route Groups**: Organize routes with shared configuration
//! - **Resource Routes**: RESTful resource routing
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_route_facade::Route;
//!
//! // Define routes using the static API
//! Route::get("/users", |_| async { "List users" })
//!     .name("users.index")
//!     .middleware("auth");
//!
//! Route::post("/users", |_| async { "Create user" })
//!     .name("users.store")
//!     .middleware("auth");
//! ```
//!
//! ## Route Groups
//!
//! ```rust,no_run
//! use rf_route_facade::Route;
//!
//! Route::group()
//!     .prefix("/api")
//!     .middleware("auth")
//!     .routes(|group| {
//!         group.get("/users", |_| async { "List users" });
//!         group.post("/users", |_| async { "Create user" });
//!     });
//! ```
//!
//! ## Resource Routes
//!
//! ```rust,no_run
//! use rf_route_facade::Route;
//!
//! // Generates standard RESTful routes
//! Route::resource("posts", "PostController");
//! ```

pub mod builder;
pub mod facade;
pub mod group;
pub mod registry;
pub mod handler;

pub use builder::FacadeRouteBuilder;
pub use facade::Route;
pub use group::{RouteGroupFacade, GroupBuilder};
pub use registry::{global_router, GlobalRouter};
pub use handler::{Handler, HandlerFunc};

// Re-export commonly used types from rf-routing
pub use rf_routing::{HttpMethod, ControllerAction};
