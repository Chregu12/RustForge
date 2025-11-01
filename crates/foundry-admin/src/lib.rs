//! Foundry Admin Panel - CRUD UI & Dashboard System
//!
//! Provides a Filament/Nova-style admin interface with:
//! - Automatic CRUD generation for models
//! - Dashboard with customizable widgets
//! - Model inspector and data management
//! - User management UI
//! - Settings and activity log
//!
//! # Example
//!
//! ```no_run
//! use foundry_admin::{AdminPanel, AdminConfig};
//!
//! let config = AdminConfig::default()
//!     .with_prefix("/admin")
//!     .with_auth(true);
//!
//! let panel = AdminPanel::new(config);
//! ```

pub mod admin;
pub mod config;
pub mod dashboard;
pub mod middleware;
pub mod resource;
pub mod routes;
pub mod templates;
pub mod widgets;

pub use admin::{AdminPanel, AdminPanelBuilder};
pub use config::AdminConfig;
pub use dashboard::{Dashboard, Widget};
pub use resource::{AdminResource, CrudOperations, ResourceConfig};
pub use widgets::{ChartWidget, MetricWidget, TableWidget, WidgetType};

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// Initialize admin routes
pub fn admin_routes(panel: Arc<AdminPanel>) -> Router {
    Router::new()
        .route("/", get(routes::dashboard))
        .route("/login", get(routes::login).post(routes::do_login))
        .route("/logout", post(routes::logout))
        .route("/resources", get(routes::list_resources))
        .route("/resources/:resource", get(routes::show_resource))
        .route("/resources/:resource/create", get(routes::create_form).post(routes::store))
        .route("/resources/:resource/:id", get(routes::show).post(routes::update))
        .route("/resources/:resource/:id/edit", get(routes::edit_form))
        .route("/resources/:resource/:id/delete", post(routes::delete))
        .route("/users", get(routes::users_index))
        .route("/settings", get(routes::settings))
        .route("/activity", get(routes::activity_log))
        .with_state(panel)
}
