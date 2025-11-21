//! # rf-view: Laravel Blade-like Template System for Rust
//!
//! A Tera-based template engine with Laravel Blade-inspired API for RustForge.
//!
//! ## Features
//!
//! - **Blade-like Syntax**: Familiar template syntax for Laravel developers
//! - **Layouts & Extends**: Template inheritance with layouts
//! - **Components**: Reusable template components
//! - **Custom Filters**: Register custom Tera filters
//! - **Axum Integration**: First-class support for Axum responses
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_view::{View, ViewEngine};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize the view engine
//! ViewEngine::init("templates/**/*")?;
//!
//! // Render a view
//! let html = View::make("welcome", json!({
//!     "title": "Welcome",
//!     "user": "John Doe"
//! })).render().await?;
//!
//! // With layout
//! let html = View::make("pages.home", json!({"title": "Home"}))
//!     .layout("layouts.app")
//!     .render()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Template Example
//!
//! ```html
//! {{!-- layouts/app.tera --}}
//! <!DOCTYPE html>
//! <html>
//! <head>
//!     <title>{% block title %}{{ title }}{% endblock %}</title>
//! </head>
//! <body>
//!     {% block content %}{% endblock %}
//! </body>
//! </html>
//!
//! {{!-- pages/home.tera --}}
//! {% extends "layouts/app.tera" %}
//!
//! {% block title %}Home - {{ super() }}{% endblock %}
//!
//! {% block content %}
//! <h1>Welcome, {{ user }}!</h1>
//! {% endblock %}
//! ```

pub mod engine;
pub mod error;
pub mod response;
pub mod view;

pub use engine::ViewEngine;
pub use error::{ViewError, ViewResult};
pub use response::ViewResponse;
pub use view::View;

// Re-export commonly used types
pub use tera::{Context, Tera, Value};
