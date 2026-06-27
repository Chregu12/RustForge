//! Unit tests for the `AppError` builder API and its HTTP-status mapping.

use rf_plugins::AppError;

#[test]
fn new_defaults_to_internal_server_error() {
    let e = AppError::new("BOOM", "something broke");
    assert_eq!(e.code, "BOOM");
    assert_eq!(e.message, "something broke");
    assert_eq!(e.status, 500);
    assert!(e.context().is_empty());
}

#[test]
fn named_constructors_carry_the_right_code_and_status() {
    let nf = AppError::not_found("User");
    assert_eq!(nf.code, "NOT_FOUND");
    assert_eq!(nf.status, 404);
    assert_eq!(nf.message, "User not found");

    assert_eq!(AppError::unauthorized().status, 401);
    assert_eq!(AppError::unauthorized().code, "UNAUTHORIZED");
    assert_eq!(AppError::forbidden().status, 403);
    assert_eq!(AppError::forbidden().code, "FORBIDDEN");
    assert_eq!(AppError::internal_server_error("x").status, 500);
}

#[test]
fn validation_attaches_field_context_and_422_status() {
    let e = AppError::validation("email", "must be a valid email");
    assert_eq!(e.code, "VALIDATION");
    assert_eq!(e.status, 422);
    let ctx = e.context();
    assert_eq!(ctx.len(), 1);
    assert_eq!(ctx[0].key, "email");
    assert_eq!(ctx[0].value, "must be a valid email");
}

#[test]
fn with_status_and_with_context_chain_fluently() {
    let e = AppError::new("E", "msg")
        .with_status(418)
        .with_context("teapot", "short and stout")
        .with_context("count", "2");
    assert_eq!(e.status, 418);
    assert_eq!(e.context().len(), 2);
    assert_eq!(e.context()[0].key, "teapot");
    assert_eq!(e.context()[1].value, "2");
}

#[test]
fn status_code_converts_valid_codes() {
    assert_eq!(AppError::not_found("x").status_code().as_u16(), 404);
    assert_eq!(AppError::new("E", "m").with_status(418).status_code().as_u16(), 418);
}

#[test]
fn status_code_falls_back_to_500_for_out_of_range_values() {
    // 42 is not a valid HTTP status code, so the conversion must not panic and
    // should degrade to 500.
    let e = AppError::new("E", "m").with_status(42);
    assert_eq!(e.status_code().as_u16(), 500);
}
