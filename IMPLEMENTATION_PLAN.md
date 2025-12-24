# Implementation Plan: Remaining Laravel Packages

## Status Overview

| Package | Laravel | RustForge | Status |
|---------|---------|-----------|--------|
| Sanctum | laravel/sanctum | rf-sanctum | ✅ Exists |
| Passport | laravel/passport | rf-passport | ✅ Exists |
| Pest | pestphp/pest | rf-pest | ❌ Needs creation |
| Nightwatch | laravel/nightwatch | rf-nightwatch | ❌ Needs creation |
| MCP | laravel/mcp | rf-mcp | ❌ Needs creation |
| Cashier | laravel/cashier-stripe | rf-cashier | ❌ Needs creation |

---

## 1. rf-pest - Testing Framework

**Laravel Equivalent**: pestphp/pest
**Purpose**: Simpler, expressive testing syntax

### Target Syntax

```rust
use rf::Pest;

// Describe/it style
Pest::describe("User Registration", |ctx| {
    ctx.it("should create a new user", || {
        let user = User::factory().create()?;
        expect(&user.email).to_contain("@");
        expect(&user.id).to_be_greater_than(0);
    });

    ctx.it("should hash the password", || {
        let user = User::factory().create()?;
        expect(&user.password).not().to_equal("password123");
    });
});

// Fluent test() style
Pest::test("users can login", || {
    let user = User::factory().create()?;

    expect(Auth::attempt(json!({
        "email": user.email,
        "password": "password"
    }))).to_be_true();
});

// Expect API
expect(&value).to_equal(expected);
expect(&value).to_be_true();
expect(&value).to_be_false();
expect(&value).to_be_none();
expect(&value).to_be_some();
expect(&value).to_contain("substring");
expect(&list).to_have_count(5);
expect(&result).to_be_ok();
expect(&result).to_be_err();
```

### Dependencies
- rf-testing (existing - build on top)
- tokio (async support)
- serde_json
- colored (output formatting)

### Files to Create
```
crates/rf-pest/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── describe.rs      # describe/it blocks
    ├── test.rs          # test() function
    ├── expect.rs        # Expectation API
    ├── runner.rs        # Test runner
    ├── output.rs        # Pretty output
    └── macros.rs        # test!, describe!, it!, expect!
```

---

## 2. rf-nightwatch - Application Monitoring

**Laravel Equivalent**: laravel/nightwatch
**Purpose**: Production monitoring for RustForge apps

### Target Syntax

```rust
use rf::Nightwatch;

// Initialize
Nightwatch::init(NightwatchConfig {
    api_key: env!("NIGHTWATCH_API_KEY"),
    environment: "production",
    ..Default::default()
});

// Automatic tracking (via middleware)
Route::middleware(&["nightwatch"]).group(|| {
    Route::get("/api/*", handler);
});

// Manual tracking
Nightwatch::track_query("SELECT * FROM users", Duration::from_millis(50));
Nightwatch::track_job("SendEmail", JobStatus::Completed);
Nightwatch::track_error(&error);
Nightwatch::track_metric("api.requests", 1);

// Dashboard access
Route::get("/nightwatch", Nightwatch::dashboard());
```

### Features
- Request/Response logging
- Database query tracking
- Job monitoring
- Error tracking
- Performance metrics
- Real-time dashboard
- Alerts

### Dependencies
- rf-telescope (similar, can reuse)
- rf-metrics
- axum (middleware)
- tokio
- serde

### Files to Create
```
crates/rf-nightwatch/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── config.rs
    ├── collector.rs     # Data collection
    ├── storage.rs       # Store metrics
    ├── middleware.rs    # Axum middleware
    ├── dashboard.rs     # Web dashboard
    ├── alerts.rs        # Alert system
    ├── api.rs           # API endpoints
    └── facade.rs        # Nightwatch facade
```

---

## 3. rf-mcp - Model Context Protocol

**Laravel Equivalent**: laravel/mcp
**Purpose**: AI integration (Claude, GPT, etc.)

### Target Syntax

```rust
use rf::MCP;

// Define MCP tools
MCP::tool("get_user", |params| async {
    let user_id: i64 = params.get("user_id")?;
    let user = User::find(user_id).await?;
    Ok(json!(user))
})
.description("Get user by ID")
.parameter("user_id", "integer", "The user ID", true);

MCP::tool("search_posts", |params| async {
    let query: String = params.get("query")?;
    let posts = Post::search(&query).limit(10).get().await?;
    Ok(json!(posts))
})
.description("Search blog posts")
.parameter("query", "string", "Search query", true);

// Resources (read-only data)
MCP::resource("users", || async {
    User::all().await
});

// Start MCP server
MCP::serve(MCPConfig {
    transport: Transport::Stdio,  // or Http, WebSocket
    ..Default::default()
}).await;
```

### Features
- Tool registration
- Resource exposure
- Parameter validation
- stdio/HTTP/WebSocket transport
- OpenAI/Anthropic compatible
- Rate limiting
- Authentication

### Dependencies
- serde_json
- tokio
- axum (for HTTP transport)
- async-trait

### Files to Create
```
crates/rf-mcp/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── tool.rs          # Tool definitions
    ├── resource.rs      # Resources
    ├── params.rs        # Parameter handling
    ├── transport/
    │   ├── mod.rs
    │   ├── stdio.rs     # stdio transport
    │   ├── http.rs      # HTTP transport
    │   └── websocket.rs # WebSocket transport
    ├── server.rs        # MCP server
    ├── protocol.rs      # MCP protocol
    └── facade.rs        # MCP facade
```

---

## 4. rf-cashier - Stripe Integration

**Laravel Equivalent**: laravel/cashier-stripe
**Purpose**: Subscription billing with Stripe

### Target Syntax

```rust
use rf::Cashier;

// Make User billable
#[derive(Model, Billable)]
struct User {
    id: i64,
    email: String,
    stripe_id: Option<String>,
    pm_type: Option<String>,
    pm_last_four: Option<String>,
    trial_ends_at: Option<DateTime>,
}

// Create subscription
let subscription = user.new_subscription("default", "price_xxx")
    .create(payment_method)?;

// Check subscription
if user.subscribed("default") {
    // Premium features
}

// Change plan
user.subscription("default")
    .swap("price_yyy")?;

// Cancel
user.subscription("default")
    .cancel()?;

// Resume
user.subscription("default")
    .resume()?;

// One-time charge
user.charge(1000, payment_method)?;

// Invoice
let invoice = user.invoice(1000, "Consulting fee")?;

// Checkout session
let session = user.checkout("price_xxx", [
    "success_url" => "http://example.com/success",
    "cancel_url" => "http://example.com/cancel",
])?;

// Customer portal
let portal_url = user.billing_portal_url("http://example.com/dashboard")?;

// Stripe webhook handling
Route::post("/stripe/webhook", Cashier::webhook_handler());
```

### Features
- Subscription management
- One-time charges
- Invoicing
- Payment methods
- Customer portal
- Webhook handling
- Trial periods
- Coupons/Promotions

### Dependencies
- stripe-rust (Stripe API client)
- chrono
- serde
- tokio
- sea-orm (models)

### Files to Create
```
crates/rf-cashier/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── billable.rs      # Billable trait
    ├── subscription.rs  # Subscription management
    ├── payment.rs       # Payment methods
    ├── invoice.rs       # Invoicing
    ├── checkout.rs      # Checkout sessions
    ├── portal.rs        # Customer portal
    ├── webhook.rs       # Webhook handler
    ├── models/
    │   ├── mod.rs
    │   ├── subscription.rs
    │   └── subscription_item.rs
    ├── config.rs
    └── facade.rs        # Cashier facade
```

---

## Implementation Order

1. **rf-pest** (2-3 hours)
   - Build on rf-testing
   - Add describe/it/expect syntax
   - Already have test infrastructure

2. **rf-cashier** (4-5 hours)
   - Most business-critical
   - Stripe API is well-documented
   - stripe-rust crate exists

3. **rf-mcp** (3-4 hours)
   - New but simple protocol
   - High value for AI integration
   - Relatively straightforward

4. **rf-nightwatch** (3-4 hours)
   - Can reuse rf-telescope patterns
   - Dashboard from rf-horizon
   - Mostly collection + UI

---

## rf Crate Updates

After implementing, add to `rf/src/lib.rs`:

```rust
// Direct exports
pub use rf_pest::Pest;
pub use rf_nightwatch::Nightwatch;
pub use rf_mcp::MCP;
pub use rf_cashier::Cashier;

// In prelude
pub mod prelude {
    pub use rf_pest::{Pest, expect, test, describe};
    pub use rf_nightwatch::Nightwatch;
    pub use rf_mcp::MCP;
    pub use rf_cashier::{Cashier, Billable};
}

// In services module
pub mod services {
    pub mod cashier {
        pub use rf_cashier::*;
    }

    pub mod mcp {
        pub use rf_mcp::*;
    }

    pub mod monitoring {
        pub use rf_nightwatch::*;
    }
}
```

---

## Cargo.toml Dependencies

### rf-pest
```toml
[dependencies]
rf-testing = { path = "../rf-testing" }
tokio = { version = "1.37", features = ["macros", "rt-multi-thread"] }
colored = "2.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### rf-cashier
```toml
[dependencies]
stripe-rust = "24.0"  # or async-stripe
tokio = { version = "1.37", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
sea-orm = { version = "1.1", features = ["runtime-tokio-rustls"] }
thiserror = "1.0"
async-trait = "0.1"
```

### rf-mcp
```toml
[dependencies]
tokio = { version = "1.37", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
axum = { version = "0.8", features = ["ws"] }
async-trait = "0.1"
thiserror = "1.0"
uuid = { version = "1.10", features = ["v4"] }
```

### rf-nightwatch
```toml
[dependencies]
rf-telescope = { path = "../rf-telescope" }
rf-metrics = { path = "../rf-metrics" }
tokio = { version = "1.37", features = ["full"] }
axum = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
dashmap = "5.5"
```

---

## Timeline

| Package | Priority | Complexity | Dependencies |
|---------|----------|------------|--------------|
| rf-pest | High | Low | rf-testing |
| rf-cashier | High | Medium | stripe-rust, sea-orm |
| rf-mcp | Medium | Medium | axum, tokio |
| rf-nightwatch | Low | Medium | rf-telescope, rf-metrics |

**Recommended order**: rf-pest → rf-cashier → rf-mcp → rf-nightwatch
