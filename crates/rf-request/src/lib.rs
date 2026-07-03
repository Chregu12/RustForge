//! # RF Request
//!
//! Laravel-style request handling for Rust DX Framework.
//!
//! ## Features
//!
//! - **Custom Request Wrapper**: Enhanced wrapper around Axum's Request
//! - **Field Access**: Easy access to request fields via `get()`
//! - **Validation Integration**: Seamless integration with rf-validation
//! - **User & Session**: Built-in support for authenticated users and sessions
//! - **Extractors**: Axum-compatible extractors
//!
//! ## Quick Start
//!
//! ```ignore
//! use rf_request::Request;
//! use rf_macros::{function, rules};
//!
//! let handler = function!(request: Request) -> Response {
//!     // Access fields
//!     let name: String = request.get("name").unwrap();
//!
//!     // Validate
//!     let validated = request.validate(rules! {
//!         name: required | min(3),
//!         email: required | email,
//!     }).await?;
//!
//!     Response::json(validated)
//! };
//! ```

pub mod error;
pub mod extractors;
pub mod request;
pub mod session;
pub mod upload;
pub mod user;

// Re-export main types
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
