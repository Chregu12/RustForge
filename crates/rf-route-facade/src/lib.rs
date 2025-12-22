//! # rf-route-facade
//!
//! Laravel-style Route facade for the RustForge framework.
//!
//! This crate provides a static, fluent API for defining routes similar to Laravel's routing,
//! making it easy to define routes with middleware, names, and groups.
//!
//! # Recommended Usage
//!
//! Use the consolidated `rf` crate for simpler imports:
//! ```rust
//! use rf::Route;  // or use rf::prelude::*;
//! ```
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
//! // Recommended: use rf::Route;
//! use rf_route_facade::Route;  // Direct import also works
//!
//! // Define routes using the static API
//! Route::get("/users", "UserController@index")
//!     .name("users.index")
//!     .middleware("auth");
//!
//! Route::post("/users", "UserController@store")
//!     .name("users.store")
//!     .middleware("auth");
//! ```
//!
//! ## Route Groups
//!
//! ```rust,no_run
//! // Recommended: use rf::Route;
//! use rf_route_facade::Route;  // Direct import also works
//!
//! Route::group()
//!     .prefix("/api")
//!     .middleware("auth")
//!     .routes(|group| {
//!         group.get("/users", "UserController@index");
//!         group.post("/users", "UserController@store");
//!     });
//! ```
//!
//! ## Resource Routes
//!
//! ```rust,no_run
//! // Recommended: use rf::Route;
//! use rf_route_facade::Route;  // Direct import also works
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
pub use facade::{Route, MiddlewareGroupBuilder};
pub use group::{RouteGroupFacade, GroupBuilder};
pub use registry::{global_router, GlobalRouter};
pub use handler::{Handler, HandlerFunc};

// Re-export commonly used types from rf-routing
pub use rf_routing::{HttpMethod, ControllerAction};
