//! Application error types

use crate::context::RequestContext;
use crate::error::ProblemDetails;
use thiserror::Error;

/// Main application error type
///
/// Maps to HTTP status codes and RFC 7807 Problem Details responses.
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
///     // Simulate not found
///     Err(AppError::NotFound {
///         resource: format!("User {}", id),
///     })
/// }
/// # #[derive(Debug)]
/// # struct User;
/// ```
#[derive(Debug, Error)]
pub enum AppError {
    /// Validation errors (422 Unprocessable Entity)
    #[error("Validation failed")]
    #[cfg(feature = "validation")]
    Validation(#[from] validator::ValidationErrors),

    /// Structured validation failure (422 Unprocessable Entity)
    ///
    /// Carries a per-field errors map (field name -> list of `{code, message}`)
    /// so the HTTP response mirrors the `ValidatedJson` extractor's 422 body
    /// instead of stringifying the structure into a flat message. This is the
    /// target of the `From<rf_validation::ValidationErrors>` conversion, which
    /// cannot use the external `validator` crate's type above.
    #[error("Validation failed")]
    ValidationFailed {
        /// Per-field errors map (`{ "field": [{ "code", "message" }, ..] }`).
        errors: serde_json::Value,
    },

    /// Resource not found (404)
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Unauthorized access (401)
    #[error("Unauthorized")]
    Unauthorized,

    /// Forbidden access (403)
    #[error("Forbidden: {reason}")]
    Forbidden { reason: String },

    /// Bad request (400)
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    /// Conflict (409)
    #[error("Conflict: {message}")]
    Conflict { message: String },

    /// Too many requests (429)
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Internal server error (500)
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    /// Service unavailable (503)
    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },
}

impl AppError {
    /// Convert error to RFC 7807 Problem Details
    ///
    /// # Arguments
    ///
    /// * `ctx` - Request context for trace ID and path
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::{AppError, RequestContext};
    ///
    /// let ctx = RequestContext::new("/api/users/123", "GET");
    /// let error = AppError::NotFound {
    ///     resource: "User 123".to_string(),
    /// };
    ///
    /// let problem = error.to_problem_details(&ctx);
    /// assert_eq!(problem.status, 404);
    /// assert_eq!(problem.title, "Not Found");
    /// ```
    pub fn to_problem_details(&self, ctx: &RequestContext) -> ProblemDetails {
        match self {
            #[cfg(feature = "validation")]
            Self::Validation(errors) => {
                let mut problem = ProblemDetails::new(
                    422,
                    "Validation Failed",
                    "One or more fields failed validation",
                )
                .with_trace_id(ctx.trace_id())
                .with_instance(ctx.path())
                .with_type_uri("validation-failed");

                // Add validation errors to extensions
                if let Ok(json) = serde_json::to_value(errors) {
                    problem = problem.with_extension("errors", json);
                }

                problem
            }

            Self::ValidationFailed { errors } => {
                ProblemDetails::new(
                    422,
                    "Validation Failed",
                    "One or more fields failed validation",
                )
                .with_trace_id(ctx.trace_id())
                .with_instance(ctx.path())
                .with_type_uri("validation-failed")
                .with_extension("errors", errors.clone())
            }

            Self::NotFound { resource } => ProblemDetails::new(404, "Not Found", resource)
                .with_trace_id(ctx.trace_id())
                .with_instance(ctx.path())
                .with_type_uri("not-found"),

            Self::Unauthorized => ProblemDetails::new(
                401,
                "Unauthorized",
                "Authentication required to access this resource",
            )
            .with_trace_id(ctx.trace_id())
            .with_instance(ctx.path())
            .with_type_uri("unauthorized"),

            Self::Forbidden { reason } => ProblemDetails::new(403, "Forbidden", reason)
                .with_trace_id(ctx.trace_id())
                .with_instance(ctx.path())
                .with_type_uri("forbidden"),

            Self::BadRequest { message } => ProblemDetails::new(400, "Bad Request", message)
                .with_trace_id(ctx.trace_id())
                .with_instance(ctx.path())
                .with_type_uri("bad-request"),

            Self::Conflict { message } => ProblemDetails::new(409, "Conflict", message)
                .with_trace_id(ctx.trace_id())
                .with_instance(ctx.path())
                .with_type_uri("conflict"),

            Self::RateLimitExceeded => ProblemDetails::new(
                429,
                "Too Many Requests",
                "Rate limit exceeded. Please try again later.",
            )
            .with_trace_id(ctx.trace_id())
            .with_instance(ctx.path())
            .with_type_uri("rate-limit-exceeded"),

            Self::ServiceUnavailable { service } => {
                ProblemDetails::new(503, "Service Unavailable", service)
                    .with_trace_id(ctx.trace_id())
                    .with_instance(ctx.path())
                    .with_type_uri("service-unavailable")
            }

            Self::Internal(err) => {
                // Log the full error for debugging
                tracing::error!(error = ?err, "Internal server error");

                let detail = if ctx.is_development() {
                    format!("{:?}", err)
                } else {
                    "An internal error occurred. Please contact support with the trace ID."
                        .to_string()
                };

                let mut problem = ProblemDetails::new(500, "Internal Server Error", detail)
                    .with_trace_id(ctx.trace_id())
                    .with_instance(ctx.path())
                    .with_type_uri("internal-error");

                // Add backtrace in development mode
                if ctx.is_development() {
                    problem = problem.with_extension("backtrace", format!("{:?}", err).into());
                }

                problem
            }
        }
    }

    /// Get the HTTP status code for this error
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::AppError;
    ///
    /// let error = AppError::NotFound {
    ///     resource: "User".to_string(),
    /// };
    /// assert_eq!(error.status_code(), 404);
    /// ```
    pub fn status_code(&self) -> u16 {
        match self {
            #[cfg(feature = "validation")]
            Self::Validation(_) => 422,
            Self::ValidationFailed { .. } => 422,
            Self::NotFound { .. } => 404,
            Self::Unauthorized => 401,
            Self::Forbidden { .. } => 403,
            Self::BadRequest { .. } => 400,
            Self::Conflict { .. } => 409,
            Self::RateLimitExceeded => 429,
            Self::ServiceUnavailable { .. } => 503,
            Self::Internal(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_status_code() {
        let error = AppError::NotFound {
            resource: "User 123".to_string(),
        };
        assert_eq!(error.status_code(), 404);
    }

    #[test]
    fn test_unauthorized_status_code() {
        let error = AppError::Unauthorized;
        assert_eq!(error.status_code(), 401);
    }

    #[test]
    fn test_forbidden_status_code() {
        let error = AppError::Forbidden {
            reason: "Insufficient permissions".to_string(),
        };
        assert_eq!(error.status_code(), 403);
    }

    #[test]
    fn test_bad_request_status_code() {
        let error = AppError::BadRequest {
            message: "Invalid input".to_string(),
        };
        assert_eq!(error.status_code(), 400);
    }

    #[test]
    fn test_internal_error_status_code() {
        let error = AppError::Internal(anyhow::anyhow!("Database error"));
        assert_eq!(error.status_code(), 500);
    }

    #[test]
    fn test_not_found_to_problem_details() {
        let ctx = RequestContext::new("/api/users/123", "GET");
        let error = AppError::NotFound {
            resource: "User 123".to_string(),
        };

        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert!(problem.detail.contains("User 123"));
        assert_eq!(problem.instance, "/api/users/123");
        assert!(!problem.trace_id.is_empty());
    }

    #[test]
    fn test_internal_error_hides_details_in_production() {
        use crate::context::Environment;
        let ctx = RequestContext::with_environment("/api/users", "GET", Environment::Production);

        let error = AppError::Internal(anyhow::anyhow!("Database password: secret123"));
        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 500);
        assert!(!problem.detail.contains("secret123"));
        assert!(problem.detail.contains("contact support"));
        assert!(problem.extensions.is_empty());
    }

    #[test]
    fn test_internal_error_shows_details_in_development() {
        use crate::context::Environment;
        let ctx = RequestContext::with_environment("/api/users", "GET", Environment::Development);

        let error = AppError::Internal(anyhow::anyhow!("Connection timeout"));
        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 500);
        assert!(problem.detail.contains("Connection timeout"));
        assert!(problem.extensions.contains_key("backtrace"));
    }

    #[test]
    fn test_unauthorized_to_problem_details() {
        let ctx = RequestContext::new("/api/users", "GET");
        let error = AppError::Unauthorized;

        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 401);
        assert_eq!(problem.title, "Unauthorized");
        assert!(problem.detail.contains("Authentication required"));
    }

    #[test]
    fn test_rate_limit_exceeded() {
        let ctx = RequestContext::new("/api/users", "POST");
        let error = AppError::RateLimitExceeded;

        let problem = error.to_problem_details(&ctx);

        assert_eq!(problem.status, 429);
        assert_eq!(problem.title, "Too Many Requests");
    }
}
