//! # RustForge Views
//!
//! A Blade-like template system for RustForge built on Tera.
//!
//! ## Features
//!
//! - **Template Inheritance**: Use layouts and sections like Laravel Blade
//! - **Custom Filters**: Route generation, asset URLs, date/money formatting
//! - **Custom Functions**: CSRF tokens, authentication, validation errors, flash messages
//! - **Components**: Reusable UI components with a simple API
//! - **Axum Integration**: First-class support for Axum web framework
//! - **Testing Utilities**: Comprehensive testing helpers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_views::prelude::*;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a view engine
//! let engine = ViewEngine::new("resources/views")?;
//!
//! // Render a template
//! let html = engine.render_with_data("welcome", serde_json::json!({
//!     "name": "World"
//! }))?;
//!
//! println!("{}", html);
//! # Ok(())
//! # }
//! ```
//!
//! ## Using with Axum
//!
//! ```rust,no_run
//! use rf_views::prelude::*;
//! use axum::{Router, routing::get};
//! use std::sync::Arc;
//!
//! async fn index(engine: axum::extract::State<Arc<ViewEngine>>) -> Result<axum::response::Html<String>, axum::http::StatusCode> {
//!     view(&engine, "index", serde_json::json!({
//!         "title": "Home"
//!     }))
//!     .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = Arc::new(ViewEngine::new("resources/views")?);
//!
//! let app = Router::new()
//!     .route("/", get(index))
//!     .with_state(engine);
//! # Ok(())
//! # }
//! ```
//!
//! ## Templates
//!
//! ### Layout (layouts/app.tera)
//!
//! ```html
//! <!DOCTYPE html>
//! <html>
//! <head>
//!     <title>{% block title %}App{% endblock %}</title>
//! </head>
//! <body>
//!     {% block content %}{% endblock %}
//! </body>
//! </html>
//! ```
//!
//! ### View (posts/index.tera)
//!
//! ```html
//! {% extends "layouts/app" %}
//!
//! {% block title %}Posts{% endblock %}
//!
//! {% block content %}
//!     <h1>Posts</h1>
//!     {% for post in posts %}
//!         <article>
//!             <h2>{{ post.title }}</h2>
//!             <p>{{ post.body }}</p>
//!         </article>
//!     {% endfor %}
//! {% endblock %}
//! ```

pub mod components;
pub mod composers;
pub mod config;
pub mod context;
pub mod engine;
pub mod error;
pub mod filters;
pub mod functions;
pub mod helpers;
pub mod response;
pub mod testing;

// Re-exports
pub use components::{register_default_components, ComponentRegistry};
pub use composers::{global as global_composers, ComposerRegistry, ViewComposer};
pub use config::ViewConfig;
pub use context::Context;
pub use engine::ViewEngine;
pub use error::{ViewError, ViewResult};
pub use helpers::{redirect, redirect_with_error, redirect_with_success, view, view_with_context};
pub use response::{HtmlResponse, ViewResponse};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::components::{register_default_components, ComponentRegistry};
    pub use crate::composers::{self, ComposerRegistry, ViewComposer};
    pub use crate::config::ViewConfig;
    pub use crate::context;
    pub use crate::context::Context;
    pub use crate::engine::ViewEngine;
    pub use crate::error::{ViewError, ViewResult};
    pub use crate::helpers::{
        redirect, redirect_with_error, redirect_with_info, redirect_with_success,
        redirect_with_warning, view, view_with_context, ViewBuilder,
    };
    pub use crate::response::{render, render_context, HtmlResponse, ViewResponse};
}
