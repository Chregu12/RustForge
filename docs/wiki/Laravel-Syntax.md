# Laravel-Inspired Syntax in RustForge

RustForge borrows vocabulary and conventions from Laravel. This page describes the
helpers that work today, what the Route facade actually does (and does not do), and
where to find the real working application API.

For a full working application guide, see the top-level
[GETTING_STARTED.md](../GETTING_STARTED.md) and the
[README maturity matrix](../../README.md#feature-maturity-matrix).

---

## What is functional today

| Feature | API | Grade |
|---------|-----|-------|
| Password hashing | `Hash::make(pw)` / `Hash::check(pw, hash)` | Stable |
| CSRF token generation | `csrf_token()` | Stable |
| Pipe-syntax validation | `rules! { field: required \| email }` | Stable (see caveat) |
| Typed validation DSL | `validate! { field: type.rule }` | Stable (preferred) |
| Route metadata | `Route::get("/path", "Controller@method")` | Metadata only |
| Named routes / URL gen | `Route::get(..).name("x")` | Stable |
| Full application API | `use rf::prelude::*;` + `Model!`, `create!`, `validate!` | Stable |

---

## Password hashing

```rust
use rf::Hash;

let hash = Hash::make("password123");
let ok   = Hash::check("password123", &hash); // true
```

Bcrypt with configurable cost factor. Argon2 is also available via `rf-auth`.

---

## CSRF token

```rust
use rf::csrf_token;

let token = csrf_token(); // UUID v4, unique per call
```

---

## Validation

### Pipe-syntax rules! macro

```rust
use rf_macros::rules;

let rules = rules! {
    email:    required | email,
    password: required | min(8) | max(72),
    age:      integer  | between(18, 120),
};
```

**Caveat:** `min(8)` on a `String` field validates the numeric value (>= 8), not the
string length. Use the typed `validate!` DSL to avoid this ambiguity.

### Typed validate! DSL (recommended)

```rust
use rf::prelude::*;

// type prefix resolves ambiguity: string.max = length, int.min = value
if validate! { title: string.max(100), age: int.min(18), email: email }.is_err() {
    return json(serde_json::json!({"error": "validation failed"}));
}
```

---

## Route facade — metadata only, not serving

`Route::get("/path", "Controller@method")` registers route metadata but does **not**
serve HTTP traffic. `GlobalRouter::build_router()` returns an empty `Router::new()` —
the handler string is discarded at registration time.

Use the Route facade for tooling that inspects route metadata (e.g., listing registered
routes). For actual HTTP traffic, use the `rf::prelude::*` routing functions:

```rust
use rf::prelude::*;

get("/users",      list_users);
post("/users",     create_user);
put("/users/{id}", update_user);
delete("/users/{id}", delete_user);

let router = rf::global_router()
    .build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request));
```

---

## Complete working example

The `examples/laravel-syntax-simple` crate demonstrates Hash, csrf_token, and rules!
in isolation:

```bash
cargo run --bin simple
```

For a full REST API with real database persistence, run:

```bash
cargo run -p blog-slice     # port 3000 — the canonical reference slice
cargo run -p rest-crud-resource  # port 3001 — five-verb CRUD + eager relations
```

---

## What is not yet functional

The following syntax forms appear in older documentation but are **not implemented**:

- `rustforge! { ... }` block
- `laravel! { class User extends Model { ... } }` syntax
- `#[auto_await]` macro
- `blade! { @if ... }` template macro
- `mailable! { ... }` class macro
- `form_request! { ... }` macro
- `function!(request: Request) { ... }` route closure macro
- `Route::resource("posts", "PostController")` expansion

Do not use these forms in production code. The working API is `use rf::prelude::*;`
as documented in [GETTING_STARTED.md](../GETTING_STARTED.md).

---

## See also

- [Quick Start](../LARAVEL_SYNTAX_QUICK_START.md) — 5-minute tour of functional helpers
- [Full reference](../LARAVEL_SYNTAX.md) — status table and real usage
- [Getting Started](../GETTING_STARTED.md) — complete working application guide
- [Example Gallery](../EXAMPLES.md) — CI-tested runnable examples
- [README maturity matrix](../../README.md#feature-maturity-matrix) — graded status

---

*This page reflects the verified state after 8 production-loop rounds documented in
VISION\_GAP.md. Last updated 2026-07.*
