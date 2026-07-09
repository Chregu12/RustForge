//! # rf-core Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-core,
//! making them easy to import with a single use statement.
//!
//! ## Usage
//!
//! ```rust
//! use rf_core::prelude::*;
//!
//! // Now you have access to:
//! // - AppError, AppResult
//! // - RequestContext, Environment
//! // - ProblemDetails
//! ```

pub use crate::context::{Environment, RequestContext};
pub use crate::error::{AppError, AppResult, OrNotFound, ProblemDetails};
