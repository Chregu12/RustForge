//! RustForge Error Handling
//!
//! Comprehensive error handling with user-friendly messages in development
//! and secure error reporting in production.
//!
//! # Features
//!
//! - **User-Friendly Messages**: Helpful, actionable error messages with context
//! - **Error Codes**: Structured error codes (RF001-RF999) for easy searching
//! - **Development Mode**: Full stack traces, syntax highlighting, code snippets
//! - **Production Mode**: Generic error messages, error IDs for correlation
//! - **Error Reporting**: Sentry integration and custom reporter support
//! - **Error Pages**: Custom 404, 500, 403 pages with Blade templates
//!
//! # Example
//!
//! ```rust
//! use rf_errors::{RustForgeError, ErrorContext, error::DatabaseError};
//!
//! // Create a database connection error with context
//! let error = DatabaseError::connection("localhost:5432", "rustforge_dev", "postgres");
//! let rf_error = RustForgeError::Database(error);
//!
//! // In development: shows full details, helpful suggestions
//! // In production: shows generic message with error ID
//! ```

pub mod code;
pub mod context;
pub mod dev_mode;
pub mod error;
pub mod friendly;
pub mod prod_mode;
pub mod reporting;

#[cfg(feature = "error-pages")]
pub mod views;

// Re-exports
pub use code::ErrorCode;
pub use context::{ErrorContext, ErrorLocation};
pub use dev_mode::DevErrorDisplay;
pub use error::{DatabaseError, RustForgeError};
pub use friendly::FriendlyError;
pub use prod_mode::ProdErrorDisplay;
pub use reporting::{ErrorReporter, SentryReporter};

#[cfg(feature = "error-pages")]
pub use views::ErrorPages;

/// Result type alias for RustForge operations
pub type Result<T> = std::result::Result<T, RustForgeError>;
