# Additional / Extended Packages

This page documents four extended crates that ship in the RustForge monorepo but are
**not part of the v1 stable surface**. All four are classified **[beta]** in
[docs/TIERS.md](../../TIERS.md), which means they have real implementations with
documented gaps or missing integration tests, and their APIs may shift in minor
versions without a SemVer guarantee.

> **None of these crates are covered by the v1 API contract.**
> If you need a stability guarantee, use only the 34 stable crates listed in
> [docs/STABLE_CORE.md](../../STABLE_CORE.md) and described in [Features.md](Features.md).

---

## Monorepo consumption

RustForge is **not published to crates.io**. Add these crates as git or path dependencies:

```toml
# Git dependency — pin to a release tag (recommended)
rf-cashier   = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
rf-mcp       = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
rf-nightwatch = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
rf-pest      = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }

# Path dependency — if you have the repo cloned locally
rf-cashier   = { path = "../RustForge/crates/rf-cashier" }
```

Do **not** write `rf-cashier = "0.1"` — that is a crates.io form and will fail to resolve.

See [docs/RELEASING.md](../../RELEASING.md) for the full downstream consumption guide.

---

## Maturity legend

| Tag | Meaning |
|-----|---------|
| **[beta]** | Real implementation; documented gaps or not exhaustively integration-tested. API may shift in minor versions. No SemVer guarantee. |

---

## rf-pest — BDD-style test runner

**Tier: [beta]** — `crates/rf-pest` — 6 files / ~1.3k lines

A Pest PHP-inspired testing DSL layered on top of `rf-testing`. Provides
`test()` / `test_async()` free functions, `describe()` / `it()` BDD blocks, and
a fluent `expect()` assertion API. Unit tests in the crate itself pass under
`cargo test`.

**What actually works:**

- `expect(&value).to_equal(&expected)` and `.not().to_equal(...)` negation
- Boolean checks: `.to_be_true()`, `.to_be_false()`
- Option checks: `.to_be_some()`, `.to_be_none()`
- Result checks: `.to_be_ok()`, `.to_be_err()`
- String checks: `.to_contain()`, `.to_start_with()`, `.to_end_with()`, `.to_match(regex)`
- Collection checks: `.to_have_count()`, `.to_contain_item()`, `.to_be_empty()`
- Numeric checks: `.to_be_greater_than()`, `.to_be_less_than()`, `.to_be_between()`
- `describe("Suite", |ctx| { ctx.it("case", || { ... }); })`
- `TestRunner` collects and runs registered tests with colored output

**Limitations / gaps:**

- No integration with `cargo test`'s native harness; `TestRunner::run()` is a
  standalone runner that must be called from a `main` function or a single
  `#[test]` entry point.
- No CI probe covering async test paths against real RustForge models.
- Name is misleading: this crate has nothing to do with the `pest` PEG parser
  crate — it is a testing DSL inspired by Pest PHP's expressive test style.

**Usage:**

```toml
[dev-dependencies]
rf-pest = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
```

```rust
use rf_pest::prelude::*;

#[test]
fn suite() {
    expect(&(2 + 2)).to_equal(&4);
    expect(&"Hello World".to_string()).to_contain("World");
    expect(&Some(42)).to_be_some();

    describe("arithmetic", |ctx| {
        ctx.it("adds correctly", || {
            expect(&(1 + 1)).to_equal(&2);
        });
    });
}
```

---

## rf-cashier — Stripe subscription billing

**Tier: [beta]** — `crates/rf-cashier` — 14 files / ~1.4k lines

A Laravel Cashier-inspired billing layer wrapping the `async-stripe` crate.
Provides subscription management, one-time charges, checkout sessions, a
customer portal redirect, invoice retrieval, and Stripe webhook verification.

**What actually works:**

- `Billable` trait and `BillableExt` extension methods
- `SubscriptionBuilder` for creating new subscriptions via Stripe API
- `Subscription` struct with `SubscriptionStatus` mapped from Stripe's own enum
- `CheckoutBuilder` / `CheckoutSession` for Stripe Checkout
- `PortalSession` for Stripe Billing Portal redirects
- `Invoice` / `InvoiceBuilder` for retrieving customer invoices
- `PaymentMethod` / `PaymentMethodBuilder`
- `webhook_handler` axum handler that verifies the `Stripe-Signature` header
  via HMAC-SHA256 before passing the `WebhookPayload` to your code
- `Cashier::webhook()` returns a ready-made `MethodRouter` to mount at
  `/stripe/webhook`

**Limitations / gaps:**

- Requires a live Stripe account and `STRIPE_SECRET_KEY` / `STRIPE_WEBHOOK_SECRET`
  environment variables. There is no mock Stripe client for unit tests.
- No integration test in CI; billing flows are only exercised manually.
- The `Billable` derive macro is documented in lib.rs but is not a proc-macro —
  you must implement the trait manually or call the builder methods directly.
- SeaORM database models for subscriptions/invoices are defined but migrations
  are not bundled; you must write and run them yourself.

**Usage:**

```toml
[dependencies]
rf-cashier = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
```

```rust
use rf_cashier::{Cashier, CashierConfig};
use axum::Router;

// In your app startup
Cashier::configure(CashierConfig {
    stripe_secret: std::env::var("STRIPE_SECRET_KEY").unwrap(),
    webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").unwrap(),
    ..Default::default()
});

// Mount the webhook endpoint
let app = Router::new()
    .route("/stripe/webhook", Cashier::webhook());
```

---

## rf-mcp — Model Context Protocol server

**Tier: [beta]** — `crates/rf-mcp` — 8 files / ~1.5k lines

Exposes a [Model Context Protocol](https://modelcontextprotocol.io/) server that
AI assistants (Claude, etc.) can connect to in order to call your application's
tools, read resources, and use prompt templates. Transport is stdio (JSON-RPC over
stdin/stdout), which is the standard MCP transport for local tool servers.

**What actually works:**

- `ToolRegistry` — register async handlers that accept `serde_json::Value` and
  return `McpResult<serde_json::Value>`
- `ResourceRegistry` — register URI-template handlers that serve text or binary
  content
- `PromptRegistry` — register named prompt templates with argument interpolation
- `McpServer` — dispatches `initialize`, `tools/list`, `tools/call`,
  `resources/list`, `resources/read`, `prompts/list`, `prompts/get` JSON-RPC
  methods per the MCP spec
- `StdioTransport` — reads newline-delimited JSON from stdin, writes to stdout
- `Mcp` facade with `Mcp::tool()`, `Mcp::resource()`, `Mcp::prompt()`,
  `Mcp::serve().await`

**Limitations / gaps:**

- Only stdio transport is implemented; HTTP/SSE transport (needed for remote
  servers) is not present.
- No integration test in CI against a real MCP client; conformance with the
  latest MCP spec revision is unverified.
- `Mcp::tool()` in the facade signature differs from the lib.rs doc-comment
  example (which showed a closure argument) — the real API uses a builder
  chain: `Mcp::tool("name").description("...").handler(|input| async { ... })`.

**Usage:**

```toml
[dependencies]
rf-mcp = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
```

```rust
use rf_mcp::{Mcp, McpResult};

#[tokio::main]
async fn main() -> McpResult<()> {
    // Register a tool
    Mcp::tool("greet")
        .description("Return a greeting")
        .handler(|input| {
            Box::pin(async move {
                let name = input["name"].as_str().unwrap_or("World");
                Ok(serde_json::json!({ "greeting": format!("Hello, {name}!") }))
            })
        })
        .register();

    // Start stdio MCP server (blocks until stdin closes)
    Mcp::serve().await
}
```

---

## rf-nightwatch — Application monitoring and alerting

**Tier: [beta]** — `crates/rf-nightwatch` — 10 files / ~1.5k lines

A Laravel-Nightwatch-inspired monitoring layer. Provides health checks, metrics
recording (counter / gauge / histogram backed by the `metrics` crate), alert
rules, an event recorder, and an axum router that exposes a `/health` JSON
endpoint and a basic dashboard.

**What actually works:**

- `CheckRegistry` — register async health checks; `Nightwatch::run_checks().await`
  returns `Vec<(String, CheckResult)>`; `Nightwatch::is_healthy().await` returns
  `bool`
- `CheckResult::pass()`, `CheckResult::warn()`, `CheckResult::fail()` constructors
- `MetricsRegistry` — `Nightwatch::counter("name").increment()`,
  `Nightwatch::gauge("name").set(f64)`,
  `Nightwatch::histogram("name").record(f64)`
- `AlertRegistry` — `Nightwatch::alert("name")` returns an `AlertBuilder` for
  condition-based alerting
- `Monitor` / `Recorder` for event recording
- `nightwatch_routes()` returns an `axum::Router` you can nest into your app
- `Nightwatch::serve("0.0.0.0:9090").await` runs a standalone monitoring server

**Limitations / gaps:**

- Alert notification delivery (email, Slack, etc.) is defined in the type
  structure but no transport backend is wired in; calling `.notify()` records
  the alert rule but does not actually send a notification.
- Dashboard HTML is minimal; no live auto-refresh.
- Metrics are recorded via the `metrics` crate facade but there is no Prometheus
  exporter endpoint bundled — use `rf-metrics` (stable) for that.
- No CI integration test; all paths require a running service to exercise.

**Usage:**

```toml
[dependencies]
rf-nightwatch = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
```

```rust
use rf_nightwatch::{Nightwatch, CheckResult};
use axum::Router;

// Register checks
Nightwatch::check("database", || async {
    // replace with real DB ping
    CheckResult::pass("Connected")
});

Nightwatch::check("cache", || async {
    CheckResult::pass("Redis available")
});

// Embed in your existing app
let app = Router::new()
    .nest("/nightwatch", Nightwatch::router())
    // ... your other routes
    ;

// — or run a dedicated monitoring server —
// Nightwatch::serve("0.0.0.0:9090").await.unwrap();
```

---

## Summary table

| Crate | Tier | Files | What works | Key gap |
|-------|------|-------|------------|---------|
| `rf-pest` | **beta** | 6 / ~1.3k lines | Fluent `expect()` assertions, `describe`/`it` BDD blocks, async test runner | No `cargo test` harness integration; name unrelated to `pest` PEG parser |
| `rf-cashier` | **beta** | 14 / ~1.4k lines | Stripe subscriptions, checkout, webhooks, portal via `async-stripe` | Requires live Stripe; no mock; no bundled DB migrations |
| `rf-mcp` | **beta** | 8 / ~1.5k lines | MCP stdio server — tools, resources, prompts; JSON-RPC dispatch | stdio only; HTTP/SSE transport missing; no CI conformance test |
| `rf-nightwatch` | **beta** | 10 / ~1.5k lines | Health checks, counter/gauge/histogram metrics, alert rules, axum router | Alert notifications not wired; no Prometheus exporter; no CI test |

---

## Related pages

- [Features.md](Features.md) — full feature inventory including the 34 stable crates
- [docs/TIERS.md](../../TIERS.md) — authoritative maturity tier for every workspace crate
- [docs/RELEASING.md](../../RELEASING.md) — how to add RustForge as a git/path dependency
- [Installation.md](Installation.md) — getting started with the core framework
