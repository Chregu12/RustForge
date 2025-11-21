# Migration Guide: Restructuring to Laravel-Style Organization

This guide helps you migrate existing RustForge projects to the new Laravel-style structure.

## Overview of Changes

RustForge has been restructured to match Laravel's two-repository pattern:

### Before (v0.x)
```
rust-dx-framework/
├── crates/          # Framework crates
├── examples/        # Mixed with app code
├── src/            # Mixed structure
└── ...
```

### After (v1.0)
```
rust-dx-framework/
├── crates/              # Framework code (like laravel/framework)
├── rustforge-starter/   # Application template (like laravel/laravel)
├── examples/            # Clean examples
└── docs/               # Documentation
```

## Migration Steps

### Step 1: Understand the New Structure

The new structure separates:

1. **Framework Code** (`crates/`) - The actual RustForge framework
2. **Application Template** (`rustforge-starter/`) - What users get with `forge new`

### Step 2: For Framework Contributors

If you're contributing to the framework itself:

```bash
# Your work stays in crates/
cd crates/
# Continue working on framework crates as before
```

No migration needed! The framework structure remains the same.

### Step 3: For Application Developers

If you have an existing RustForge application, migrate to the new structure:

#### Option A: Start Fresh (Recommended)

```bash
# Create a new project with the new structure
forge new my-app-v2

# Copy your application code
cp -r old-app/app my-app-v2/
cp -r old-app/routes my-app-v2/
cp -r old-app/database/migrations my-app-v2/database/
cp old-app/.env my-app-v2/.env

# Update dependencies in Cargo.toml
# Test and deploy
```

#### Option B: Manual Migration

```bash
# 1. Create new directory structure
mkdir -p app/{Http/{Controllers,Middleware},Models,Services}
mkdir -p config database/{migrations,seeders,factories} routes
mkdir -p resources/{views,js,css} public storage tests

# 2. Move existing files
mv src/controllers app/Http/Controllers/
mv src/models app/Models/
mv src/middleware app/Http/Middleware/

# 3. Update imports
# Change: use src::controllers::UserController
# To: use crate::app::Http::Controllers::UserController

# 4. Update Cargo.toml dependencies
# Change path-based dependencies to match new structure
```

### Step 4: Update Configuration

#### Old .env
```env
DATABASE_URL=sqlite:./app.db
```

#### New .env (More Laravel-like)
```env
APP_NAME="My RustForge App"
APP_ENV=local
APP_DEBUG=true

DB_CONNECTION=postgres
DB_HOST=127.0.0.1
DB_PORT=5432
DB_DATABASE=my_app
DB_USERNAME=postgres
DB_PASSWORD=
```

### Step 5: Update Cargo.toml

#### Old Dependencies
```toml
[dependencies]
rustforge = { path = "../crates/rustforge" }
```

#### New Dependencies
```toml
[dependencies]
# Use individual framework crates
rf-core = { path = "../crates/rf-core" }
rf-web = { path = "../crates/rf-web" }
rf-orm = { path = "../crates/rf-orm" }
rf-auth = { path = "../crates/rf-auth" }

# Or use published versions
rf-core = "0.1"
rf-web = "0.1"
```

### Step 6: Update main.rs

#### Old main.rs
```rust
use rustforge::App;

#[tokio::main]
async fn main() {
    let app = App::new();
    app.run().await;
}
```

#### New main.rs
```rust
mod app;
mod routes;

use rf_core::prelude::*;
use rf_web::Application;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let app = Application::new()
        .config_path("config/")
        .register_routes(routes::web::routes())
        .register_routes(routes::api::routes())
        .boot()
        .await?;

    let addr = "0.0.0.0:3000";
    println!("🚀 Server running on http://{}", addr);

    app.serve(&addr).await?;
    Ok(())
}
```

### Step 7: Migrate Routes

#### Old routes
```rust
// src/routes.rs
pub fn routes() -> Router {
    Router::new()
        .route("/", get(home))
}
```

#### New routes
```rust
// routes/web.rs
use rf_web::{Router, Route};
use crate::app::Http::Controllers::HomeController;

pub fn routes() -> Router {
    Router::new()
        .route("/", Route::get(HomeController::index))
}
```

### Step 8: Test Your Migration

```bash
# Clean build
cargo clean

# Build the project
cargo build

# Run tests
cargo test

# Start the server
cargo run

# Visit http://localhost:3000
```

## Breaking Changes

### 1. Import Paths

**Before:**
```rust
use rustforge::models::User;
```

**After:**
```rust
use crate::app::Models::User;
```

### 2. Configuration Loading

**Before:**
```rust
Config::from_env()
```

**After:**
```rust
Application::new().config_path("config/")
```

### 3. Route Registration

**Before:**
```rust
App::routes(routes)
```

**After:**
```rust
Application::new().register_routes(routes::web::routes())
```

## Compatibility Matrix

| Old Structure | New Structure | Compatible? |
|--------------|---------------|-------------|
| `src/models/` | `app/Models/` | ✅ Yes, with path updates |
| `src/controllers/` | `app/Http/Controllers/` | ✅ Yes, with path updates |
| `.env` | `.env` / `config/*.toml` | ⚠️ Partial, needs conversion |
| `Cargo.toml` | `Cargo.toml` | ⚠️ Needs dependency updates |
| `examples/` | `rustforge-starter/` | ❌ No, completely separate |

## Common Issues

### Issue 1: Import Errors

**Error:**
```
error[E0432]: unresolved import `rustforge::models`
```

**Fix:**
Update imports to use new paths:
```rust
use crate::app::Models::User;
```

### Issue 2: Configuration Not Found

**Error:**
```
Configuration file not found at config/app.toml
```

**Fix:**
Copy configuration files from `rustforge-starter/config/`:
```bash
cp -r rustforge-starter/config .
```

### Issue 3: Database Connection Failed

**Error:**
```
Failed to connect to database
```

**Fix:**
Update `.env` with new format:
```env
DB_CONNECTION=postgres
DB_HOST=127.0.0.1
DB_DATABASE=my_app
```

## Rollback Plan

If you need to rollback:

```bash
# Checkout previous version
git checkout v0.x

# Or restore from backup
cp -r backup/. .

# Rebuild
cargo build
```

## Getting Help

If you encounter issues during migration:

1. Check the [Migration FAQ](docs/migration-faq.md)
2. Review the [Example Applications](examples/)
3. Ask on [Discord](https://discord.gg/rustforge)
4. Open an [Issue](https://github.com/rustforge/rustforge/issues)

## Migration Checklist

Use this checklist to track your migration:

- [ ] Backup your current project
- [ ] Create new directory structure
- [ ] Move application code to new locations
- [ ] Update import paths
- [ ] Convert configuration files
- [ ] Update Cargo.toml dependencies
- [ ] Update main.rs
- [ ] Update route definitions
- [ ] Test database connections
- [ ] Run test suite
- [ ] Verify all features work
- [ ] Deploy to staging
- [ ] Deploy to production

## Timeline

The old structure will be supported until:

- **v1.0**: New structure introduced
- **v1.1**: Deprecation warnings added
- **v2.0**: Old structure removed (est. 6 months)

## Additional Resources

- [Laravel Migration Guide](docs/laravel-migration.md)
- [Quick Start](docs/quickstart.md)
- [Configuration Guide](docs/configuration.md)
- [Example Applications](examples/)

---

**Last Updated:** 2025-11-14
**Version:** 1.0.0
