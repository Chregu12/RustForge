# RustForge Testing Infrastructure

This directory contains integration tests, end-to-end tests, and test infrastructure for the RustForge framework.

## Directory Structure

```
tests/
├── docker-compose.test.yml    # Test infrastructure (PostgreSQL, Redis, MinIO)
├── integration/               # Integration tests
│   ├── p0_complete_test.rs   # P0 critical features integration tests
│   ├── test_auth_flow.rs     # Authentication flow tests
│   ├── test_database_operations.rs
│   └── ...
├── e2e/                       # End-to-end tests
│   └── test_complete_application_lifecycle.rs
├── support/                   # Test helpers and utilities
│   └── mod.rs                # Test helper functions
└── README.md                  # This file
```

## Prerequisites

1. **Docker & Docker Compose** installed
2. **Rust toolchain** (stable)
3. **Environment variables** configured (optional)

## Quick Start

### 1. Start Test Infrastructure

Start all test services (PostgreSQL, Redis, MinIO):

```bash
# From project root
docker-compose -f tests/docker-compose.test.yml up -d

# Check services are healthy
docker-compose -f tests/docker-compose.test.yml ps
```

### 2. Run Tests

```bash
# Run all tests (excluding ignored ones)
cargo test

# Run integration tests only
cargo test --test '*' --features test-integration

# Run ignored tests (requires database)
cargo test -- --ignored

# Run specific P0 integration tests
cargo test --test p0_complete_test -- --ignored
```

### 3. Stop Test Infrastructure

```bash
docker-compose -f tests/docker-compose.test.yml down
```

## Test Categories

### Unit Tests
Located in individual crate files (`src/` directories).
Run with: `cargo test --lib`

### Integration Tests
Located in `tests/integration/`.
Test multiple components working together.
Run with: `cargo test --test '*'`

### End-to-End Tests
Located in `tests/e2e/`.
Test complete application workflows.
Run with: `cargo test --test test_complete_application_lifecycle`

### Ignored Tests
Tests marked with `#[ignore]` require external dependencies (database, Redis, etc.).

**Count:** 89 ignored tests

Run with: `cargo test -- --ignored`

## P0 Integration Tests

The P0 integration tests verify the three critical features:

1. **P0-1: Eloquent Relationships**
   - HasMany, BelongsTo, BelongsToMany
   - Tests actual database queries

2. **P0-2: Database Validation Rules**
   - Unique validation
   - Exists validation
   - Foreign key checks

3. **P0-3: Eager Loading**
   - N+1 query prevention
   - Performance benchmarks
   - Query count verification

### Running P0 Tests

```bash
# Start test database
docker-compose -f tests/docker-compose.test.yml up -d postgres

# Run P0 integration tests
cargo test --test p0_complete_test -- --ignored

# Run with output
cargo test --test p0_complete_test -- --ignored --nocapture
```

### Current Status

⚠️ **All P0 tests are currently marked as `#[ignore]`** because the implementations are not complete.

**P0-1 Status:** ❌ Not Implemented (returns empty data)
**P0-2 Status:** ❌ Not Implemented (returns hardcoded error)
**P0-3 Status:** ❌ Not Implemented (does nothing)

Tests will be enabled incrementally as implementations are completed.

## Environment Variables

Set these environment variables to customize test configuration:

```bash
# Database
export TEST_DATABASE_URL="postgres://test:test@localhost:5432/rustforge_test"

# Redis
export TEST_REDIS_URL="redis://localhost:6379"

# MinIO/S3
export TEST_S3_ENDPOINT="http://localhost:9000"
export TEST_S3_ACCESS_KEY="minioadmin"
export TEST_S3_SECRET_KEY="minioadmin"
```

Or create a `.env.test` file:

```env
TEST_DATABASE_URL=postgres://test:test@localhost:5432/rustforge_test
TEST_REDIS_URL=redis://localhost:6379
TEST_S3_ENDPOINT=http://localhost:9000
TEST_S3_ACCESS_KEY=minioadmin
TEST_S3_SECRET_KEY=minioadmin
```

## Test Infrastructure Services

### PostgreSQL (Port 5432)
- Database: `rustforge_test`
- User: `test`
- Password: `test`
- Data stored in tmpfs (memory) for speed

### Redis (Port 6379)
- No authentication by default
- Data stored in tmpfs (memory)

### MinIO (Ports 9000, 9001)
- S3-compatible object storage
- Root user: `minioadmin`
- Root password: `minioadmin`
- Console: http://localhost:9001

### MySQL (Port 3306) - Optional
- Database: `rustforge_test`
- User: `test`
- Password: `test`
- Root password: `root`

## Writing Tests

### Integration Test Template

```rust
#[tokio::test]
#[ignore = "requires database"]  // Remove when ready
async fn test_my_feature() {
    let db = setup_test_db().await;

    // Create test data
    let user = User::create(&db, UserData {
        email: "test@example.com",
        name: "Test User",
    }).await.expect("Failed to create user");

    // Test your feature
    let result = my_feature(&db, user.id).await;

    // Assert expectations
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name, "Test User");
}
```

### Query Counter Template

```rust
#[tokio::test]
#[ignore = "requires database"]
async fn test_query_performance() {
    let db = setup_test_db_with_counter().await;

    db.reset_query_counter();

    // Execute your code
    let users = User::with("posts").get(&db).await?;

    // Verify query count
    let query_count = db.query_count();
    assert_eq!(query_count, 2, "Should execute only 2 queries");
}
```

## Troubleshooting

### Tests fail with "Connection refused"
**Solution:** Make sure Docker services are running:
```bash
docker-compose -f tests/docker-compose.test.yml ps
```

### Database schema not found
**Solution:** Run migrations first:
```bash
cargo run --bin migrate -- up
```

### Tests hang indefinitely
**Solution:** Check for database deadlocks or increase timeout:
```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_with_timeout() {
    tokio::time::timeout(
        Duration::from_secs(30),
        my_test()
    ).await.expect("Test timed out");
}
```

### Port already in use
**Solution:** Stop existing services:
```bash
docker-compose -f tests/docker-compose.test.yml down
# Or kill specific ports
lsof -ti:5432 | xargs kill -9
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_DB: rustforge_test
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

      redis:
        image: redis:7
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run tests
        run: cargo test -- --ignored
        env:
          TEST_DATABASE_URL: postgres://test:test@localhost:5432/rustforge_test
          TEST_REDIS_URL: redis://localhost:6379
```

## Performance Benchmarks

Run performance benchmarks to measure improvements:

```bash
# Run benchmark suite
cargo test --test p0_complete_test benchmark -- --ignored --nocapture

# Example output:
# Performance Benchmark Results:
#   N+1 Problem:  101 queries, 523ms
#   Eager Load:   2 queries, 15ms
#   Time saved:   97.1%
#   Queries saved: 98.0%
```

## Coverage Reports

Generate test coverage reports:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage

# Open report
open coverage/index.html
```

## Test Metrics

**Current Status:**
- Total tests: ~150
- Ignored tests: 89 (59.3%)
- Passing tests: ~61
- Coverage: ~45%

**Target Status:**
- Total tests: 200+
- Ignored tests: 0 (0%)
- Passing tests: 200+
- Coverage: 70%+

## Contributing

When adding tests:

1. ✅ Use descriptive test names
2. ✅ Add `#[ignore]` if requires external services
3. ✅ Document expected behavior
4. ✅ Clean up test data after test
5. ✅ Use test helpers for common setup
6. ✅ Add performance assertions where relevant

## Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [SeaORM Testing](https://www.sea-ql.org/SeaORM/docs/write-test/testing/)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [Docker Compose Documentation](https://docs.docker.com/compose/)

---

**Last Updated:** 2025-11-15
**Maintained By:** QA Team
