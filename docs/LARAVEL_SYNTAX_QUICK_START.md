# Laravel-Style Helpers — Quick Start

A 5-minute tour of the Laravel-inspired helpers that are fully functional today.
For a complete working application guide, see [GETTING_STARTED.md](./GETTING_STARTED.md).

---

## What works now

| Helper | API | Notes |
|--------|-----|-------|
| Password hashing | `Hash::make(pw)` / `Hash::check(pw, hash)` | bcrypt |
| CSRF tokens | `csrf_token()` | UUID v4 |
| Validation rules | `rules! { field: rule \| rule }` | pipe syntax |
| Typed validation DSL | `validate! { field: type.rule }` | preferred over rules! |

---

## Password hashing

```rust
use rf::Hash;

let hash = Hash::make("my_password");

if Hash::check("my_password", &hash) {
    println!("Password is correct");
}
```

## CSRF token

```rust
use rf::csrf_token;

let token = csrf_token(); // unique UUID v4 string per call
```

## Validation

### Pipe-syntax rules! (functional, but has a caveat)

```rust
use rf_macros::rules;

let rules = rules! {
    email:    required | email,
    password: required | min(8) | max(72),
};
```

**Caveat:** `min(8)` on a string field validates the numeric *value* (>=8) not the
string *length*. For length validation use the typed `validate!` DSL.

### Typed validate! DSL (preferred)

```rust
use rf::prelude::*;

// string.max = max length; int.min = min value — no ambiguity
if validate! { title: string.max(100), age: int.min(18), email: email }.is_err() {
    return json(serde_json::json!({"error": "validation failed"}));
}
```

---

## Route facade — metadata only

`Route::get("/path", "Controller@method")` registers route metadata but does NOT serve
HTTP traffic. Use it for tooling/inspection only.

For real HTTP handling, use the `rf::prelude::*` routing functions:

```rust
use rf::prelude::*;

get("/users",     list_users);
post("/users",    create_user);
put("/users/{id}", update_user);

let router = rf::global_router()
    .build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request));
```

---

## Try it

Run the standalone demonstration:

```bash
cargo run --bin simple   # in examples/laravel-syntax-simple/
```

---

## Learn more

- [LARAVEL_SYNTAX.md](./LARAVEL_SYNTAX.md) — full feature reference and status table
- [GETTING_STARTED.md](./GETTING_STARTED.md) — complete working application guide
- [EXAMPLES.md](./EXAMPLES.md) — runnable, CI-tested example gallery
