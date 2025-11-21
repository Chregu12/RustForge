# rf-errors

Comprehensive error handling with user-friendly messages for RustForge framework.

## Features

- **User-Friendly Error Messages** - Helpful, actionable error messages with context and troubleshooting steps
- **Error Codes** - Structured error codes (RF001-RF999) for easy documentation searching
- **Development Mode** - Full stack traces, syntax highlighting, code snippets, and variable inspection
- **Production Mode** - Generic error messages without sensitive data exposure
- **Error Reporting** - Sentry integration and custom reporter support
- **Error Pages** - Customizable HTML error pages for different HTTP status codes
- **Context Tracking** - Automatic sensitive data sanitization and request correlation
- **Type Safety** - Structured error types with exhaustive pattern matching

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-errors = { path = "../rf-errors" }
```

### Basic Usage

```rust
use rf_errors::{RustForgeError, error::DatabaseError};

// Create an error
let error = DatabaseError::connection("localhost:5432", "mydb", "postgres");
let rf_error = RustForgeError::Database(error);

// Error code
println!("Error code: {}", rf_error.code().code()); // "RF001"

// HTTP status
println!("Status: {}", rf_error.status_code()); // 500
```

### Development Mode Display

```rust
use rf_errors::dev_mode::format_dev_error;

let error = DatabaseError::connection("localhost", "db", "user");
let rf_error = RustForgeError::Database(error);

// Rich, colorful terminal output with suggestions
println!("{}", format_dev_error(&rf_error));
```

Output:
```
┌─────────────────────────────────────────────┐
│ RustForge Error (RF001)                     │
├─────────────────────────────────────────────┤
│ Database Connection Failed                  │
│                                             │
│ The application couldn't connect to the     │
│ database.                                   │
│                                             │
│ Caused by:                                  │
│   • Database server is not running          │
│   • Incorrect credentials in .env file      │
│   • Network/firewall blocking connection    │
│                                             │
│ How to fix:                                 │
│   1. Check if PostgreSQL is running         │
│   2. Verify DATABASE_URL in .env file       │
│   3. Test connection: psql -h localhost...  │
│                                             │
│ Documentation:                              │
│   https://docs.rustforge.dev/errors/RF001   │
└─────────────────────────────────────────────┘
```

### Production Mode Display

```rust
use rf_errors::prod_mode::format_prod_json;

let error = DatabaseError::connection("localhost", "db", "user");
let rf_error = RustForgeError::Database(error);

// Safe JSON response without sensitive data
let json = format_prod_json(&rf_error);
```

Output:
```json
{
  "error": {
    "message": "An unexpected error occurred. Please try again later.",
    "code": "RF001",
    "request_id": "req_abc123xyz",
    "timestamp": "2025-11-16T12:30:00Z",
    "status": 500
  }
}
```

### Error Context

```rust
use rf_errors::{ErrorContext, error_location};

let mut error = DatabaseError::connection("localhost", "db", "user");

// Add context
let context = ErrorContext::new()
    .with_location(error_location!("connect_to_db"))
    .with_request_id("req_123")
    .with_user_id("user_456")
    .with_path("/api/users")
    .with_method("GET")
    .with_value("host", "localhost")
    .with_value("password", "secret123");  // Automatically sanitized!

error = error.with_context(context);
```

### Error Reporting

```rust
use rf_errors::reporting::{LoggingReporter, ErrorReporter, ErrorLevel};

// Use built-in logging reporter
let reporter = LoggingReporter::new()
    .with_level(ErrorLevel::Error);

reporter.report(&error, &context).await;
```

#### Sentry Integration

```rust
use rf_errors::reporting::SentryReporter;

let reporter = SentryReporter::new(
    "https://key@sentry.io/project",
    "production"
)
.with_release("1.0.0")
.with_level(ErrorLevel::Error);

// Initialize Sentry
let _guard = reporter.init();

// Report errors
reporter.report(&error, &context).await;
```

### Error Pages

```rust
use rf_errors::ErrorPages;

let pages = ErrorPages::new()
    .set_page(404, "errors/404.blade.php")
    .set_page(500, "errors/500.blade.php");

// Render error page
let html = pages.render(&error);
```

## Error Codes

All errors have a unique code for documentation and troubleshooting:

| Code Range | Category | Examples |
|------------|----------|----------|
| RF001-RF099 | Database | Connection, queries, migrations |
| RF100-RF199 | Validation | Field validation, constraints |
| RF200-RF299 | Authentication | Login, tokens, sessions |
| RF300-RF399 | Authorization | Permissions, policies |
| RF400-RF499 | Cache | Redis operations |
| RF500-RF599 | Queue | Background jobs |
| RF600-RF699 | HTTP | Routes, requests |
| RF700-RF799 | Template | Blade rendering |
| RF800-RF899 | Storage | File operations, S3 |
| RF900-RF999 | General | Configuration, system |

See [ERROR_CODES.md](../../docs/ERROR_CODES.md) for complete reference.

## Error Types

### Database Errors
```rust
DatabaseError::connection(host, database, user)
DatabaseError::query(query, error)
// ... etc
```

### Validation Errors
```rust
ValidationError::new(field, message)
    .with_value("invalid-value")
```

### Authentication Errors
```rust
AuthenticationError::invalid_credentials()
AuthenticationError::token_expired()
```

### HTTP Errors
```rust
HttpError::not_found(resource)
HttpError::rate_limit_exceeded()
```

## Features

### Default Features
- `error-pages` - HTML error page rendering
- `sentry-integration` - Sentry error reporting

### Optional Features
```toml
[dependencies]
rf-errors = { path = "../rf-errors", default-features = false }
# Or enable specific features:
rf-errors = { path = "../rf-errors", features = ["sentry-integration"] }
```

## Security

### Automatic Data Sanitization

Sensitive fields are automatically redacted:
- `password`, `passwd`, `pwd`
- `secret`, `token`, `key`
- `api_key`, `apikey`
- `credential`, `private`
- `session`, `cookie`
- `ssn`, `credit_card`, `cvv`

```rust
let context = ErrorContext::new()
    .with_value("email", "user@example.com")  // OK
    .with_value("password", "secret123");      // Redacted as "***REDACTED***"
```

Nested values are also sanitized:
```rust
let data = json!({
    "user": "john",
    "credentials": {
        "password": "secret"  // Will be "***REDACTED***"
    }
});
```

## Testing

Run tests:
```bash
cargo test --package rf-errors
```

The crate includes **90 comprehensive tests** covering:
- Error code formatting (4 tests)
- Error context tracking (7 tests)
- Database errors (3 tests)
- Validation errors (2 tests)
- Friendly error messages (5 tests)
- Development mode display (3 tests)
- Production mode display (10 tests)
- Error reporting (3 tests)
- Error pages (3 tests)
- Integration tests (35 tests)
- Plus additional tests in submodules

## Examples

See the [integration tests](tests/integration_test.rs) for comprehensive usage examples.

## Documentation

- [Error Codes Reference](../../docs/ERROR_CODES.md) - Complete error code documentation
- [API Documentation](https://docs.rustforge.dev/errors) - Full API reference

## License

MIT OR Apache-2.0
