//! Deployment tests for rf-core

#[cfg(test)]
mod tests {
    use rf_core::{AppError, AppResult, Environment, ProblemDetails, RequestContext};

    // ── Environment ──────────────────────────────────────────────

    #[test]
    fn environment_variants_exist() {
        let _dev = Environment::Development;
        let _staging = Environment::Staging;
        let _prod = Environment::Production;
    }

    #[test]
    fn environment_detect_defaults_to_development() {
        // Without APP_ENV set, should default to Development
        let env = Environment::detect();
        // Just verify it doesn't panic
        let _ = env;
    }

    // ── RequestContext ────────────────────────────────────────────

    #[test]
    fn request_context_new() {
        let ctx = RequestContext::new("/api/users", "GET");
        assert_eq!(ctx.path(), "/api/users");
        assert_eq!(ctx.method(), "GET");
        assert!(!ctx.trace_id().is_empty());
    }

    #[test]
    fn request_context_with_trace_id() {
        let ctx = RequestContext::with_trace_id("/api/users", "POST", "trace-123");
        assert_eq!(ctx.trace_id(), "trace-123");
        assert_eq!(ctx.path(), "/api/users");
        assert_eq!(ctx.method(), "POST");
    }

    #[test]
    fn request_context_unique_trace_ids() {
        let ctx1 = RequestContext::new("/a", "GET");
        let ctx2 = RequestContext::new("/b", "GET");
        assert_ne!(ctx1.trace_id(), ctx2.trace_id());
    }

    #[test]
    fn request_context_environment_checks() {
        let dev = RequestContext::with_environment("/", "GET", Environment::Development);
        assert!(dev.is_development());
        assert!(!dev.is_production());
        assert!(!dev.is_staging());

        let prod = RequestContext::with_environment("/", "GET", Environment::Production);
        assert!(prod.is_production());
        assert!(!prod.is_development());

        let staging = RequestContext::with_environment("/", "GET", Environment::Staging);
        assert!(staging.is_staging());
    }

    // ── AppError ─────────────────────────────────────────────────

    #[test]
    fn app_error_status_codes() {
        assert_eq!(AppError::NotFound { resource: "User".into() }.status_code(), 404);
        assert_eq!(AppError::Unauthorized.status_code(), 401);
        assert_eq!(AppError::Forbidden { reason: "no access".into() }.status_code(), 403);
        assert_eq!(AppError::BadRequest { message: "bad".into() }.status_code(), 400);
        assert_eq!(AppError::Conflict { message: "conflict".into() }.status_code(), 409);
        assert_eq!(AppError::RateLimitExceeded.status_code(), 429);
        assert_eq!(AppError::ServiceUnavailable { service: "db".into() }.status_code(), 503);
    }

    #[test]
    fn app_error_to_problem_details() {
        let ctx = RequestContext::with_environment("/api/test", "GET", Environment::Development);
        let err = AppError::NotFound { resource: "User".into() };
        let pd = err.to_problem_details(&ctx);
        assert_eq!(pd.status, 404);
        assert!(!pd.title.is_empty());
    }

    #[test]
    fn app_error_hides_internal_details_in_production() {
        let ctx = RequestContext::with_environment("/api/test", "GET", Environment::Production);
        let err = AppError::Internal(anyhow::anyhow!("secret database error"));
        let pd = err.to_problem_details(&ctx);
        assert_eq!(pd.status, 500);
        assert!(!pd.detail.contains("secret database error"));
    }

    #[test]
    fn app_error_shows_details_in_development() {
        let ctx = RequestContext::with_environment("/api/test", "GET", Environment::Development);
        let err = AppError::Internal(anyhow::anyhow!("detailed error info"));
        let pd = err.to_problem_details(&ctx);
        assert!(pd.detail.contains("detailed error info"));
    }

    // ── ProblemDetails ───────────────────────────────────────────

    #[test]
    fn problem_details_builder() {
        let pd = ProblemDetails::new(422, "Validation Failed", "Invalid input")
            .with_type_uri("validation-error")
            .with_trace_id("trace-abc")
            .with_instance("/api/users")
            .with_extension("field", serde_json::json!("email"));

        assert_eq!(pd.status, 422);
        assert_eq!(pd.title, "Validation Failed");
        assert_eq!(pd.detail, "Invalid input");
        assert_eq!(pd.trace_id, "trace-abc");
        assert_eq!(pd.instance, "/api/users");
        assert!(pd.extensions.contains_key("field"));
    }

    #[test]
    fn problem_details_serialization_roundtrip() {
        let pd = ProblemDetails::new(400, "Bad Request", "Missing field");
        let json = serde_json::to_string(&pd).expect("serialize");
        let deserialized: ProblemDetails = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.status, 400);
        assert_eq!(deserialized.title, "Bad Request");
    }

    // ── AppResult ────────────────────────────────────────────────

    #[test]
    fn app_result_type_alias_works() {
        let ok: AppResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: AppResult<i32> = Err(AppError::Unauthorized);
        assert!(err.is_err());
    }

    // ── Runtime block_on ─────────────────────────────────────────

    #[test]
    fn runtime_block_on() {
        let result = rf_core::runtime::block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn runtime_block_on_multiple_calls() {
        let a = rf_core::runtime::block_on(async { 1 });
        let b = rf_core::runtime::block_on(async { 2 });
        assert_eq!(a + b, 3);
    }
}
