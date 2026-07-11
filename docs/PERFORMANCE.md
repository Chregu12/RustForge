# RustForge Performance Baselines

> **IMPORTANT**: These are indicative numbers measured on one specific machine at
> one moment in time. They are NOT guarantees, SLAs, or production throughput
> figures. Always benchmark on your own hardware under your own workload.

---

## Reference Machine

| Field       | Value |
|-------------|-------|
| CPU         | Apple M1 Max |
| OS          | macOS Darwin 25.5.0 (arm64) |
| Rust        | 1.96.0 (ac68faa20 2026-05-25, Homebrew) |
| Profile     | `cargo bench` (`release` + `debug = true`) |
| Date        | 2026-07-11 |

---

## How to Reproduce

```bash
# Compile the hot-path bench (no actual runs):
cargo bench --no-run -p rustforge-benchmarks --bench hot_paths

# Run with shorter warm-up / measurement for a quick pass:
cargo bench -p rustforge-benchmarks --bench hot_paths -- \
    --measurement-time 5 --warm-up-time 2

# Run the N+1 query-count assertion test:
cargo test -p rustforge-benchmarks --test n_plus_1 -- --nocapture
```

---

## Results

### (a) AsyncBridge — sync-over-async handoff overhead

**What it measures**: the cost of `AsyncBridge::block_on(async { 42u64 })`
(no I/O, no sleep) — pure channel round-trip from caller thread to the bridge
worker thread and back.

| Path | Median | CI 95% |
|------|--------|--------|
| `AsyncBridge::block_on` (noop future) | **3.84 µs** | 3.58–4.14 µs |
| `tokio::runtime::Runtime::block_on` (same-thread, noop future) | **90.8 ns** | 90.0–92.0 ns |
| **Overhead ratio** | **~42×** | |

**Interpretation**: the bridge adds ~3.75 µs per call compared to calling a
same-thread Tokio runtime directly. This is the cost of:
- One `tokio::sync::mpsc::UnboundedSender::send` (to the worker thread)
- One `tokio::spawn` on the bridge runtime
- One `std::sync::mpsc::SyncChannel` send (to reply)
- Cross-thread wake-up (OS scheduler hand-off)

**Architecture note (no pathological spawning)**: The bridge does **NOT** spawn
a new thread or runtime per call. It owns ONE dedicated OS thread and ONE
current-thread Tokio runtime, started in `AsyncBridge::new()`. All
`block_on` calls fan into that single worker via an unbounded channel.
Cloning the bridge is cheap — clones share the same worker via `Arc`.
See `crates/rf-async-bridge/src/lib.rs:83-117`.

**When this overhead matters**: ~3.8 µs means the bridge supports
~260K facade calls/second in a single-threaded caller. For batch operations
or async handlers that call the bridge from `spawn_blocking`, the cost
amortizes. Avoid calling it in tight inner loops per-row or per-field.

---

### (b) HTTP handler — axum Router via in-process `tower::ServiceExt::oneshot`

**What it measures**: full axum routing path for a `GET /api/users/{id}`
handler returning a JSON body — no TCP stack, no serialization of headers
over the wire, but full router matching + handler dispatch + body encoding.

| Scenario | Median | CI 95% |
|----------|--------|--------|
| GET JSON handler (oneshot) | **1.09 µs** | 1.06–1.12 µs |
| Theoretical in-process peak | **~918K req/s** | |

**Interpretation**: pure routing + JSON marshal overhead is ~1 µs. With a
real TCP stack (loopback or LAN), this becomes dominated by network latency.
The ~918K req/s figure is a processing ceiling, not a real-world throughput
number — in practice your bottleneck will be I/O, middleware, or DB latency.

---

### (c) SQLite (in-memory, sqlx 0.8, pool max_connections=1)

**What it measures**: raw sqlx query round-trip against an in-memory SQLite
database with a single connection — the fastest possible SQLite configuration.
This approximates the "ORM driver layer" cost excluding sea-orm macro overhead.

| Operation | Median | CI 95% |
|-----------|--------|--------|
| `INSERT` (no unique index) | **17.8 µs** | 17.5–18.1 µs |
| `SELECT by PK` | **19.2 µs** | 18.0–20.4 µs |

**Why ~17–20 µs for in-memory?** sqlx's SQLite driver serialises async
calls through a background blocking thread via `tokio::task::spawn_blocking`.
Each query involves:
- A future poll → `spawn_blocking` → OS thread dispatch
- The actual SQLite in-memory write/read (~1–2 µs)
- Return trip back to the async executor

Even though the DB itself is in RAM, the async ↔ blocking thread hop adds
~15–18 µs of overhead. For a real SQLite file on NVMe, expect 100–500 µs.
For a PostgreSQL server (even local), expect 0.5–5 ms per query.

---

### (d) DTO validation — `rf-validation` Validator (3 fields)

**What it measures**: constructing a `Validator` with three fields
(`name: required + min_length(2)`, `email: required + EmailRule`,
`password: required + min_length(8)`) and running `.validate().await`.

| Scenario | Median | CI 95% | Rate |
|----------|--------|--------|------|
| Passing DTO (all fields valid) | **951 ns** | 946–955 ns | ~1.05M/s |
| Failing DTO (required fails + invalid email + short password) | **1.38 µs** | 1.37–1.38 µs | ~726K/s |

**Interpretation**: a 3-field validation round-trip costs under 1 µs (passing)
or ~1.4 µs (failing, due to extra error collection work). At 1M+ validations/s
on a single core, validation itself is almost never a bottleneck.

**Regex performance cliff — fixed**: prior to this release, `EmailRule`,
`UrlRule`, `IpRule`, and `UuidRule` compiled a new `regex::Regex` on
**every validation call**. Regex compilation takes 10–40 µs (measured
informally), making a single email validation slower than an entire in-memory
SQLite query. This was fixed by caching the compiled `Regex` in a
`std::sync::OnceLock<Regex>` static — one compile at first use, zero cost on
subsequent calls. See `crates/rf-validation/src/rules/string.rs`.

---

## N+1 Query Count Test

The `n_plus_1` integration test (run with
`cargo test -p rustforge-benchmarks --test n_plus_1`) proves that the
"eager loading" pattern issues O(1) queries relative to the number of users:

```
PASS  N+1 queries: 11  |  Eager queries: 2  (ratio 5.5x)
```

For 10 users with posts:
- **Naive loop (N+1)**: 11 queries (1 user query + 1 post query per user)
- **Eager loading**: 2 queries (1 user query + 1 `WHERE user_id IN (...)` query)

The test asserts `eager_queries == 2` regardless of user count. sea-orm's
`find_with_related` API implements this pattern at the ORM level; the test
uses raw sqlx to demonstrate the pattern is correct and countable.

---

## Known Costs and Residual Risks

| Cost | Details |
|------|---------|
| AsyncBridge per-call | ~3.8 µs cross-thread channel hop; design is correct (reuses one thread), but not suitable for tight inner loops |
| sqlx SQLite async | ~17–20 µs per query due to `spawn_blocking` thread dispatch; unavoidable in the current driver design |
| Validation rule boxing | Each `Validator::rules()` call heap-allocates `Box<dyn Rule>` per rule; acceptable at 1M+/s but could be avoided with static dispatch for known rule sets |
| No profiling of sea-orm layer | These benches use sqlx directly; sea-orm's entity + relation macro overhead is unmeasured |
| No HTTP throughput bench | The oneshot bench measures latency only; throughput under concurrent load (wrk/oha) is not measured here |

---

## Load Harness (HTTP path)

A lightweight committed HTTP load script is available at
`benchmarks/benches/hot_paths.rs` — the `http/get_json_handler_oneshot`
bench exercises the full axum routing + JSON handler path in-process via
`tower::ServiceExt::oneshot`.

For real-network load testing, `oha` or `wrk` can target a running server.
Example (not committed, requires a running server on port 3000):

```bash
# oha (install: cargo install oha)
oha -n 100000 -c 64 http://127.0.0.1:3000/api/users/1

# wrk
wrk -t4 -c64 -d30s http://127.0.0.1:3000/api/users/1
```
