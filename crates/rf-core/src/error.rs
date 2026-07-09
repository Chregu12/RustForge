//! RFC 7807 Problem Details error handling
//!
//! Provides type-safe error types that map to HTTP status codes and
//! RFC 7807 Problem Details JSON responses.

mod app_error;
mod not_found;
mod problem_details;
mod result;

pub use app_error::AppError;
pub use not_found::OrNotFound;
pub use problem_details::ProblemDetails;
pub use result::AppResult;
