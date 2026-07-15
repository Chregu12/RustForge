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
| `capture_request` DX layer (GET) | ~1.0 µs vs raw axum with `Path` extractor (+114%); see §"RustForge DX vs raw axum" |
| `capture_request` DX layer (POST JSON) | ~0.5 µs vs raw axum with `Json` extractor (+39%); see §"RustForge DX vs raw axum" |
| AsyncBridge per-call | ~3.8 µs cross-thread channel hop; design is correct (reuses one thread), but not suitable for tight inner loops |
| sqlx SQLite async | ~17–20 µs per query due to `spawn_blocking` thread dispatch; unavoidable in the current driver design |
| Validation rule boxing | Each `Validator::rules()` call heap-allocates `Box<dyn Rule>` per rule; acceptable at 1M+/s but could be avoided with static dispatch for known rule sets |
| No profiling of sea-orm layer | These benches use sqlx directly; sea-orm's entity + relation macro overhead is unmeasured |
| No HTTP throughput bench | The oneshot bench measures latency only; throughput under concurrent load (wrk/oha) is not measured here |

---

## Build & Runtime Footprint

> **Measured**: Apple M1 Max · macOS Darwin 25.5.0 · rustc 1.96.0
> (ac68faa20 2026-05-25, Homebrew) · 2026-07-16
>
> **Reproduction** (run from repo root, requires internet on first run):
> ```bash
> bash scripts/build-footprint-bench.sh
> ```
> The script creates a throwaway axum-baseline project in a `mktemp` directory,
> cold-builds both apps with a fresh `target/`, then measures startup time
> and idle RSS.  All numbers are printed as `METRIC=value` lines.

### What is being compared

| App | Description |
|-----|-------------|
| **raw axum** (`axum-baseline`) | Standalone Cargo project (NOT a workspace member) created by the script in a temp dir.  Implements GET /health + GET+POST /posts + GET /posts/{id} using plain axum 0.8 + tokio + serde + an in-memory Vec.  **No database** — no sea-orm / sqlx / sqlite3-sys. |
| **RustForge** (`blog-slice`) | `examples/blog-slice` from this repo.  Same REST surface but written against the `rf` umbrella crate, using `capture_request` middleware, `input()` helpers, the `validate!` DSL, and `sea-orm` / `sqlx` / SQLite for real persistence. |

The raw-axum baseline is intentionally minimal: it represents the smallest useful
axum application.  The RustForge app includes the full rf-\* crate graph **plus**
the ORM/DB stack (`sea-orm`, `sqlx`, `sqlite3-sys`).  Both axes of cost — the
DX/framework layer and the DB layer — are therefore reflected in the numbers.

### Metrics

| Metric | raw axum | RustForge (blog-slice) | Ratio | Notes |
|--------|----------|------------------------|-------|-------|
| **Cold compile time** | **12.8 s** | **6 m 55 s (415 s)** | **~32×** | Fresh target dir, all deps from source; axum-baseline: 53 crates; blog-slice: 481 rlibs |
| **Binary size (stripped)** | **1.09 MB** (1,144,648 B) | **6.03 MB** (6,321,264 B) | **5.5×** | `strip` on macOS arm64 Mach-O; no dSYM generated |
| **Startup time** (boot → first HTTP 200) | **~20 ms** | **~19 ms** | **~1×** | Median of 5 runs, 10 ms polling; blog-slice includes SQLite schema creation on boot |
| **Idle RSS** (after boot, before requests) | **~2.5 MB** (2,528 KB) | **~8.8 MB** (8,976 KB) | **3.5×** | `ps -o rss` on macOS; 1 s after readiness probe |

### Interpretation

**Compile time (32×)**: This is the dominant footprint cost and is honest to
report.  The full rf-\* crate graph plus sea-orm/sqlx is large.  On first build
(no Cargo registry cache) a developer waits roughly 7 minutes instead of 13
seconds.  On subsequent incremental builds (only changed crates recompile) the
gap narrows — a re-build of only `blog-slice` after a code change takes
well under a second.

**Binary size (5.5×)**: A 6 MB stripped arm64 binary is still tiny by modern
standards (comparable to many Go or Zig applications).  The 5× inflation
reflects the generics monomorphisation cost of the ORM + axum extractor stack
being statically compiled in.  If binary size is a hard constraint, consider
stripping further with LTO (`lto = "thin"`) and `opt-level = "z"` in
`[profile.release]`.

**Startup time (~identical)**: Both apps start serving in under 25 ms.  Despite
the blog-slice running a SQLite `CREATE TABLE IF NOT EXISTS` on boot, Tokio's
runtime startup and axum's router construction are fast enough that there is no
measurable difference between the two.  RustForge does **not** impose a slow
boot penalty.

**Idle RSS (3.5×)**: The ~6 MB extra RSS at idle covers: loaded Rust runtime
data segments from the larger binary, the in-memory SQLite database, the
sea-orm connection pool, and the `rf` global state (router registry, DB
singleton).  For a long-running service the incremental cost is marginal; it
becomes relevant only in environments with very tight memory limits (< 20 MB),
such as some edge / serverless runtimes.

### Note on Loco comparison

This document does **not** benchmark [Loco](https://loco.rs/) head-to-head.
Doing so fairly would require building a Loco project with an equivalent
feature set, running it through the same measurement harness, and controlling
for ORM backend choice.  That was out of scope for this cycle.  All numbers
here are **RustForge vs raw axum only**.  Do not infer Loco numbers from
this data.

---

## RustForge DX vs raw axum

> RustForge is built *on top of* axum. The DX layer (`capture_request`
> middleware + `tokio::task_local` scope + `input()` helper) adds overhead vs a
> plain axum handler with typed extractors. The benchmarks here measure that
> overhead honestly and reproducibly.

### Methodology

- Both sides use `tower::ServiceExt::oneshot` — no TCP stack, no network I/O.
- The same response body and status code are returned by both handlers.
- 100 samples, 8 s measurement window, 3 s warm-up per benchmark.
- Source: `benchmarks/benches/dx_vs_raw_axum.rs`

### Reproduction

```bash
# Full run (100 samples × 8 s measurement):
cargo bench -p rustforge-benchmarks --bench dx_vs_raw_axum

# Quick pass (shorter windows):
cargo bench -p rustforge-benchmarks --bench dx_vs_raw_axum -- \
    --measurement-time 5 --warm-up-time 2
```

### Results (Apple M1 Max, rustc 1.96.0, 2026-07-16)

#### GET /users/{id} — path-parameter read

Both handlers return the same JSON `UserResponse`. The difference is HOW they
read the matched path segment `42`:

- **Raw axum**: `Path(id): Path<i64>` typed extractor — axum hands the
  already-matched segment directly to the handler, zero middleware overhead.
- **RustForge DX**: `capture_request` outer layer + `capture_path_params`
  route_layer; handler has no arguments and calls `input::<i64>("id")`.

| Variant | Median | CI 95% | Throughput |
|---------|--------|--------|------------|
| Raw axum (`Path` extractor) | **877.84 ns** | 875–880 ns | ~1.14 M req/s |
| RustForge DX (`capture_request` + `input`) | **1875.1 ns** | 1868–1884 ns | ~533 K req/s |
| **DX overhead (absolute)** | **+997 ns (~1.0 µs)** | | |
| **DX overhead (relative)** | **+114%  (2.14×)** | | |

#### POST /echo — small JSON body field read

Both handlers receive `{"title":"hello"}` (17 bytes). The difference is body
parsing strategy:

- **Raw axum**: `Json(body): Json<Value>` extractor — axum buffers + parses JSON
  and passes the parsed value directly to the handler.
- **RustForge DX**: `capture_request` outer layer buffers + parses JSON into an
  intermediate `HashMap<String, Value>`, then handler reads `input::<String>("title")`.

| Variant | Median | CI 95% | Throughput |
|---------|--------|--------|------------|
| Raw axum (`Json` extractor) | **1304.2 ns** | 1302–1307 ns | ~767 K req/s |
| RustForge DX (`capture_request` + `input`) | **1806.8 ns** | 1803–1811 ns | ~553 K req/s |
| **DX overhead (absolute)** | **+503 ns (~0.5 µs)** | | |
| **DX overhead (relative)** | **+39%** | | |

The POST gap is smaller than the GET gap because raw axum *also* does body
buffering and JSON parsing via the `Json` extractor — the two sides share that
cost. The DX-specific increment is the intermediate `HashMap`, the `Arc<RequestContext>`
allocation, and the `tokio::task_local` scope setup.

#### Middleware isolation — `capture_request` alone (empty GET, no body)

This isolates the pure middleware cost from the handler logic: the SAME no-op
handler (`async fn noop() -> &'static str { "ok" }`), with vs without the
`capture_request` layer.

| Variant | Median | CI 95% |
|---------|--------|--------|
| Raw axum (no middleware, no extractor) | **515.40 ns** | 514–516 ns |
| `capture_request` (empty body, no fields) | **1076.6 ns** | 1072–1081 ns |
| **Middleware overhead alone** | **+561 ns (~0.56 µs)** | |
| **Overhead ratio** | **+109% (2.09×)** | |

What that 561 ns buys per request (even with an empty body):
- `parse_request`: query-string branch check, one `HashMap::new()`, one `if let Some(query)` (None for this bench)
- `Arc::new(RequestContext { fields: HashMap::new(), files: HashMap::new() })`
- `CURRENT_REQUEST.scope(ctx, next.run(inner)).await` — task-local future wrapper

### Interpretation

**The DX overhead is real but context-dependent:**

| Scenario | DX overhead | Realistic bottleneck | Overhead significance |
|----------|-------------|---------------------|----------------------|
| GET with path param | ~1.0 µs | SQLite: 17–20 µs; Postgres: 0.5–5 ms | 5–5000× smaller |
| POST with JSON body | ~0.5 µs | Same as above | Same |
| `capture_request` alone | ~0.56 µs | Network RTT: 100+ µs | ~200× smaller |

**For any handler that touches a database or makes a network call, the DX
overhead is invisible**: a single in-memory SQLite query (the fastest possible
database) takes 17–20 µs, which is 17–34× larger than the ~1 µs DX tax. A
real Postgres query over loopback is 0.5–5 ms — 500–5000× larger.

**Where the overhead *does* show up:**

1. Pure in-memory handlers (health-check pings, static string responses) where
   latency is sub-microsecond. If you do not need `input()` or `file()` in such
   a handler, skip `capture_request` on that route.
2. Tight benchmark loops measuring only the routing path — synthetic, not
   representative of real application load.

**Per-route opt-out**: `capture_request` is applied as a layer on the router,
not globally enforced. Routes that don't use `input()` or `file()` can be
placed on a separate sub-router that skips the layer entirely.

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
