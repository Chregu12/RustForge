# Installation Guide

This guide will help you install RustForge and set up your development environment.

## Prerequisites

Before installing RustForge, ensure you have the following installed:

### Required

- **Rust 1.75 or higher**
  ```bash
  # Check your Rust version
  rustc --version

  # Update Rust if needed
  rustup update
  ```

- **Cargo** (comes with Rust)
  ```bash
  cargo --version
  ```

### Optional (Recommended)

- **Database**:
  - PostgreSQL 12+ (recommended)
  - MySQL 8+
  - SQLite 3.35+ (for development)

- **Cache Server**:
  - Redis 6+ (recommended)
  - Memcached 1.6+

- **Git** (for version control)

## Installation Methods

### Method 1: Using the `rf` Crate (Recommended)

The easiest way to use RustForge is with the unified `rf` crate:

```toml
[dependencies]
# All-in-one RustForge
rf = "1.0.0"

# Async runtime
tokio = { version = "1.37", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

This gives you access to everything with simplified imports:

```rust
use rf::{Route, Auth, DB, Hash, Collection};
// Or use prelude for all common imports
use rf::prelude::*;
```

Then run:

```bash
cargo build
```

### Method 2: Individual Crates (Advanced)

For more control, you can add individual crates:

```toml
[dependencies]
# Core framework
rf-core = "1.0.0"

# Database & ORM
rf-orm = "1.0.0"
rf-eloquent = "1.0.0"

# HTTP & Routing
rf-web = "1.0.0"
rf-routing = "1.0.0"

# Authentication
rf-auth = "1.0.0"
rf-sanctum = "1.0.0"

# Validation
rf-validation = "1.0.0"

# Caching
rf-cache = "1.0.0"

# Queue & Jobs
rf-queue = "1.0.0"
rf-jobs = "1.0.0"

# Additional features (optional)
rf-mail = "1.0.0"
rf-storage = "1.0.0"
rf-broadcast = "1.0.0"

# Phase 21 features (optional)
rf-dusk = "1.0.0"      # Browser testing
rf-echo = "1.0.0"      # Broadcasting client
rf-envoy = "1.0.0"     # SSH deployment
rf-sail = "1.0.0"      # Docker environment
rf-spark = "1.0.0"     # SaaS billing

# Async runtime
tokio = { version = "1.37", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

Then run:

```bash
cargo build
```

### Method 3: Clone from GitHub

Clone the repository and build from source:

```bash
# Clone the repository
git clone https://github.com/Chregu12/RustForge.git
cd RustForge

# Build the workspace
cargo build --release

# Install the CLI tool
cargo install --path crates/forge-cli
```

## Setting Up Your Project

### 1. Create a New Project

```bash
# Create a new Rust project
cargo new my-rustforge-app
cd my-rustforge-app
```

### 2. Configure Dependencies

Edit your `Cargo.toml` to include RustForge dependencies (see Method 1 above).

### 3. Set Up Environment Variables

Create a `.env` file in your project root:

```env
# Application
APP_NAME=MyRustForgeApp
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:8000

# Database
DATABASE_URL=sqlite://database.db
# Or for PostgreSQL:
# DATABASE_URL=postgres://user:password@localhost:5432/myapp
# Or for MySQL:
# DATABASE_URL=mysql://user:password@localhost:3306/myapp

# Cache
CACHE_DRIVER=file
# Or for Redis:
# CACHE_DRIVER=redis
# REDIS_URL=redis://localhost:6379

# Queue
QUEUE_DRIVER=sync
# Or for Redis queue:
# QUEUE_DRIVER=redis

# Mail
MAIL_DRIVER=smtp
MAIL_HOST=smtp.mailtrap.io
MAIL_PORT=2525
MAIL_USERNAME=your_username
MAIL_PASSWORD=your_password
MAIL_FROM_ADDRESS=noreply@example.com
MAIL_FROM_NAME="${APP_NAME}"

# Storage
FILESYSTEM_DRIVER=local
# Or for S3:
# FILESYSTEM_DRIVER=s3
# AWS_ACCESS_KEY_ID=your_key
# AWS_SECRET_ACCESS_KEY=your_secret
# AWS_DEFAULT_REGION=us-east-1
# AWS_BUCKET=your_bucket

# Session
SESSION_DRIVER=file
SESSION_LIFETIME=120

# Logging
LOG_LEVEL=info
```

### 4. Initialize the Database

For SQLite (development):
```bash
# The database file will be created automatically
```

For PostgreSQL:
```bash
# Create the database
createdb myapp

# Update DATABASE_URL in .env
DATABASE_URL=postgres://user:password@localhost:5432/myapp
```

For MySQL:
```bash
# Create the database
mysql -u root -p -e "CREATE DATABASE myapp;"

# Update DATABASE_URL in .env
DATABASE_URL=mysql://user:password@localhost:3306/myapp
```

## Installing the CLI Tool (Forge)

RustForge comes with a powerful CLI tool called `forge`:

```bash
# If you cloned the repository
cargo install --path crates/forge-cli

# Verify installation
forge --version
```

### CLI Commands Available

```bash
forge make:model User --migration      # Create model with migration
forge make:controller UserController   # Create controller
forge make:migration create_users      # Create migration
forge migrate                          # Run migrations
forge migrate:rollback                 # Rollback last migration
forge db:seed                          # Seed database
forge route:list                       # List all routes
forge cache:clear                      # Clear cache
forge queue:work                       # Start queue worker
```

See [CLI Commands](CLI-Commands) for full documentation.

## Verifying Installation

Create a simple test file to verify your installation:

```rust
// src/main.rs
use rf::prelude::*;          // Auth, DB, Hash, Response, ...
use serde_json::json;        // `json!` is not part of `rf::prelude`

#[tokio::main]
async fn main() {
    // Hash facade (synchronous) — make() returns a String, check() returns a bool
    let hashed = Hash::make("secret");
    assert!(Hash::check("secret", &hashed));
    println!("Hash facade works");

    // DB facade query builder — get() is async and yields a Vec of rows
    let users = DB::table("users").get().await.unwrap();
    println!("Fetched {} user row(s)", users.len());

    // Response builder — json() takes a reference and returns a ResponseBuilder
    let _response = Response::json(&json!({ "message": "Hello, RustForge!" }));
    println!("Response facade works");

    println!("RustForge is installed correctly!");
}
```

Run your application:

```bash
cargo run
```

You should see the facade checks print successfully, confirming RustForge is
installed and the core facades (`Hash`, `DB`, `Response`) are available.

## Troubleshooting

### Common Issues

#### Issue: "Cannot find crate rf-core"

**Solution**: Ensure you've added all dependencies to `Cargo.toml` and run `cargo build`.

#### Issue: "Database connection failed"

**Solution**:
1. Check your `DATABASE_URL` in `.env`
2. Ensure database server is running
3. Verify credentials and database exists

#### Issue: "Redis connection failed"

**Solution**:
1. Install Redis: `brew install redis` (macOS) or `apt install redis` (Linux)
2. Start Redis: `redis-server`
3. Check `REDIS_URL` in `.env`

#### Issue: "Permission denied" when installing CLI

**Solution**: Use `cargo install --path crates/forge-cli` without sudo, or install to user directory.

### Getting Help

If you encounter issues:

1. Check the [FAQ](FAQ)
2. Search existing [GitHub Issues](https://github.com/Chregu12/RustForge/issues)
3. Create a new issue with details about your problem

## Next Steps

Now that you have RustForge installed, continue with:

1. **[Quick Start Guide](Quick-Start)** - Build your first application
2. **[Features](Features)** - Learn about available features
3. **[Examples](Examples)** - See code examples
4. **[API Documentation](API-Documentation)** - Detailed API reference

## Updating RustForge

To update to the latest version:

```bash
# Update dependencies in Cargo.toml to latest version
[dependencies]
rf-core = "1.0.0"  # Update to latest version

# Update cargo
cargo update

# Rebuild
cargo build
```

## Development vs Production

### Development Setup

```env
APP_ENV=local
APP_DEBUG=true
DATABASE_URL=sqlite://database.db
CACHE_DRIVER=file
QUEUE_DRIVER=sync
```

### Production Setup

```env
APP_ENV=production
APP_DEBUG=false
DATABASE_URL=postgres://user:password@localhost:5432/myapp
CACHE_DRIVER=redis
REDIS_URL=redis://localhost:6379
QUEUE_DRIVER=redis
LOG_LEVEL=warning
```

**Important**: Never commit your `.env` file to version control. Use `.env.example` as a template.

---

Ready to build? Continue to the [Quick Start Guide](Quick-Start).
