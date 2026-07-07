//! RustForge dogfooding example: end-to-end field-level validation with the
//! `Model!` `@` DSL and the auto-validating `ValidatedJson` extractor.
//!
//! This is the shipped counterpart to `sandbox/probes/validated_json_dto`: it
//! shows the pattern a reader is meant to copy. A SINGLE `Model!` declaration
//! opts into a real `rf_validation::Validate` impl (via `validated`) and layers
//! explicit per-field `@` rules on top of the inferred ones:
//!
//!   * `@ min(N)` / `@ max(N)`  — string length bounds,
//!   * `@ alphanumeric`         — letters + digits only,
//!   * `@ email`                — a syntactically valid email address,
//!   * `@ message("...")`       — a CUSTOM per-field error message override,
//!   * `@ regex("...")`         — value must match the given pattern.
//!
//! The generated `CreateUser` DTO therefore implements the real `Validate`
//! trait, so the handler can take `ValidatedJson<CreateUser>` and the body is
//! deserialized AND validated INSIDE the extractor — there is NO manual
//! `validate!()` call anywhere in the handler. Valid input reaches the handler
//! and returns `201 Created`; invalid input is rejected by the extractor with
//! the framework's `422 Unprocessable Entity` whose body carries the per-field
//! errors (including the custom `@ message` text).
//!
//! Run it:  `cargo run -p validated-signup`  (serves on http://127.0.0.1:3002)
//!   POST /signup {"username":"ada42","email":"ada@example.com",
//!                 "password":"hunter2!","zipcode":"12345"}
//!
//! NOTE: this example uses axum 0.8 directly (like the probe) rather than rf's
//! `post`/`build_router`, because `ValidatedJson` is a `FromRequest` built
//! against axum 0.8 while rf-routing pins axum 0.7; the two Handler traits are
//! not interchangeable. The validation itself is 100% the real framework path.
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use rf::Model; // Model! proc-macro (entity + DTOs + opt-in Validate impl)
use rf_db_facade::Model as ModelTrait; // trait so the generated struct type-checks
use rf_validation::ValidatedJson; // the REAL auto-validating extractor

// ONE declaration. `validated` opts the DTOs into a real `Validate` impl; the
// `@` DSL adds length + alphanumeric + email + regex checks, and the custom
// `@ message("...")` overrides the email field's default English message.
Model!(User {
    validated,
    username: String @ min(3) max(20) alphanumeric,
    email: String @ email message("Please enter a valid email address"),
    password: String @ min(8),
    zipcode: String @ regex("^\\d{5}$"),
});

/// POST /signup — the handler takes the auto-validating extractor. If execution
/// reaches this body, `CreateUser` already deserialized AND validated; a real
/// app would persist the user here. Returns `201 Created` with the echoed
/// public fields. An invalid body never reaches this code — the extractor
/// short-circuits with `422` before the handler runs.
async fn signup(ValidatedJson(user): ValidatedJson<CreateUser>) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "created": true,
            "username": user.username,
            "email": user.email,
        })),
    )
}

/// Wire the one POST route and return the served router (axum 0.8, matching the
/// `ValidatedJson` extractor).
fn build_app() -> Router {
    Router::new().route("/signup", post(signup))
}

#[tokio::main]
async fn main() {
    // Entity + Model trait intact (the `@` rules are additive).
    assert_eq!(<User as ModelTrait>::TABLE, "users");
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3002")
        .await
        .expect("bind");
    println!("validated-signup listening on http://127.0.0.1:3002");
    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`

    /// Send one JSON POST /signup through the real router; return the (status,
    /// parsed-JSON-body) pair.
    async fn post_signup(body: &str) -> (StatusCode, serde_json::Value) {
        let resp = build_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/signup")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    fn valid_body() -> String {
        r#"{"username":"ada42","email":"ada@example.com","password":"hunter2!","zipcode":"12345"}"#
            .to_string()
    }

    /// Drive the FULL validation path through the REAL extractor + validator:
    /// a valid signup passes (201); each kind of invalid input is rejected by
    /// the extractor with 422, and the response body carries the offending
    /// field error — including the CUSTOM `@ message` on the email field.
    #[tokio::test]
    async fn validated_signup_end_to_end() {
        // The generated CreateUser really implements rf_validation::Validate;
        // if it didn't, ValidatedJson<CreateUser> would not even compile.
        fn assert_is_validate<T: rf_validation::Validate>() {}
        assert_is_validate::<CreateUser>();

        // ---- 1. Fully valid body -> 201 Created, driven by the real extractor.
        let (status, body) = post_signup(&valid_body()).await;
        assert_eq!(status, StatusCode::CREATED, "valid signup must return 201");
        assert_eq!(body["created"], true);
        assert_eq!(body["username"], "ada42");

        // ---- 2. Invalid email (@ email) -> 422, and the body carries the
        //         CUSTOM @ message override for the email field.
        let (status, body) =
            post_signup(&valid_body().replace("ada@example.com", "not-an-email")).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid email must be rejected with 422 by the extractor"
        );
        assert_eq!(
            body["errors"]["email"][0]["message"], "Please enter a valid email address",
            "422 body must carry the custom @ message on the email field"
        );

        // ---- 3. Too-short password (@ min(8)) -> 422, body names `password`.
        let (status, body) =
            post_signup(&valid_body().replace("hunter2!", "short")).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "too-short password must fail the @ min(8) rule (422)"
        );
        assert!(
            body["errors"].get("password").is_some(),
            "422 body must carry the password field error"
        );

        // ---- 4. Non-matching zipcode (@ regex) -> 422, body names `zipcode`.
        let (status, body) =
            post_signup(&valid_body().replace("12345", "abcde")).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "non-matching zipcode must fail the @ regex rule (422)"
        );
        assert!(
            body["errors"].get("zipcode").is_some(),
            "422 body must carry the zipcode field error"
        );

        // ---- 5. Non-alphanumeric username (@ alphanumeric) -> 422.
        let (status, body) =
            post_signup(&valid_body().replace("ada42", "ada_42")).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "non-alphanumeric username must fail the @ alphanumeric rule (422)"
        );
        assert!(
            body["errors"].get("username").is_some(),
            "422 body must carry the username field error"
        );
    }
}
