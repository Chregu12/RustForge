# Laravel-Inspired Syntax in RustForge

RustForge borrows vocabulary and conventions from Laravel. This document describes the
features that exist under this inspiration, their actual implementation status, and the
**recommended path** for building real applications (which differs from the string-based
Route facade described in older examples).

For the production-ready API, read [GETTING_STARTED.md](./GETTING_STARTED.md) first.

---

## Table of Contents

1. [What actually works](#what-actually-works)
2. [Route Facade — metadata only, non-serving](#route-facade)
3. [Validation rules! macro](#validation-rules-macro)
4. [Global helpers](#global-helpers)
5. [Recommended path for real apps](#recommended-path)

---

## What actually works

| Feature | Status | Notes |
|---------|--------|-------|
| `Hash::make(password)` | Stable | bcrypt via rf-auth |
| `Hash::check(password, hash)` | Stable | bcrypt comparison |
| `csrf_token()` | Stable | UUID-based token helper |
| `rules! { field: rule | rule }` | Stable | Pipe-syntax validation; see caveats below |
| `rf::prelude::*` import | Stable | The real one-import API for handlers |
| `validate! { field: type.rule }` | Stable | Typed DSL (preferred over rules!) |

---

## Route Facade

`Route::get("/path", "Controller@method")` compiles and registers route metadata, but
**does not serve HTTP traffic**. `GlobalRouter::build_router()` returns an empty
`Router::new()` — the handler string is discarded at registration time.

This API is useful only for listing registered routes (e.g., for tooling that inspects
route metadata). It cannot replace real axum routing.

**Do not use the Route string facade for production applications.** See the
[Recommended path](#recommended-path) section below.

```rust
use rf::prelude::*;

// This registers metadata but does NOT serve requests:
// Route::get("/users", "UserController@index");   // non-functional

// This actually serves requests:
get("/users", list_users);   // real axum routing via rf prelude
let router = rf::global_router().build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request));
```

Named routes, URL generation, and route signing ARE functional:

```rust
// Named routes and URL generation work correctly
Route::get("/dashboard", "DashboardController@index")
    .name("dashboard");

Route::post("/login", "AuthController@login")
    .name("auth.login");
```

---

## Validation rules! macro

The `rules!` macro provides Laravel-style pipe syntax for building validation rule sets.
It is real and functional.

```rust
use rf_macros::rules;

let validation_rules = rules! {
    email: required | email,
    password: required | min(8) | max(72),
    username: required | min(3) | max(20),
    age: integer | between(18, 120),
};
```

**Caveat — numeric vs. length ambiguity:** `min(8)` on a string field validates the
numeric *value* (>=8), not the string length. To validate string length, use the typed
`validate!` DSL instead:

```rust
// Preferred: typed DSL (string.max means max length, int.min means min value)
if validate! { title: string.max(100), age: int.min(18), email: email }.is_err() {
    return json(serde_json::json!({"error": "validation failed"}));
}
```

The `validate!` DSL is available via `use rf::prelude::*` and resolves the
length-vs-numeric ambiguity by requiring a type prefix.

---

## Global Helpers

### Hash

```rust
use rf::Hash;

let hash = Hash::make("my_password");         // bcrypt hash
let ok   = Hash::check("my_password", &hash); // true
```

### CSRF Token

```rust
use rf::csrf_token;

let token = csrf_token(); // UUID v4 string, unique per call
```

### Translation placeholder

```rust
use rf::__;

let msg = __("auth.failed"); // Returns the key as-is (placeholder implementation)
```

`rf-i18n` provides a real localization implementation with CLDR plural rules and
Handlebars templating. See [EXAMPLES.md](./EXAMPLES.md) for the `i18n-localized-api`
example.

---

## Recommended path

For a real working application, use the `rf::prelude::*` API documented in
[GETTING_STARTED.md](./GETTING_STARTED.md):

```rust
use rf::prelude::*;

// Declare a model (real SQLite/Postgres/MySQL via SeaORM)
Model!(Post: title, body);

// Argument-less handler — reads request through ambient globals
async fn create_post() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(100), body: string }.is_err() {
        return json(serde_json::json!({"error": "validation failed"}));
    }
    let title: String = input("title").unwrap_or_default();
    let body:  String = input("body").unwrap_or_default();
    match create!(Post, title = title, body = body) {
        Ok(row) => json(row),
        Err(e)  => json(serde_json::json!({"error": e.to_string()})),
    }
}

// Wire real routes
fn build_app() -> axum::Router {
    get("/posts", list_posts);
    post("/posts", create_post);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}
```

This is the design verified through 8 production-loop rounds and reflected in the
feature-maturity matrix in [README.md](../README.md).

---

## See Also

- [Getting Started](./GETTING_STARTED.md) — the real working API
- [Example Gallery](./EXAMPLES.md) — runnable, CI-tested examples
- [README.md maturity matrix](../README.md#feature-maturity-matrix) — graded status of every surface
- `examples/laravel-syntax-simple/` — demonstrates Hash, csrf_token, and rules! in isolation
