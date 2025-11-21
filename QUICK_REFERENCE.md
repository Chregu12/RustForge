# RustForge Quick Reference Card

## New Project Structure

```
rustforge/                    (Framework Repository)
│
├── crates/                   Framework Code
│   ├── rf-core/             Core utilities
│   ├── rf-orm/              Database ORM
│   ├── rf-web/              Web framework
│   ├── rf-auth/             Authentication
│   └── ...                  95+ crates
│
└── rustforge-starter/        Application Template
    ├── app/                 Your code here
    ├── config/              Configuration
    ├── routes/              Routes
    ├── resources/           Views & assets
    └── database/            Migrations & seeds
```

## Quick Start

```bash
# 1. Install CLI
cargo install forge-cli

# 2. Create project
forge new my-app

# 3. Run it
cd my-app
cp .env.example .env
cargo run
```

## Laravel → RustForge Cheat Sheet

| Laravel | RustForge |
|---------|-----------|
| `laravel new` | `forge new` |
| `php artisan` | `forge` |
| `Route::get()` | `Router::get()` |
| `User::find()` | `User::find().await?` |
| `Cache::get()` | `cache.get().await?` |
| `app/Http/Controllers` | `app/Http/Controllers` ✅ Same! |
| `config/*.php` | `config/*.toml` |

## Directory Structure

```
my-app/
├── app/
│   ├── Http/
│   │   ├── Controllers/     ← Your controllers
│   │   └── Middleware/      ← Your middleware
│   ├── Models/              ← Your models
│   └── Services/            ← Business logic
│
├── config/                  ← Configuration
│   ├── app.toml
│   ├── database.toml
│   └── ...
│
├── routes/                  ← Routes
│   ├── web.rs
│   └── api.rs
│
├── resources/               ← Frontend
│   ├── views/
│   ├── js/
│   └── css/
│
├── database/                ← Database
│   ├── migrations/
│   ├── seeders/
│   └── factories/
│
└── tests/                   ← Tests
    ├── Feature/
    └── Unit/
```

## Common Commands

```bash
# Code Generation
forge make:model User
forge make:controller UserController
forge make:migration create_users_table
forge make:seeder UserSeeder

# Database
forge migrate
forge migrate:rollback
forge migrate:fresh --seed
forge db:seed

# Development
forge serve
forge tinker
forge queue:work

# Cache
forge cache:clear
forge config:cache

# Testing
cargo test
```

## Configuration Files

```
config/
├── app.toml        → Application settings
├── database.toml   → Database connections
├── cache.toml      → Cache & Redis
├── mail.toml       → Email settings
├── queue.toml      → Job queue
└── services.toml   → Third-party APIs
```

## Creating Your First Feature

```rust
// 1. Create a model
// app/Models/Post.rs
#[derive(Model)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
}

// 2. Create a controller
// app/Http/Controllers/PostController.rs
pub async fn index() -> Result<Response> {
    let posts = Post::all().await?;
    Ok(Response::json(posts))
}

// 3. Add a route
// routes/web.rs
Router::new()
    .route("/posts", Route::get(PostController::index))
```

## Environment Variables

```env
# .env
APP_NAME="My App"
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:3000

DB_CONNECTION=postgres
DB_HOST=127.0.0.1
DB_DATABASE=my_app

REDIS_HOST=127.0.0.1
CACHE_DRIVER=redis
QUEUE_CONNECTION=redis
```

## Important Paths

| What | Path |
|------|------|
| Controllers | `app/Http/Controllers/` |
| Models | `app/Models/` |
| Middleware | `app/Http/Middleware/` |
| Routes | `routes/` |
| Views | `resources/views/` |
| Config | `config/` |
| Migrations | `database/migrations/` |
| Tests | `tests/` |
| Public | `public/` |
| Storage | `storage/` |

## Getting Help

- **Docs:** https://rustforge.dev/docs
- **Discord:** https://discord.gg/rustforge
- **GitHub:** https://github.com/rustforge/rustforge
- **Migration Guide:** MIGRATION_GUIDE.md

## Links

- [Installation Guide](docs/installation.md)
- [Laravel Migration](docs/laravel-migration.md)
- [Restructuring Report](RESTRUCTURING_REPORT.md)
- [Before/After](BEFORE_AFTER_COMPARISON.md)
