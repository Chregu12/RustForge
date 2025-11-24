use rf_request::{Request, User, Session};
use serde_json::json;
use std::collections::HashMap;
use axum::body::Body;
use http::Request as HttpRequest;

fn create_test_request_with_fields() -> Request {
    let http_req = HttpRequest::builder()
        .method("POST")
        .uri("/test")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), json!("John Doe"));
    fields.insert("email".to_string(), json!("john@example.com"));
    fields.insert("age".to_string(), json!(30));
    fields.insert("active".to_string(), json!(true));

    Request::new(http_req).with_fields(fields)
}

#[test]
fn test_request_get() {
    let request = create_test_request_with_fields();

    let name: String = request.get("name").unwrap();
    assert_eq!(name, "John Doe");

    let age: u32 = request.get("age").unwrap();
    assert_eq!(age, 30);

    let active: bool = request.get("active").unwrap();
    assert!(active);
}

#[test]
fn test_request_require() {
    let request = create_test_request_with_fields();

    let name: String = request.require("name").unwrap();
    assert_eq!(name, "John Doe");

    let result: Result<String, _> = request.require("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_request_get_or() {
    let request = create_test_request_with_fields();

    let name: String = request.get_or("name", "Default".to_string());
    assert_eq!(name, "John Doe");

    let phone: String = request.get_or("phone", "N/A".to_string());
    assert_eq!(phone, "N/A");
}

#[test]
fn test_request_has() {
    let request = create_test_request_with_fields();

    assert!(request.has("name"));
    assert!(request.has("email"));
    assert!(!request.has("phone"));
}

#[test]
fn test_request_only() {
    let request = create_test_request_with_fields();
    let result = request.only(&["name", "email"]);

    assert_eq!(result.len(), 2);
    assert!(result.contains_key("name"));
    assert!(result.contains_key("email"));
    assert!(!result.contains_key("age"));
}

#[test]
fn test_request_except() {
    let request = create_test_request_with_fields();
    let result = request.except(&["age"]);

    assert_eq!(result.len(), 3);
    assert!(result.contains_key("name"));
    assert!(result.contains_key("email"));
    assert!(!result.contains_key("age"));
}

#[test]
fn test_request_has_any() {
    let request = create_test_request_with_fields();

    assert!(request.has_any(&["name", "phone"]));
    assert!(!request.has_any(&["phone", "address"]));
}

#[test]
fn test_request_has_all() {
    let request = create_test_request_with_fields();

    assert!(request.has_all(&["name", "email"]));
    assert!(!request.has_all(&["name", "phone"]));
}

#[test]
fn test_request_with_user() {
    let request = create_test_request_with_fields();
    let user = User::new(1, "test@example.com".to_string());

    let request = request.with_user(user);

    assert!(request.user().is_some());
    assert_eq!(request.user().unwrap().id, 1);
    assert_eq!(request.user().unwrap().email, "test@example.com");
}

#[test]
fn test_request_require_user() {
    let request = create_test_request_with_fields();
    assert!(request.require_user().is_err());

    let user = User::new(1, "test@example.com".to_string());
    let request = request.with_user(user);
    assert!(request.require_user().is_ok());
}

#[test]
fn test_request_with_session() {
    let request = create_test_request_with_fields();
    let session = Session::new("session_123".to_string());

    let request = request.with_session(session);

    assert!(request.session().is_some());
    assert_eq!(request.session().unwrap().id, "session_123");
}

#[test]
fn test_request_require_session() {
    let request = create_test_request_with_fields();
    assert!(request.require_session().is_err());

    let session = Session::new("session_123".to_string());
    let request = request.with_session(session);
    assert!(request.require_session().is_ok());
}

#[test]
fn test_request_merge() {
    let mut request = create_test_request_with_fields();

    let mut additional = HashMap::new();
    additional.insert("phone".to_string(), json!("123-456-7890"));

    request.merge(additional);

    assert!(request.has("phone"));
    let phone: String = request.get("phone").unwrap();
    assert_eq!(phone, "123-456-7890");
}

#[test]
fn test_request_method_and_uri() {
    let request = create_test_request_with_fields();

    assert_eq!(request.method(), &http::Method::POST);
    assert_eq!(request.uri().path(), "/test");
}

#[test]
fn test_request_headers() {
    let request = create_test_request_with_fields();

    assert_eq!(request.header("content-type"), Some("application/json"));
    assert!(request.is_json());
}
