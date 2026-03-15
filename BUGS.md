# RustForge - Known Issues & Bug Tracker

This file documents known issues, incomplete implementations, and technical debt
across the RustForge framework. Issues are organized by severity and crate.

---

## Already Fixed (Session: 2026-03-15)

| # | Crate | Issue | Commit |
|---|-------|-------|--------|
| 1 | `rf-web` | **CRITICAL SECURITY**: CSRF middleware only checked header existence, never validated token value against server-side store | `b3cef85` |
| 2 | `rf-web` | **CRITICAL**: Session middleware used `futures::executor::block_on()` inside async → deadlock under Tokio | `b3cef85` |
| 3 | `rf-web` | Multiple `unwrap()` panics in session middleware during session creation and cookie parsing | `b3cef85` |
| 4 | `rf-web` | `Set-Cookie` header used `insert()` instead of `append()`, silently dropping other cookies | `b3cef85` |
| 5 | `rf-jobs` | `SerializedJob` was `pub(crate)` but used in public `QueueManager` methods in batch.rs and chain.rs | latest |
| 6 | `rf-jobs` | Duplicate `clone_schedules` implementation - inherent method shadowed by trait method, unused dead code | latest |
| 7 | `rf-jobs` | `WorkerPool.queue_manager` and `WorkerPool.registry` stored but never read after construction | latest |
| 8 | `rf-jobs` | `ScheduledJob.job_factory` stored but never called - scheduler can't dispatch jobs | latest |
| 9 | `rf-orm` | Unused imports: `Func`, `SelectStatement`, `SimpleExpr`, `OrderedStatement` in query_builder.rs | latest |
| 10 | `rf-orm` | `integration-tests` feature flag referenced but not defined in Cargo.toml | latest |
| 11 | `rf-orm` | Unused variable `result` in `pool_optimizer.rs::health_check()` | latest |
| 12 | `rf-orm` | `async fn` in public trait without `Send` bound (`Model` facade) | latest |
| 13 | `rf-response` | Unused `Json` import | latest |
| 14 | `rf-mail` | Duplicate `Mailable` trait in `mailer.rs` shadowed by `mailable.rs::MailableAsync` | latest |
| 15 | `rf-web` | Unused imports: `HeaderValue`, `async_trait` in versioning.rs | latest |
| 16 | `rf-validation-derive` | `field.ident.as_ref().unwrap()` panics on tuple struct fields | latest |
| 17 | Workspace | `rf-command-events`, `rf-command-pipeline`, `rf-advanced-input`, `rf-infra`, `rf-api`, `rf-oauth`, `rf-oauth-server`, `foundry-cli` incorrectly disabled - all compile cleanly | latest |
| 18 | `rf-cache` | `MemoryCache::delete()` did not remove key from tag sets — orphaned tag references accumulated forever | latest |
| 19 | `rf-cache` | `Cache::increment()`/`decrement()` default impl always reset TTL to hardcoded 86400s; `MemoryCache` now overrides to preserve the remaining TTL of the original entry | latest |
| 20 | `rf-scheduler` | `ScheduledTask.running` flag was set to `false` immediately after `tokio::spawn()` before the spawned task completed — race condition allowed duplicate concurrent execution | latest |
| 21 | `rf-scheduler` | Overlap prevention only guarded tasks with `prevent_overlap() = true`; non-overlapping tasks were never inserted into `running_tasks` leaving them completely untracked | latest |
| 22 | `rf-broadcasting` | `AuthToken::sign()` used `std::collections::hash_map::DefaultHasher` (non-deterministic, non-cryptographic) for token signing — replaced with proper HMAC-SHA256 (RFC 2104) | latest |
| 23 | `rf-broadcasting` | `WebSocketHandler` spawned forwarding tasks on subscribe but never stored their `JoinHandle` — unsubscribing or disconnecting left orphaned tasks running; now aborts on unsubscribe/disconnect | latest |

---

## Open Issues - High Priority

### `rf-jobs/src/scheduler.rs` - Scheduler Cannot Dispatch Jobs
**Severity**: High
**File**: `crates/rf-jobs/src/scheduler.rs:162-176`

The `Scheduler` stores `job_factory` closures in `ScheduledJob` but the `run_scheduler()` loop
only extracts `(Schedule, String)` pairs via `clone_schedules()`, discarding the factory. This means
scheduled jobs are never actually dispatched. The code explicitly logs:
```
"Job dispatching not yet implemented (needs job registry)"
```
**Fix needed**: Integrate `JobRegistry` into `Scheduler` and call `registry.execute()` when a cron
trigger fires, similar to how `Worker::execute_job_payload()` works.

---

### `rf-application/src/auth/database.rs` - User Creation Not Implemented
**Severity**: High
**File**: `crates/rf-application/src/auth/database.rs:87`

The `create_user()` method does not actually create users in the database. It attempts to retrieve
a user as a workaround, returning an error if not found. Affects all registration flows.

**Fix needed**: Implement proper SeaORM entity-based user creation using `ActiveModel`.

---

### `rf-application/src/commands/tier3/admin.rs` - Admin CRUD Stubs
**Severity**: Medium
**File**: `crates/rf-application/src/commands/tier3/admin.rs:85-116`

All CRUD operations (`list`, `get`, `create`, `update`, `delete`, `validate`) return empty/passthrough
responses. The admin panel is non-functional for real data.

---

## Open Issues - Medium Priority

### `rf-orm` - Duplicate Eager Loading Implementations
**Severity**: Medium
**Files**:
- `crates/rf-eloquent/src/eager_loading.rs`
- `crates/rf-eloquent/src/eager_loading_optimized.rs`

Two separate eager loading implementations exist with no clear guidance on which to use.
The `_optimized` variant likely supersedes the original but both are exported.

**Fix needed**: Deprecate or remove `eager_loading.rs`, promote `eager_loading_optimized.rs`
as the canonical implementation.

---

### `rf-queue` - No Exponential Backoff for Connection Retries
**Severity**: Medium
**Files**: `crates/rf-queue/src/redis.rs`, `crates/rf-jobs/src/queue.rs`

Redis connection errors are immediately returned without retry logic. In production, transient
connection failures should be retried with exponential backoff before failing a job.

---

### `rf-web/src/csrf.rs` - CSRF Form Body Token Not Extracted
**Severity**: Medium
**File**: `crates/rf-web/src/csrf.rs:168-181`

The `extract_token()` method explicitly comments that form body parsing is not implemented:
```rust
// Then try to get from form data
// Note: This is simplified - in production, you'd need to properly parse the body
// while preserving it for the handler
None
```
This means form-based CSRF (e.g. `<input type="hidden" name="_token">`) is never validated,
only `X-CSRF-TOKEN` headers work.

**Fix needed**: Implement multipart/form-data and `application/x-www-form-urlencoded` body
parsing that preserves the body for downstream handlers.

---

### `rf-cache` - In-Memory Cache Has No Background Eviction
**Severity**: Medium

The in-memory cache backend has no proactive TTL expiration. Expired entries are only removed
lazily when `get()` is called on them — entries set with `set_with_ttl()` and never read again
will occupy memory forever in long-running applications.

**Fix needed**: Spawn a background cleanup task on `MemoryCache::new()` that periodically
sweeps `entries` and removes all items where `entry.is_expired()`.

---

### `rf-broadcasting/src/auth.rs` - Channel Authorization Not Enforced in WebSocket Handler
**Severity**: High (security)
**File**: `crates/rf-broadcasting/src/websocket.rs:218-222`

The `Subscribe` handler has a `TODO` comment acknowledging that auth tokens for private/presence
channels are logged but never validated. Any client can subscribe to `private-*` or
`presence-*` channels without providing a valid authorization token.

**Fix needed**: Call `authorize_channel()` from `rf-broadcasting::auth` when `channel` starts
with `"private-"` or `"presence-"`, reject the subscription with an `Error` message if authorization fails.

---

### `rf-broadcasting/src/websocket.rs` - Channel Registry Not Sharded
**Severity**: Medium

Uses a single `RwLock<HashMap>` for all WebSocket channels. Under high concurrent connection
counts this creates lock contention. Consider `DashMap` or sharding by channel prefix.

---

## Open Issues - Low Priority

### `rf-scheduler` - No Queue Integration
**Severity**: Medium
**File**: `crates/rf-scheduler/src/lib.rs:244-273`

`Scheduler::start()` runs tasks directly via `tokio::spawn()`. If the process crashes mid-run,
the task is lost with no persistence or retry. Laravel-style schedulers dispatch to a persistent
queue (like `rf-jobs`) so workers can retry on failure.

**Fix needed**: Add an optional `queue_dispatch` mode that serializes the task into a `JobPayload`
and pushes it to `rf-queue`, falling back to inline `tokio::spawn` when no queue is configured.

---

### `rf-jobs/src/scheduler.rs` - Scheduler Only Checks Once Per Minute
**Severity**: Low

The scheduler sleeps 30 seconds between checks but only dispatches if `current_minute != last_minute`,
effectively limiting precision to 1-minute granularity. Sub-minute cron expressions are silently ignored.

---

### Test Coverage ~0%
**Severity**: Low (for now, will become High as framework matures)

With 1,104 source files and only a handful of test modules, the framework has minimal unit test
coverage. Critical paths like ORM query building, authentication flows, and job processing need
dedicated test suites.

**Priority order for tests**:
1. `rf-orm` (query builder, relationships)
2. `rf-auth` (guards, tokens)
3. `rf-jobs` (dispatch, retry, DLQ)
4. `rf-validation` (all 50+ rules)
5. `rf-web` (middleware stack integration)

---

### `rf-orm/src/facade/model.rs` - String-Based Error Propagation
**Severity**: Low

The `Model` trait returns `Result<_, String>` throughout. This loses error type information
and makes programmatic error handling difficult. Should use a proper `ModelError` enum.

---

### Multiple Crates - Inconsistent Error Types
**Severity**: Low

Five different error systems with no unified trait:
- `rf-core::AppError`
- `rf-orm::DbError`
- `rf-cache::CacheError`
- `rf-queue::QueueError`
- `rf-broadcasting::BroadcastError`

**Fix needed**: Add `From<X> for AppError` conversions or a common `FrameworkError` trait.

---

## Disabled / Deprecated Crates

| Crate | Status | Reason |
|-------|--------|--------|
| `rf-oauth` | Re-enabled | Was incorrectly disabled - compiles fine |
| `rf-oauth-server` | Re-enabled | Was incorrectly disabled - compiles fine |
| `rf-oauth2-server` | Active | Preferred OAuth2 implementation |

> **Note**: `rf-oauth-server` and `rf-oauth2-server` provide overlapping functionality.
> Consider consolidating into a single crate.
