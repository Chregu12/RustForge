# Chapter 1: Project Setup

**Time:** 15 minutes
**Difficulty:** Easy

---

## Overview

In this chapter, you'll set up the foundation for your blog application. We'll create the project, configure the database, and install dependencies.

## Goals

By the end of this chapter, you will:

- ✅ Create a new RustForge project
- ✅ Configure PostgreSQL database
- ✅ Set up environment variables
- ✅ Install required dependencies
- ✅ Verify the installation works

---

## Step 1: Create the Project

Open your terminal and run:

```bash
forge new blog
cd blog
```

This creates a new RustForge project with the standard structure:

```
blog/
├── src/
│   ├── main.rs
│   ├── routes.rs
│   ├── controllers/
│   ├── models/
│   └── views/
├── migrations/
├── tests/
├── .env.example
└── Cargo.toml
```

---

## Step 2: Configure the Database

### Option A: Using Docker (Recommended)

If you have Docker installed, start a PostgreSQL container:

```bash
docker run -d \
  --name blog-postgres \
  -e POSTGRES_PASSWORD=secret \
  -e POSTGRES_DB=blog \
  -p 5432:5432 \
  postgres:16
```

### Option B: Local PostgreSQL

If you have PostgreSQL installed locally, create the database:

```bash
createdb blog
```

---

## Step 3: Environment Configuration

Copy the example environment file:

```bash
cp .env.example .env
```

Edit `.env` with your database credentials:

```env
# Application
APP_NAME=RustForgeBlog
APP_ENV=local
APP_DEBUG=true
APP_KEY=base64:your-random-key-here
APP_URL=http://127.0.0.1:8000

# Database
DATABASE_URL=postgres://postgres:secret@localhost:5432/blog

# Cache
CACHE_DRIVER=redis
REDIS_URL=redis://127.0.0.1:6379

# Mail
MAIL_DRIVER=smtp
MAIL_HOST=localhost
MAIL_PORT=1025
MAIL_USERNAME=null
MAIL_PASSWORD=null
MAIL_FROM=noreply@blog.local

# Session
SESSION_DRIVER=cookie
SESSION_LIFETIME=120

# Storage
FILESYSTEM_DRIVER=local
```

### Generate Application Key

Generate a secure application key:

```bash
forge key:generate
```

This updates your `.env` file with a secure random key.

---

## Step 4: Install Dependencies

Add the required dependencies to `Cargo.toml`:

```toml
[package]
name = "blog"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge Core
rf-core = "0.1"
rf-routing = "0.1"
rf-http = "0.1"
rf-views = "0.1"

# Database
rf-orm = "0.1"
rf-eloquent = "0.1"
sea-orm = { version = "0.12", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros"] }

# Authentication
rf-auth = "0.1"
rf-hashing = "0.1"

# Validation
rf-validation = "0.1"

# Storage
rf-storage = "0.1"

# Async Runtime
tokio = { version = "1.35", features = ["full"] }
axum = "0.7"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Templates
tera = "1.19"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["serde", "v4"] }
```

Install the dependencies:

```bash
cargo build
```

This will take a few minutes on the first run as it downloads and compiles all dependencies.

---

## Step 5: Test Database Connection

Let's verify the database connection works.

Create `src/bin/test_db.rs`:

```rust
use sea_orm::{Database, ConnectionTrait, Statement};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    println!("Connecting to database...");
    let db = Database::connect(&database_url).await?;

    // Test query
    let result = db.query_one(Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT 1 as num".to_string()
    )).await?;

    if let Some(row) = result {
        let num: i32 = row.try_get("", "num")?;
        println!("✅ Database connection successful! (test query returned: {})", num);
    }

    Ok(())
}
```

Run the test:

```bash
cargo run --bin test_db
```

You should see:

```
Connecting to database...
✅ Database connection successful! (test query returned: 1)
```

---

## Step 6: Initial Application Structure

Let's set up the basic application structure.

### Update `src/main.rs`

```rust
use axum::{Router, serve};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

mod routes;
mod controllers;
mod models;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Build application
    let app = Router::new()
        .merge(routes::register_routes())
        .nest_service("/static", ServeDir::new("public"));

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    println!("🚀 Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    serve(listener, app).await.unwrap();
}
```

### Create `src/routes.rs`

```rust
use axum::{Router, routing::get};
use axum::response::Html;

pub fn register_routes() -> Router {
    Router::new()
        .route("/", get(home))
}

async fn home() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>RustForge Blog</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            text-align: center;
        }
        h1 { color: #e74c3c; }
    </style>
</head>
<body>
    <h1>Welcome to RustForge Blog</h1>
    <p>Your blog is ready to be built!</p>
    <p><small>Chapter 1 completed ✅</small></p>
</body>
</html>
    "#)
}
```

### Create Module Files

```bash
mkdir -p src/controllers src/models src/views
touch src/controllers/mod.rs src/models/mod.rs
```

---

## Step 7: Run the Application

Start the development server:

```bash
cargo run
```

You should see:

```
🚀 Server starting on http://127.0.0.1:8000
```

Open your browser to `http://127.0.0.1:8000` and you should see:

```
Welcome to RustForge Blog
Your blog is ready to be built!
Chapter 1 completed ✅
```

---

## Verification Checklist

Before moving to the next chapter, verify:

- [ ] Project created with `forge new blog`
- [ ] PostgreSQL database running
- [ ] `.env` file configured with correct DATABASE_URL
- [ ] Dependencies installed (`cargo build` successful)
- [ ] Database connection test passed
- [ ] Application runs and shows welcome page

---

## Project Structure Overview

Your project should now look like this:

```
blog/
├── src/
│   ├── main.rs              ✅ Application entry point
│   ├── routes.rs            ✅ Route definitions
│   ├── controllers/
│   │   └── mod.rs           ✅ Empty module
│   ├── models/
│   │   └── mod.rs           ✅ Empty module
│   └── bin/
│       └── test_db.rs       ✅ Database test script
├── migrations/               (empty for now)
├── tests/                    (empty for now)
├── public/                   (empty for now)
├── .env                     ✅ Environment configuration
└── Cargo.toml               ✅ Dependencies configured
```

---

## Troubleshooting

### Database Connection Failed

**Error:** `connection refused`

**Solutions:**
1. Check PostgreSQL is running: `docker ps` or `pg_isready`
2. Verify DATABASE_URL in `.env`
3. Check firewall isn't blocking port 5432

### Cargo Build Errors

**Error:** `could not compile`

**Solutions:**
1. Ensure Rust 1.75+: `rustc --version`
2. Clean and rebuild: `cargo clean && cargo build`
3. Update dependencies: `cargo update`

### Port Already in Use

**Error:** `Address already in use (os error 48)`

**Solution:**
Change the port in `main.rs`:
```rust
let addr = SocketAddr::from(([127, 0, 0, 1], 8080)); // Use 8080 instead
```

---

## What's Next?

Great job! You've set up the foundation for your blog. In the next chapter, we'll:

- Create database migrations
- Design the schema
- Set up tables for users, posts, and comments

**Next:** [Chapter 2: Database & Migrations](./02-database.md)

---

## Summary

In this chapter, you:

1. ✅ Created a new RustForge project
2. ✅ Configured PostgreSQL database
3. ✅ Set up environment variables
4. ✅ Installed all dependencies
5. ✅ Tested the database connection
6. ✅ Created basic application structure
7. ✅ Verified everything works

**Time spent:** ~15 minutes ✅

Continue to [Chapter 2: Database & Migrations](./02-database.md) →
