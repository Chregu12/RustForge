# rf-inertia

Inertia.js adapter for RustForge - Build modern single-page applications using server-side routing and controllers.

## Overview

Inertia.js allows you to create fully client-side rendered, single-page apps, without much of the complexity that comes with modern SPAs. It does this by leveraging existing server-side patterns that you already love.

This crate provides a complete Rust implementation compatible with Laravel's Inertia.js integration.

## Features

- **Automatic page components** - No need to manually wire up routes to components
- **Shared data** - Share data across all pages (e.g., authenticated user, flash messages)
- **Lazy props** - Defer heavy computations until they're actually needed
- **Partial reloads** - Only reload the data you need
- **Asset versioning** - Automatic cache busting
- **Axum integration** - First-class support for Axum web framework

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-inertia = "0.1"
axum = "0.7"
```

## Quick Start

### 1. Setup Middleware

```rust
use axum::{Router, routing::get};
use rf_inertia::{InertiaConfig, InertiaMiddleware};

#[tokio::main]
async fn main() {
    let config = InertiaConfig::new()
        .root_view("app")
        .version("v1.0.0");

    let app = Router::new()
        .route("/", get(index))
        .route("/dashboard", get(dashboard))
        .layer(InertiaMiddleware::layer(config));

    // Start server...
}
```

### 2. Create a Handler

```rust
use rf_inertia::Inertia;
use serde::Serialize;

#[derive(Serialize)]
struct User {
    name: String,
    email: String,
}

async fn index() -> Inertia {
    let user = User {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    Inertia::render("Dashboard/Index")
        .with("user", user)
        .with("title", "Welcome!")
}
```

### 3. Create Your Frontend Component

**Vue 3 Example:**

```vue
<template>
  <div>
    <h1>{{ title }}</h1>
    <p>Welcome, {{ user.name }}!</p>
  </div>
</template>

<script setup>
defineProps({
  user: Object,
  title: String,
})
</script>
```

**React Example:**

```jsx
export default function Index({ user, title }) {
  return (
    <div>
      <h1>{title}</h1>
      <p>Welcome, {user.name}!</p>
    </div>
  );
}
```

## Advanced Usage

### Shared Props

Share data across all pages:

```rust
use rf_inertia::{InertiaConfig, InertiaMiddleware};

let config = InertiaConfig::new();
let middleware = InertiaMiddleware::new(config);

// Add shared props
middleware.shared_props().add("app_name", "RustForge").await;
middleware.shared_props().add("user", current_user()).await;
```

### Lazy Props

Defer expensive computations:

```rust
async fn dashboard() -> Inertia {
    Inertia::render("Dashboard/Index")
        .with("user", get_user())
        .with_lazy("stats", || {
            // This will only be evaluated when explicitly requested
            compute_expensive_stats()
        })
}
```

### Partial Reloads

Only reload specific props on client-side navigation:

```typescript
// Frontend (Vue/React)
router.reload({
  only: ['stats'], // Only reload the 'stats' prop
});
```

### Asset Versioning

Various versioning strategies:

```rust
use rf_inertia::{InertiaConfig, AssetVersion};

// Fixed version
let config = InertiaConfig::new().version("v1.0.0");

// From environment variable
let config = InertiaConfig::new()
    .version_fn(|| std::env::var("APP_VERSION").unwrap_or("1".to_string()));

// From Git commit
let config = InertiaConfig::new()
    .version_fn(|| AssetVersion::from_git_hash().get());

// From file timestamp
let config = InertiaConfig::new()
    .version_fn(|| AssetVersion::from_file("public/mix-manifest.json").get());
```

### Conditional Props

```rust
async fn show_user(user_id: i64, is_admin: bool) -> Inertia {
    let user = get_user(user_id);

    Inertia::render("Users/Show")
        .with("user", user)
        .when(is_admin, "secret_data", get_secret_data())
        .when_some("optional", get_optional_value())
}
```

## Laravel Parity

This implementation aims for 100% feature parity with Laravel's Inertia adapter:

| Feature | Status | Notes |
|---------|--------|-------|
| Basic rendering | ✅ | Full support |
| Props | ✅ | Full support |
| Shared data | ✅ | Full support |
| Lazy props | ✅ | Full support |
| Partial reloads | ✅ | Full support |
| Asset versioning | ✅ | Multiple strategies |
| Validation errors | ✅ | Via flash messages |
| Server-side rendering | 🚧 | Planned |

## Examples

See the [examples](../../examples/inertia-demo) directory for complete working examples with:
- Vue 3 + Vite
- React + Vite
- Svelte + Vite

## License

MIT
