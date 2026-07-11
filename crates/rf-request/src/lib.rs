//! # rf-request
//!
//! Request handling and implicit (task-local) request globals for RustForge.
//!
//! ## Implicit request globals (`capture_request` pattern)
//!
//! Add the `capture_request` middleware once at the router level; your handlers
//! can then call the global helpers without threading a `Request` argument:
//!
//! ```rust,ignore
//! use rf_request::{input, has, all, file};
//!
//! // Inside a handler wired to a router that has capture_request as a layer:
//! async fn store() -> impl axum::response::IntoResponse {
//!     // Deserialize a JSON/form/query field.  Returns None outside a request scope
//!     // or if the field is absent / cannot coerce to the requested type.
//!     let title: Option<String> = input("title");
//!     let page:  Option<usize>  = input("page");   // coerces "2" -> 2
//!
//!     // Check presence without consuming the value.
//!     if !has("title") { /* ... */ }
//!
//!     // All merged fields as a HashMap<String, serde_json::Value>.
//!     let fields = all();
//!
//!     // An uploaded file from a multipart field named "avatar".
//!     if let Some(upload) = file("avatar") {
//!         let _ = upload.content_type();  // e.g. "image/png"
//!     }
//! }
//! ```
//!
//! These functions return empty / `None` when called outside a
//! `capture_request` scope (e.g. in unit tests without the middleware).
//!
//! ## `capture_request` middleware
//!
//! ```rust,ignore
//! use rf_request::capture_request;
//! use axum::middleware::from_fn;
//!
//! let app = router.layer(from_fn(capture_request));
//! ```

pub mod context;
pub mod error;
pub mod extractors;
pub mod request;
pub mod session;
pub mod upload;
pub mod user;

// Re-export main types
pub use context::{
    all, capture_path_params, capture_request, file, has, input, with_request_context,
    RequestContext,
};
pub use error::{RequestError, RequestResult};
pub use extractors::RequestExtractor;
pub use request::Request;
pub use session::Session;
pub use upload::UploadedFile;
pub use user::User;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        error::{RequestError, RequestResult},
        extractors::RequestExtractor,
        request::Request,
        session::Session,
        user::User,
    };
}
