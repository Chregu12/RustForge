//! RFC 7807 Problem Details implementation

use serde::{Deserialize, Serialize};

/// RFC 7807 Problem Details for HTTP APIs
///
/// Standard format for HTTP error responses.
///
/// # Example
///
/// ```rust
/// use rf_core::ProblemDetails;
///
/// let problem = ProblemDetails::new(404, "Not Found", "User not found")
///     .with_trace_id("abc-123")
///     .with_instance("/api/users/123")
///     .with_type_uri("not-found");
///
/// let json = serde_json::to_string_pretty(&problem).unwrap();
/// println!("{}", json);
/// ```
///
/// JSON output:
/// ```json
/// {
///   "type": "https://api.example.com/errors/not-found",
///   "title": "Not Found",
///   "status": 404,
///   "detail": "User not found",
///   "instance": "/api/users/123",
///   "trace_id": "abc-123"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type
    #[serde(rename = "type")]
    pub type_uri: String,

    /// A short, human-readable summary of the problem type
    pub title: String,

    /// The HTTP status code
    pub status: u16,

    /// A human-readable explanation specific to this occurrence of the problem
    pub detail: String,

    /// A URI reference identifying the specific occurrence of the problem
    pub instance: String,

    /// Trace ID for log correlation
    pub trace_id: String,

    /// Additional problem-specific fields
    #[serde(flatten)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl ProblemDetails {
    /// Create a new Problem Details
    ///
    /// # Arguments
    ///
    /// * `status` - HTTP status code
    /// * `title` - Short summary of the error
    /// * `detail` - Detailed explanation
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::ProblemDetails;
    ///
    /// let problem = ProblemDetails::new(400, "Bad Request", "Invalid email format");
    /// assert_eq!(problem.status, 400);
    /// ```
    pub fn new(status: u16, title: impl Into<String>, detail: impl Into<String>) -> Self {
        let status_str = status.to_string();
        Self {
            type_uri: format!("https://api.example.com/errors/{}", status_str),
            title: title.into(),
            status,
            detail: detail.into(),
            instance: "/".to_string(),
            trace_id: String::new(),
            extensions: serde_json::Map::new(),
        }
    }

    /// Set the type URI
    ///
    /// The type will be appended to the base URL.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::ProblemDetails;
    ///
    /// let problem = ProblemDetails::new(404, "Not Found", "Resource not found")
    ///     .with_type_uri("user-not-found");
    ///
    /// assert!(problem.type_uri.ends_with("user-not-found"));
    /// ```
    pub fn with_type_uri(mut self, type_suffix: impl Into<String>) -> Self {
        self.type_uri = format!("https://api.example.com/errors/{}", type_suffix.into());
        self
    }

    /// Set the trace ID
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::ProblemDetails;
    ///
    /// let problem = ProblemDetails::new(500, "Internal Error", "Database connection failed")
    ///     .with_trace_id("abc-123-def-456");
    ///
    /// assert_eq!(problem.trace_id, "abc-123-def-456");
    /// ```
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = trace_id.into();
        self
    }

    /// Set the instance URI
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::ProblemDetails;
    ///
    /// let problem = ProblemDetails::new(404, "Not Found", "User not found")
    ///     .with_instance("/api/users/123");
    ///
    /// assert_eq!(problem.instance, "/api/users/123");
    /// ```
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = instance.into();
        self
    }

    /// Add an extension field
    ///
    /// Extension fields allow adding problem-specific data to the response.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::ProblemDetails;
    /// use serde_json::json;
    ///
    /// let problem = ProblemDetails::new(422, "Validation Failed", "Invalid input")
    ///     .with_extension("errors", json!({
    ///         "email": ["must be a valid email address"],
    ///         "age": ["must be at least 18"]
    ///     }));
    ///
    /// assert!(problem.extensions.contains_key("errors"));
    /// ```
    pub fn with_extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_problem_details_new() {
        let problem = ProblemDetails::new(404, "Not Found", "Resource not found");

        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert_eq!(problem.detail, "Resource not found");
    }

    #[test]
    fn test_with_trace_id() {
        let problem =
            ProblemDetails::new(500, "Error", "Something went wrong").with_trace_id("trace-123");

        assert_eq!(problem.trace_id, "trace-123");
    }

    #[test]
    fn test_with_instance() {
        let problem =
            ProblemDetails::new(404, "Not Found", "User not found").with_instance("/api/users/123");

        assert_eq!(problem.instance, "/api/users/123");
    }

    #[test]
    fn test_with_type_uri() {
        let problem =
            ProblemDetails::new(404, "Not Found", "User not found").with_type_uri("user-not-found");

        assert!(problem.type_uri.contains("user-not-found"));
    }

    #[test]
    fn test_with_extension() {
        let problem = ProblemDetails::new(422, "Validation Failed", "Invalid input")
            .with_extension("field", serde_json::json!("email"));

        assert_eq!(
            problem.extensions.get("field").unwrap(),
            &serde_json::json!("email")
        );
    }

    #[test]
    fn test_serialization() {
        let problem = ProblemDetails::new(404, "Not Found", "User not found")
            .with_trace_id("abc-123")
            .with_instance("/api/users/123")
            .with_type_uri("not-found")
            .with_extension("user_id", serde_json::json!(123));

        let json = serde_json::to_value(&problem).unwrap();

        assert_eq!(json["status"], 404);
        assert_eq!(json["title"], "Not Found");
        assert_eq!(json["detail"], "User not found");
        assert_eq!(json["trace_id"], "abc-123");
        assert_eq!(json["instance"], "/api/users/123");
        assert!(json["type"].as_str().unwrap().contains("not-found"));
        assert_eq!(json["user_id"], 123);
    }

    #[test]
    fn test_deserialization() {
        let json = serde_json::json!({
            "type": "https://api.example.com/errors/not-found",
            "title": "Not Found",
            "status": 404,
            "detail": "User not found",
            "instance": "/api/users/123",
            "trace_id": "abc-123",
            "user_id": 123
        });

        let problem: ProblemDetails = serde_json::from_value(json).unwrap();

        assert_eq!(problem.status, 404);
        assert_eq!(problem.title, "Not Found");
        assert_eq!(problem.trace_id, "abc-123");
        assert_eq!(problem.extensions.get("user_id").unwrap(), &123);
    }
}
