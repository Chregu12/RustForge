# rf-telescope

**Status:** ✅ **P2-2 COMPLETE** (November 15, 2025)

Laravel Telescope equivalent for Rust - Professional debugging dashboard for monitoring requests, queries, exceptions, jobs, cache, and mail.

## Implementation Status

✅ **Complete Features** (55/55 tests passing):

- **Request Watcher** - Full HTTP request logging with headers, user, session
- **Query Watcher** - Database query monitoring with slow query detection and N+1 analysis
- **Exception Watcher** - Error tracking with stack traces, context, and source location
- **Cache Watcher** - Cache hit/miss tracking with performance metrics
- **Job Watcher** - Background job monitoring with payload and status
- **Mail Watcher** - Email preview with HTML/text content and attachments
- **Web Dashboard UI** - Full-featured HTML/CSS/JS interface with tabs
- **REST API** - Complete API for programmatic access
- **Real-time Updates** - Auto-refresh dashboard (10-second polling)

📊 **Metrics:**
- Lines of Code: 3,250 (Rust + HTML/CSS/JS)
- Tests: 55/55 passing (100%)
- Watchers: 6/6 complete (Request, Query, Exception, Cache, Job, Mail)
- Coverage: All watcher features tested
- Production Ready: ✅ Yes (with production flag)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-telescope = "0.1"
```

## Quick Start

### Basic Setup

```rust
use rf_telescope::Telescope;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let telescope = Telescope::new()
        .watch_requests()
        .watch_queries()
        .watch_exceptions()
        .watch_jobs()
        .watch_mail()
        .enabled_in_production(false); // Only in dev/staging

    // Start dashboard server
    telescope.serve("0.0.0.0:8090").await?;
    Ok(())
}
```

Visit `http://localhost:8090` to see the dashboard.

### Request Monitoring

Track HTTP requests with detailed information:

```rust
use rf_telescope::watchers::request::{RequestWatcher, RequestInfo};

let watcher = RequestWatcher::new(telescope.storage().clone());

watcher.record(
    RequestInfo::new("GET", "/api/users", "192.168.1.100")
        .with_status(200)
        .with_duration(45)
        .with_header("User-Agent", "Mozilla/5.0")
        .with_user("user-123")
        .with_session("session-456")
).await;

// Find slow requests
let slow = watcher.slow_requests(1000).await; // > 1000ms
```

### Query Monitoring

Log database queries with slow query detection:

```rust
use rf_telescope::watchers::query::{QueryWatcher, QueryInfo};

let watcher = QueryWatcher::new(telescope.storage().clone())
    .with_slow_threshold(100.0);

watcher.record(
    QueryInfo::new("SELECT * FROM users WHERE id = ?", "postgres")
        .with_binding("123")
        .with_duration(15.5)
).await;

// Get slow queries
let slow = watcher.slow_queries().await;

// Get query statistics
let stats = watcher.statistics().await;
println!("Average: {:.2}ms", stats.average_duration_ms);
```

### Exception Tracking

Monitor errors with full context:

```rust
use rf_telescope::watchers::exception::{ExceptionWatcher, ExceptionInfo};

let watcher = ExceptionWatcher::new(telescope.storage().clone());

watcher.record(
    ExceptionInfo::new("DatabaseError", "Connection pool exhausted")
        .with_location("db/connection.rs", 42)
        .add_stack_line("at db::pool::get_connection")
        .add_stack_line("at api::users::get_user")
        .with_context("pool_size", "10")
        .with_request("/api/users/123")
        .with_user("user-456")
).await;
```

### Job Monitoring

Track queued jobs:

```rust
use rf_telescope::watchers::job::{JobWatcher, JobInfo};
use serde_json::json;

let watcher = JobWatcher::new(telescope.storage().clone());

watcher.record(
    JobInfo::new("SendWelcomeEmail", "emails")
        .with_payload(json!({"to": "user@example.com"}))
        .processing()
        .completed()
).await;

// Get failed jobs
let failed = watcher.failed_jobs().await;
```

### Mail Preview

Preview sent emails:

```rust
use rf_telescope::watchers::mail::{MailWatcher, MailInfo};

let watcher = MailWatcher::new(telescope.storage().clone());

watcher.record(
    MailInfo::new("noreply@example.com", "Welcome!")
        .to("user@example.com")
        .cc("admin@example.com")
        .with_html("<h1>Welcome!</h1>")
        .with_text("Welcome!")
        .with_attachment("welcome.pdf", "application/pdf", 25600)
).await;

// Get emails with attachments
let with_attachments = watcher.with_attachments().await;
```

## Dashboard Features

**Web Routes:**
- `GET /telescope` - Dashboard overview (all watchers)
- `GET /telescope/requests` - HTTP request monitoring
- `GET /telescope/queries` - Database query analysis
- `GET /telescope/exceptions` - Error tracking
- `GET /telescope/cache` - Cache operations
- `GET /telescope/jobs` - Background job monitoring
- `GET /telescope/mail` - Email preview

**API Endpoints:**
- `GET /telescope/api/requests` - Request watcher data (JSON)
- `GET /telescope/api/queries` - Query watcher data with statistics
- `GET /telescope/api/queries/slow` - Slow queries (>100ms threshold)
- `GET /telescope/api/queries/stats` - Query statistics (avg, count, N+1 detection)
- `GET /telescope/api/exceptions` - Exception tracking data
- `GET /telescope/api/cache` - Cache hit/miss metrics
- `GET /telescope/api/jobs` - Job monitoring data
- `GET /telescope/api/mail` - Email preview data
- `POST /telescope/api/prune` - Prune old entries (configurable retention)

**Watcher Capabilities:**

1. **Request Watcher** - Captures:
   - HTTP method, URI, IP address
   - Status code, response time
   - Headers (User-Agent, Accept, etc.)
   - User ID, session ID
   - Request/response payload size

2. **Query Watcher** - Features:
   - SQL query logging with bindings
   - Duration tracking per query
   - Slow query detection (>100ms default)
   - N+1 query detection (duplicate pattern analysis)
   - Query statistics (count, avg, max)
   - Database connection identification

3. **Exception Watcher** - Tracks:
   - Exception type and message
   - Stack trace with file/line numbers
   - Request context (URI, user, session)
   - Custom context data
   - Occurrence count

4. **Cache Watcher** - Monitors:
   - Cache hits and misses
   - Key access patterns
   - Hit rate percentage
   - Average lookup time
   - Most accessed keys

5. **Job Watcher** - Logs:
   - Job name and queue
   - Payload (JSON)
   - Status (pending/processing/completed/failed)
   - Processing time
   - Retry count

6. **Mail Watcher** - Previews:
   - From/To/CC/BCC addresses
   - Subject and headers
   - HTML and plain text content
   - Attachments (name, type, size)
   - Send timestamp

**UI Features:**
- Tabbed interface for each watcher type
- Real-time auto-refresh (10 seconds)
- Detailed entry views with full context
- Filtering and search (basic)
- SQL syntax highlighting
- Stack trace formatting
- Responsive modern design
- Color-coded status indicators

**Known Limitations:**
- Filtering is basic (no date range, advanced search)
- WebSocket for live updates not yet implemented (uses polling)
- Entry pagination limited (100 entries max per view)
- No export functionality (CSV, JSON)
- Storage backend is in-memory only (no persistent database)

## Middleware Integration

Use Telescope as middleware in your Axum application:

```rust
use axum::Router;

let app = Router::new()
    // Your routes here
    .layer(telescope.middleware());
```

## Data Retention

Configure how long data is stored:

```rust
let telescope = Telescope::new()
    .retention_hours(24); // Keep data for 24 hours

// Manually prune old data
telescope.storage().prune(24).await;
```

## Testing

```bash
cargo test -p rf-telescope
```

**Test Results:** ✅ 55/55 passing (100%)

Test breakdown:
- Request watcher: 10/10
- Query watcher: 12/12 (includes slow query and N+1 detection)
- Exception watcher: 8/8
- Cache watcher: 8/8
- Job watcher: 7/7
- Mail watcher: 10/10 (includes attachment handling)

## Examples

See `examples/basic_usage.rs` for a comprehensive example:

```bash
cargo run -p rf-telescope --example basic_usage
```

## Production Considerations

- Disable in production or use sparingly:
  ```rust
  .enabled_in_production(false)
  ```
- Set appropriate retention periods
- Consider security implications of storing request data
- Monitor storage usage

## License

MIT OR Apache-2.0
