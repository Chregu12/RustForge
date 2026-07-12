# RustForge Feature-Tier Taxonomy

**Single source of truth** for every workspace crate's maturity classification.
The README maturity matrix and `docs/README.md` reference this file; any
discrepancy between those files and this one is a bug — fix it here and
propagate the change.

---

## Tier Definitions

| Tier | Meaning | SemVer |
|------|---------|--------|
| `stable` | Real engine, CI-tested or probe-verified, used in a shipped example. v1 compat promise. | Covered from v1.0 |
| `beta` | Real core implementation, documented minor gaps or not exhaustively integration-tested. API may shift in minor versions. | Best-effort; no guarantee |
| `experimental` | Exists and compiles but is excluded from `default-members` and the 1.0 supported surface. API may change or be removed without a version bump. | None |
| `stub` | Placeholder crate with no real implementation beyond type definitions or forwarding. | None |

> **Machine-check convention (partial today):** A crate's `Cargo.toml` may carry
> `[package.metadata.rustforge] tier = "<tier>"`. This is populated for the
> stable-core and experimental crates so far (~42 of the ~127 members); the
> remaining beta crates are annotated incrementally, so **this table — not the
> Cargo metadata — is the authoritative source** until annotation is complete.
> Run `grep -rl 'metadata.rustforge' crates/*/Cargo.toml | wc -l` to see coverage.

---

## Stable Crates (34)

These form the **v1.0 supported surface**. APIs here will not break without a major-version bump.

| Crate | Tier | Justification |
|-------|------|---------------|
| `rf-core` | stable | AppError/AppResult/RequestContext — foundation used by every other crate; real RFC 7807 impl |
| `rf-web` | stable | axum 0.8 routing/middleware stack; real CORS, security-headers, JSON 404/405 |
| `rf-request` | stable | Task-local request globals; body buffered + re-inserted so globals and body extractors coexist |
| `rf-response` | stable | JSON/HTML response types wiring axum and rf-core error envelope |
| `rf-macros` | stable | Model!/Route!/validate!/get!/post! proc-macros (12k lines, probe-verified, used in every example) |
| `rf-model-macro` | stable | Model derive proc-macro used internally by rf-orm and rf-macros |
| `rf-validation` | stable | validate! DSL (22 constraint types), 422 with per-field structured errors |
| `rf-validation-derive` | stable | #[derive(Validate)] proc-macro used by rf-validation |
| `rf-orm` | stable | Real SeaORM ORM — create!/find!/update!/delete!, relations, scopes, paginate (35 files, 18.7k lines) |
| `rf-eloquent` | stable | Eloquent-style ORM macros layered on rf-orm; used in examples/phase12-blog |
| `rf-auth` | stable | Auth facade + require_auth middleware; per-request state (no cross-request bleed), bearer-before-body ordering |
| `rf-sanctum` | stable | Transient API token auth (DB-free); real JSON error envelope on 401 |
| `rf-cache` | stable | Cache facade — Memory + optional Redis via AsyncBridge; no block_on panic in async runtime |
| `rf-queue` | stable | MemoryQueue + Worker; priority FIFO, retry, dead-letter (Queue::failed()), panic-isolation |
| `rf-jobs` | stable | Redis-backed WorkerPool; requires live Redis (examples/jobs-demo) |
| `rf-mail` | stable | Mail facade — real lettre SMTP + FileMailer; queued mail drains to configured transport |
| `rf-storage` | stable | Storage facade — Local + S3 via AsyncBridge; path-traversal-safe; 413 on oversize upload |
| `rf-events` | stable | Type-keyed sync event bus; listener panics isolated; no deadlock on re-entrant dispatch |
| `rf-broadcast` | stable | WebSocket broadcast — room isolation, Subscribed ack, Lagged skip-and-continue |
| `rf-ratelimit` | stable | Per-client IP RateLimitLayer; JSON 429; non-destructive info() peek |
| `rf-i18n` | stable | AcceptLanguage extractor; CLDR plural rules (Slavic/Arabic); Handlebars rendering without leakage |
| `rf-health` | stable | HealthChecker fail-closed by default; Degraded returns 503 |
| `rf-logging` | stable | Real trace/span IDs in log correlation via tracing |
| `rf-metrics` | stable | Unified Prometheus registry including HTTP timings |
| `rf-notifications` | stable | Multi-channel Notifier; aggregates failures instead of aborting on first |
| `rf-async-bridge` | stable | AsyncBridge — critical glue preventing block_on panics inside async runtimes |
| `rf-global-helpers` | stable | Hash (bcrypt/argon2), csrf_token, redirect, event() global helpers |
| `rf-facades` | stable | Consolidated facade re-exports (Auth, Cache, Mail, Storage, Event, DB) |
| `rustforge` | stable | Single-import umbrella crate re-exporting the full stable surface |
| `rf` | stable | Simplified-import umbrella (use rf::Route, rf::Hash, etc.) |
| `forge-cli` | stable | Forge CLI — make:model/controller/request/migration generates compiling code; forge deploy generate |
| `foundry-cli` | stable | Foundry CLI — legacy scaffolding commands; kept for backward compatibility |
| `rf-config` | stable | AppConfig::from_env + `Config` are re-exported in the rf prelude and are part of the 1.0 stable surface (see STABLE_CORE.md); dotenvy-backed. Internal consolidation must not break the exported surface. |
| `rf-routing` | stable | The routing facade the rf prelude exposes (`get/post/put/delete/patch/resource`, `Route`, `global_router`); part of the 1.0 stable surface (see STABLE_CORE.md). Named routes, signed URLs, route groups, resource routing (20 files/6.6k lines). |

---

## Beta Crates (71)

Real implementations with gaps, not fully integration-tested, or API not yet frozen.

| Crate | Tier | Justification |
|-------|------|---------------|
| `rf-inertia` | beta | Full Inertia.js protocol (X-Inertia-Location, SharedProps); not exhaustively load-tested |
| `rf-deploy` | beta | DockerCompose serialization correct; forge deploy generate CLI wired; not prod-battle-tested |
| `rf-env` | beta | dotenvy-backed env loading; real implementation, API surface minimal |
| `rf-graphql` | beta | async-graphql 7.0; per-request auth context injected; not load-tested against large schemas |
| `rf-tenancy` | beta | Real axum Layer + Tenant::current() + isolation helpers + spawn_with_tenant(); not stress-tested |
| `rf-api-resources` | beta | WrappedResource/WrappedCollection; manual Serialize; no silent wrapper drop — API may evolve |
| `rf-requests` | beta | Validated form-request structs (ValidatedJson extractor); real but minimal coverage |
| `rf-collections` | beta | Laravel-style Collection helpers; real map/filter/reduce/chunk impl |
| `rf-authorization` | beta | Gates, policies, RBAC (9 files/2.8k lines); real but not exhaustively integration-tested |
| `rf-pagination` | beta | Real paginator with page/per_page/total metadata |
| `rf-upload` | beta | File upload with size limits; real multipart handling |
| `rf-sse` | beta | Server-Sent Events; real axum SSE integration |
| `rf-2fa` | beta | TOTP/HOTP two-factor auth (real rfc6238 impl); no example in CI |
| `rf-search` | beta | Search engine integration (9 files/2.6k lines); real but backend-dependent |
| `rf-audit` | beta | Audit log (700 lines); real append-only log writer; no integration test |
| `rf-export` | beta | CSV/JSON/Excel export (595 lines); real but no end-to-end test |
| `rf-blade` | beta | Blade-like template engine (17 files/6.7k lines); real Tera-based impl |
| `rf-views` | beta | View layer with layouts/components (12 files/3.2k lines); real Tera-based impl |
| `rf-view` | beta | Tera-based View type with layout support; real but API overlaps rf-blade/rf-views |
| `rf-admin` | beta | Basic admin surface (610 lines); real but not integration-tested |
| `rf-horizon` | beta | Queue monitoring UI (16 files/6.8k lines); real dashboards but not prod-tested |
| `rf-tinker` | beta | Async REPL (7 files/1.4k lines); real rustyline integration |
| `rf-tinker-enhanced` | beta | Enhanced REPL with history and multi-line (8 files/1.4k lines); real |
| `rf-seeder` | beta | Database seeder (208 lines); real SeaORM integration; no CI test |
| `rf-pest` | beta | Parser DSL integration (6 files/1.3k lines); real pest grammars |
| `rf-testing` | beta | Testing utilities (17 files/6.6k lines); real test helpers for requests/responses |
| `rf-scheduling` | beta | Task scheduling with cron expressions (9 files/774 lines); real cron runner |
| `rf-scheduler` | beta | Cron scheduler with fluent API (2 files/672 lines); overlaps rf-scheduling — to be unified |
| `rf-passport` | beta | Laravel Passport OAuth (28 files/4.7k lines); real impl but requires live DB |
| `rf-encryption` | beta | AES-256-GCM encrypt/decrypt (3 files/475 lines); real aes-gcm impl |
| `rf-socialite` | beta | OAuth2 social login — GitHub/Google/Facebook/Twitter (15 files/1.8k lines); real |
| `rf-oauth` | beta | OAuth2 client (5 files/864 lines); real but overlaps rf-socialite |
| `rf-oauth-server` | beta | OAuth2 authorization server (12 files/3.9k lines); real but not integration-tested |
| `rf-oauth2-server` | beta | OAuth2 server alternative (8 files/1.2k lines); overlaps rf-oauth-server — to be unified |
| `rf-errors` | beta | RFC 7807 + dev/prod error display + Sentry integration (9 files/3.3k lines) |
| `rf-forms` | beta | Form validation/rendering (8 files/2.7k lines); real Tera-based forms |
| `rf-http-client` | beta | Reqwest-based HTTP client (6 files/828 lines); real impl |
| `rf-helpers` | beta | String/array/number helpers (5 files/1.2k lines); real |
| `rf-scaffold` | beta | Code scaffolding generator (5 files/2.3k lines); real but separate from forge-cli |
| `rf-cli-gen` | beta | CLI code gen (550 lines); real but contains a few TODO stubs for edge cases |
| `rf-feature-flags` | beta | Feature flag management (497 lines); real in-memory + env-based flags |
| `rf-cashier` | beta | Stripe billing (14 files/1.4k lines); real stripe crate usage; requires live Stripe account |
| `rf-ai` | beta | AI/LLM integration — Anthropic Messages API over reqwest; MockChatProvider for tests |
| `rf-vector` | beta | Vector search (5 files/1.2k lines); real but backend-dependent |
| `rf-soft-deletes` | beta | Soft-delete support (4 files/241 lines); real SeaORM integration |
| `rf-resources` | beta | Resource transformer layer (8 files/1.3k lines); real |
| `rf-auth-scaffolding` | beta | Auth scaffolding generator (14 files/2.7k lines); real code gen |
| `rf-maintenance` | beta | Maintenance mode (5 files/776 lines); real axum middleware |
| `rf-assets` | beta | Asset publishing with content-hash and manifest (5 files/633 lines) |
| `rf-package-dev` | beta | Package development tools (4 files/814 lines); real |
| `rf-providers` | beta | Service provider pattern (3 files/453 lines); real |
| `rf-plugins` | beta | Plugin system (2 files/463 lines); real but undocumented |
| `rf-application` | beta | DDD application layer (50 files/16.3k lines); real RustForge Core architecture layer |
| `rf-domain` | beta | DDD domain value objects/descriptors (2 files/369 lines); real but minimal |
| `rf-infra` | beta | DDD infrastructure layer (16 files/2.9k lines); real adapters for cache/db/queue/storage |
| `rf-api` | beta | DDD API layer (31 files/6.9k lines); real HTTP/artisan/event integration |
| `rf-interactive` | beta | CLI interactive prompts (3 files/391 lines); real dialoguer integration |
| `rf-console` | beta | Console output formatting (9 files/1.3k lines); real colored output |
| `rf-service-container` | beta | DI container with singleton/scoped/transient (13 files/1.7k lines) |
| `rf-container` | beta | DI container v2 with AutoResolver (6 files/1.95k lines); overlaps rf-service-container |
| `rf-observability` | beta | Observability aggregation (8 files/1.9k lines); real tracing + metrics wiring |
| `rf-command-executor` | beta | Command execution runner (7 files/1.1k lines); real |
| `rf-command-events` | beta | Command event bus (6 files/918 lines); real |
| `rf-command-pipeline` | beta | Command pipeline builder (5 files/676 lines); real |
| `rf-signal-handler` | beta | Signal handling + graceful shutdown (6 files/975 lines); real tokio signal |
| `rf-verbosity` | beta | CLI verbosity levels -v/-vv/-vvv (3 files/336 lines); real |
| `rf-stub-system` | beta | Stub/template system for code gen (7 files/1.1k lines); real |
| `rf-advanced-input` | beta | Advanced CLI input (5 files/785 lines); real |
| `rf-nightwatch` | beta | Monitoring and alerting (10 files/1.5k lines); real |
| `rf-dusk` | beta | Browser testing via WebDriver (8 files/1.9k lines); real fantoccini integration |
| `rf-echo` | beta | WebSocket channels a la Laravel Echo (6 files/1.6k lines); real but untested in CI |
| `rf-envoy` | beta | Deployment task runner (7 files/1.8k lines); real SSH-based deployment |
| `rf-sail` | beta | Docker local dev environment (5 files/1.5k lines); real Docker Compose generation |
| `rf-spark` | beta | App platform scaffolding (6 files/2k lines); real but not integration-tested |
| `rf-mcp` | beta | Model Context Protocol server (8 files/1.5k lines); real MCP wire protocol |
| `rf-broadcasting` | beta | Broadcasting layer alternative (6 files/1.2k lines); overlaps rf-broadcast — to be unified |

---

## Experimental Crates (8)

Excluded from `default-members`; not part of the 1.0 supported surface; no SemVer guarantees.
Plain `cargo check` skips them; `cargo check --workspace` compiles them to prevent bitrot.

| Crate | Tier | Justification |
|-------|------|---------------|
| `rf-nova` | experimental | Nova admin panel — multi-resource type-erased dispatch unfinished; not production-ready |
| `rf-nova-macros` | experimental | Nova derive macros — #[derive(Resource)] generates broken stubs |
| `rf-swagger` | experimental | OpenAPI/utoipa integration — route-annotation-only (no auto-scan); not load-tested |
| `rf-telescope` | experimental | Debugging dashboard — stub implementation; not stress-tested against real traffic |
| `rf-cms` | experimental | CMS features — media processing/versioning unfinished |
| `rf-breeze` | experimental | Auth scaffolding generator — depends on rf-blade; not integration-tested |
| `rf-vite` | experimental | Vite asset pipeline — dev-tool only; not verified against axum 0.8 handler model |
| `rf-livereload` | experimental | Live reload/HMR — WebSocket watcher not integration-tested |

---

## Stub Crates (0)

No workspace crate is purely a placeholder at this time. The smallest crates
(rf-domain: 369 lines, rf-async-bridge: 222 lines) have real implementations
behind their lib.rs delegation. If a future crate is added as a placeholder
before its implementation is written, it should be tiered `stub` here.

Note: nine facade crates exist under `crates/` but are **not workspace members**
(`rf-auth-facade`, `rf-cache-facade`, `rf-db-facade`, `rf-event-facade`,
`rf-mail-facade`, `rf-passport-facade`, `rf-route-facade`, `rf-sanctum-facade`,
`rf-storage-facade`). These were superseded in Phase 20 when facades were merged
into their main crates. They are unmaintained dead-code directories; the
recommendation is to delete them in a future cleanup pass.

---

## Tier Counts

| Tier | Count |
|------|-------|
| stable | 34 |
| beta | 71 |
| experimental | 8 |
| stub | 0 |
| **Total workspace crates** | **113** |

---

## Reconciliation Notes

The README maturity matrix (top-level `README.md`, section "Feature-maturity
matrix") uses four grades: **Stable**, **Usable**, **Experimental**, **Deferred**.
Those map to this file's tiers as follows:

| README grade | TIERS.md tier |
|---|---|
| Stable | stable |
| Usable | beta |
| Experimental | experimental |
| Deferred | n/a (not a crate; a language/design ceiling) |

If you add or modify a crate, update **both** this file and the README matrix
section. The rule: this file is the authoritative list; the README matrix is a
user-facing summary of the highlighted surfaces (not every crate).
