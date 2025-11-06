# RustForge Testing Guide

This document provides comprehensive information about testing in the RustForge framework.

## Table of Contents

- [Overview](#overview)
- [Test Organization](#test-organization)
- [Running Tests](#running-tests)
- [Writing Tests](#writing-tests)
- [Integration Tests](#integration-tests)
- [End-to-End Tests](#end-to-end-tests)
- [Code Coverage](#code-coverage)
- [CI/CD Testing](#cicd-testing)
- [Best Practices](#best-practices)

## Overview

RustForge uses a comprehensive testing strategy that includes:

- **Unit Tests**: Located alongside source code in `src/` directories
- **Integration Tests**: Located in `tests/integration/`
- **End-to-End Tests**: Located in `tests/e2e/`
- **Benchmarks**: Located in `benches/`

## Test Organization

### Directory Structure

```
Rust_DX-Framework/
├── crates/
│   └── */
│       ├── src/
│       │   └── lib.rs          # Unit tests here
│       └── tests/              # Integration tests
├── tests/
│   ├── integration/            # Framework-wide integration tests
│   │   ├── test_framework_bootstrap.rs
│   │   ├── test_command_execution.rs
│   │   ├── test_database_operations.rs
│   │   ├── test_auth_flow.rs
│   │   ├── test_oauth_flow.rs
│   │   └── test_api_endpoints.rs
│   └── e2e/                    # End-to-end tests
│       └── test_complete_application_lifecycle.rs
└── benches/                    # Performance benchmarks
    ├── framework_benchmarks.rs
    └── database_benchmarks.rs
```

### Test Types

#### Unit Tests
Unit tests are located in the same file as the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
}
```

#### Integration Tests
Integration tests are in `tests/integration/`:

```rust
#[tokio::test]
async fn test_database_connection() {
    let db = setup_test_db().await;
    assert!(db.is_ok());
}
```

#### End-to-End Tests
E2E tests validate complete user journeys:

```rust
#[tokio::test]
async fn test_user_registration_to_authentication_flow() {
    let user_id = register_new_user(...).await;
    let token = login_user(...).await;
    assert!(access_protected_resource(&token).await);
}
```

## Running Tests

### Quick Commands

```bash
# Run all tests
cargo test --workspace --all-features

# Run tests for a specific crate
cargo test -p foundry-application

# Run a specific test
cargo test test_database_connection

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel
cargo test -- --test-threads=4

# Run ignored tests
cargo test -- --ignored
```

### Using Cargo Aliases

We've configured convenient aliases in `.cargo/config.toml`:

```bash
# Run all tests (alias)
cargo test-all

# Run quick tests (no features)
cargo test-quick

# Run documentation tests
cargo test-doc
```

### Running Integration Tests Only

```bash
# Run all integration tests
cargo test --test '*' --all-features

# Run specific integration test file
cargo test --test test_database_operations
```

### Running E2E Tests

```bash
# Run all e2e tests
cargo test --test test_complete_application_lifecycle
```

## Writing Tests

### Test Naming Convention

- Test functions should start with `test_`
- Use descriptive names: `test_user_can_login_with_valid_credentials`
- Group related tests in modules

### Async Tests

For async code, use `#[tokio::test]`:

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = some_async_function().await;
    assert!(result.is_ok());
}
```

### Test Fixtures

Create helper functions for common test setup:

```rust
async fn setup_test_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:").await.unwrap()
}

#[tokio::test]
async fn test_something() {
    let db = setup_test_db().await;
    // Use db...
}
```

### Mocking

For mocking dependencies, consider:

- **mockall**: For mocking traits
- **wiremock**: For HTTP mocking
- Test doubles for database operations

### Testing Error Cases

Always test both success and failure paths:

```rust
#[test]
fn test_validation_fails_with_invalid_email() {
    let result = validate_email("invalid");
    assert!(result.is_err());
}
```

## Integration Tests

### Database Tests

Integration tests for database operations:

```rust
#[tokio::test]
async fn test_database_migration() {
    let db = setup_test_db().await;
    let result = apply_migrations(&db).await;
    assert!(result.is_ok());
}
```

### API Tests

Testing API endpoints:

```rust
#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_router();
    let response = app.oneshot(
        Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap()
    ).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

### Authentication Tests

Testing auth flows:

```rust
#[tokio::test]
async fn test_jwt_token_generation() {
    let token = generate_jwt("user123").await;
    assert!(token.is_ok());
    assert!(verify_jwt(&token.unwrap()).await.is_ok());
}
```

## End-to-End Tests

E2E tests validate complete application workflows:

```rust
#[tokio::test]
async fn test_complete_crud_lifecycle() {
    // Create
    let id = create_entity("Test").await.unwrap();

    // Read
    let entity = read_entity(id).await.unwrap();
    assert_eq!(entity.name, "Test");

    // Update
    update_entity(id, "Updated").await.unwrap();

    // Delete
    delete_entity(id).await.unwrap();
    assert!(read_entity(id).await.is_none());
}
```

## Code Coverage

### Generating Coverage Reports

Using `cargo-llvm-cov`:

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --workspace --all-features --html

# Generate LCOV format (for CI)
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
```

### Coverage Targets

- **Minimum**: 70% code coverage
- **Target**: 80%+ code coverage
- **Critical paths**: 90%+ coverage for auth, database, API

### Viewing Coverage

After generating HTML coverage:

```bash
open target/llvm-cov/html/index.html
```

## CI/CD Testing

### GitHub Actions Workflows

Our CI pipeline runs tests automatically:

1. **On Pull Requests**: All tests must pass
2. **On Push to Main**: Full test suite + coverage
3. **Nightly**: Extended tests + benchmarks

### CI Test Matrix

Tests run on:
- **OS**: Ubuntu, macOS, Windows
- **Rust**: stable, beta, nightly
- **Features**: All feature combinations

### Running CI Tests Locally

Simulate CI environment:

```bash
# Run the same checks as CI
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Or use our production check script:

```bash
./scripts/production_check.sh
```

## Best Practices

### 1. Test Independence

Each test should be independent:

```rust
#[tokio::test]
async fn test_independent() {
    // Setup
    let db = setup_fresh_db().await;

    // Test
    // ...

    // Cleanup (if needed)
    cleanup_db(db).await;
}
```

### 2. Use Descriptive Assertions

```rust
// Good
assert_eq!(user.email, "test@example.com", "Email should match");

// Better
assert!(
    user.email == "test@example.com",
    "Expected email 'test@example.com', got '{}'",
    user.email
);
```

### 3. Test Edge Cases

```rust
#[test]
fn test_edge_cases() {
    assert!(validate_age(0).is_err());      // Zero
    assert!(validate_age(-1).is_err());     // Negative
    assert!(validate_age(150).is_err());    // Too large
    assert!(validate_age(25).is_ok());      // Valid
}
```

### 4. Organize Tests with Modules

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod authentication {
        use super::*;

        #[test]
        fn test_login() { }

        #[test]
        fn test_logout() { }
    }

    mod authorization {
        use super::*;

        #[test]
        fn test_permissions() { }
    }
}
```

### 5. Use Test Helpers

```rust
#[cfg(test)]
mod helpers {
    pub fn create_test_user() -> User {
        User {
            id: 1,
            email: "test@example.com".to_string(),
            // ...
        }
    }
}

#[test]
fn test_with_helper() {
    let user = helpers::create_test_user();
    assert_eq!(user.id, 1);
}
```

### 6. Document Complex Tests

```rust
/// Tests the complete OAuth2 authorization code flow including:
/// 1. Authorization request
/// 2. User consent
/// 3. Token exchange
/// 4. Token refresh
#[tokio::test]
async fn test_oauth2_authorization_flow() {
    // Implementation
}
```

### 7. Parameterized Tests

```rust
#[test]
fn test_email_validation() {
    let test_cases = vec![
        ("user@example.com", true),
        ("invalid", false),
        ("@example.com", false),
        ("user@", false),
    ];

    for (email, expected) in test_cases {
        assert_eq!(
            is_valid_email(email),
            expected,
            "Failed for email: {}",
            email
        );
    }
}
```

### 8. Continuous Integration

- All tests must pass before merging
- Code coverage must not decrease
- New features must include tests
- Flaky tests should be fixed immediately

## Performance Testing

See [BENCHMARKS.md](./BENCHMARKS.md) for detailed information about:
- Running benchmarks
- Interpreting results
- Performance regression testing

## Troubleshooting

### Tests Hang

If tests hang, it's often due to:
- Deadlocks in async code
- Waiting for network timeouts
- Database connections not closing

Add timeouts:

```rust
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_with_timeout() {
    let result = timeout(
        Duration::from_secs(5),
        some_operation()
    ).await;

    assert!(result.is_ok(), "Test timed out");
}
```

### Flaky Tests

Common causes:
- Race conditions
- Time-dependent logic
- External dependencies

Solutions:
- Use fixed test data
- Mock time
- Isolate tests properly

### Database Tests Failing

Ensure:
- Database is running (for integration tests)
- Migrations are applied
- Test data is isolated
- Using transactions for rollback

## Resources

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [Criterion.rs Benchmarking](https://github.com/bheisler/criterion.rs)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)

## Contributing

When contributing:
1. Write tests for new features
2. Ensure all tests pass locally
3. Add integration tests for API changes
4. Update this guide if adding new test patterns
