# Integration Tests

This directory contains integration tests for the RustForge starter template.

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_health_endpoint
```

## Test Structure

- `integration_test.rs` - Full API integration tests
- Each test sets up a fresh database and test environment
- Tests are isolated and can run in parallel

## Writing Tests

Example test structure:

```rust
#[tokio::test]
async fn test_my_feature() {
    // 1. Setup
    let app = create_test_app().await;

    // 2. Execute
    let response = app
        .oneshot(Request::builder()
            .uri("/api/endpoint")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    // 3. Assert
    assert_eq!(response.status(), StatusCode::OK);
}
```

## Test Database

Tests use SQLite in-memory databases by default for speed and isolation.
Each test gets its own database instance.

## Coverage

To generate code coverage:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```
