//! Convenient Result type alias

use crate::error::AppError;

/// Type alias for Result with AppError
///
/// Convenience type for functions that return AppError.
///
/// # Example
///
/// ```rust
/// use rf_core::{AppError, AppResult};
///
/// fn find_user(id: i32) -> AppResult<User> {
///     if id <= 0 {
///         return Err(AppError::BadRequest {
///             message: "ID must be positive".to_string(),
///         });
///     }
///
///     Ok(User { id, name: "John".to_string() })
/// }
///
/// # #[derive(Debug)]
/// # struct User { id: i32, name: String }
/// ```
pub type AppResult<T> = Result<T, AppError>;
