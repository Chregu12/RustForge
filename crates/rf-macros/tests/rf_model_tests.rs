//! Integration tests for the `#[derive(RfModel)]` macro.
//!
//! These tests verify that the generated methods return the correct values for
//! a variety of struct configurations.

use rf_macros::RfModel;

// ---------------------------------------------------------------------------
// Test model: full configuration
// ---------------------------------------------------------------------------

#[derive(RfModel)]
#[rf(table = "users")]
#[rf(hidden = ["password", "remember_token"])]
#[rf(fillable = ["name", "email", "password"])]
#[rf(guarded = ["id"])]
#[rf(timestamps)]
struct User {
    id: i64,
    name: String,
    email: String,
    password: String,
}

// ---------------------------------------------------------------------------
// Test model: minimal (no attributes)
// ---------------------------------------------------------------------------

#[derive(RfModel)]
struct Product {
    id: i64,
    name: String,
}

// ---------------------------------------------------------------------------
// Test model: soft-delete enabled
// ---------------------------------------------------------------------------

#[derive(RfModel)]
#[rf(table = "posts")]
#[rf(fillable = ["title", "body"])]
#[rf(soft_delete)]
struct Post {
    id: i64,
    title: String,
    body: String,
}

// ---------------------------------------------------------------------------
// Test model: guarded only (no fillable)
// ---------------------------------------------------------------------------

#[derive(RfModel)]
#[rf(table = "settings")]
#[rf(guarded = ["id", "key"])]
struct Setting {
    id: i64,
    key: String,
    value: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_table_name_explicit() {
    assert_eq!(User::table_name(), "users");
}

#[test]
fn test_table_name_default_snake_case_plural() {
    // `Product` → `products`
    assert_eq!(Product::table_name(), "products");
}

#[test]
fn test_table_name_explicit_posts() {
    assert_eq!(Post::table_name(), "posts");
}

#[test]
fn test_hidden_fields_contains_password() {
    assert!(
        User::hidden_fields().contains(&"password"),
        "hidden_fields should contain 'password'"
    );
}

#[test]
fn test_hidden_fields_contains_remember_token() {
    assert!(
        User::hidden_fields().contains(&"remember_token"),
        "hidden_fields should contain 'remember_token'"
    );
}

#[test]
fn test_hidden_fields_length() {
    assert_eq!(User::hidden_fields().len(), 2);
}

#[test]
fn test_fillable_fields_user() {
    let fillable = User::fillable_fields();
    assert!(fillable.contains(&"name"));
    assert!(fillable.contains(&"email"));
    assert!(fillable.contains(&"password"));
    assert_eq!(fillable.len(), 3);
}

#[test]
fn test_fillable_fields_post() {
    let fillable = Post::fillable_fields();
    assert!(fillable.contains(&"title"));
    assert!(fillable.contains(&"body"));
}

#[test]
fn test_fillable_fields_empty_when_not_set() {
    assert_eq!(Product::fillable_fields().len(), 0);
}

#[test]
fn test_guarded_fields_user() {
    let guarded = User::guarded_fields();
    assert!(guarded.contains(&"id"));
    assert_eq!(guarded.len(), 1);
}

#[test]
fn test_guarded_fields_setting() {
    let guarded = Setting::guarded_fields();
    assert!(guarded.contains(&"id"));
    assert!(guarded.contains(&"key"));
    assert_eq!(guarded.len(), 2);
}

#[test]
fn test_uses_timestamps_true() {
    assert!(User::uses_timestamps(), "User should use timestamps");
}

#[test]
fn test_uses_timestamps_false_by_default() {
    assert!(!Product::uses_timestamps(), "Product should not use timestamps");
}

#[test]
fn test_uses_soft_delete_false_by_default() {
    assert!(!User::uses_soft_delete(), "User should not use soft delete");
}

#[test]
fn test_uses_soft_delete_true_when_set() {
    assert!(Post::uses_soft_delete(), "Post should use soft delete");
}

#[test]
fn test_table_name_is_static_str() {
    // Ensure the return type is &'static str (compile-time check via assignment)
    let _: &'static str = User::table_name();
    let _: &'static str = Product::table_name();
}

#[test]
fn test_methods_callable_multiple_times() {
    // Static slice references must be consistent across calls.
    assert_eq!(User::hidden_fields(), User::hidden_fields());
    assert_eq!(User::fillable_fields(), User::fillable_fields());
    assert_eq!(User::guarded_fields(), User::guarded_fields());
    assert_eq!(User::table_name(), User::table_name());
}
