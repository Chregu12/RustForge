# Installation Guide

This guide will help you install RustForge and create your first application.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Git** - [Install Git](https://git-scm.com/)
- **PostgreSQL, MySQL, or SQLite** - For database operations
- **Redis** (optional) - For caching and queue features

## Installing the Forge CLI

The Forge CLI is the command-line tool for creating and managing RustForge applications.

```bash
cargo install forge-cli
```

Verify the installation:

```bash
forge --version
```

## Creating a New Application

### Option 1: Using Forge CLI (Recommended)

```bash
# Create a new application
forge new my-app

# Navigate to your project
cd my-app

# Run the application
cargo run
```

### Option 2: Using the Starter Template

```bash
# Clone the starter template
git clone https://github.com/rustforge/rustforge-starter.git my-app

# Navigate to your project
cd my-app

# Remove git history and reinitialize
rm -rf .git
git init

# Copy environment file
cp .env.example .env

# Run the application
cargo run
```

## Configuration

### Environment Setup

Edit your `.env` file to configure your application:

```env
APP_NAME="My RustForge App"
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:3000

DB_CONNECTION=postgres
DB_HOST=127.0.0.1
DB_PORT=5432
DB_DATABASE=my_app
DB_USERNAME=postgres
DB_PASSWORD=
```

### Database Setup

1. Create your database:
```bash
# PostgreSQL
createdb my_app

# Or use the forge command
forge database:create
```

2. Run migrations:
```bash
forge migrate
```

3. (Optional) Seed the database:
```bash
forge db:seed
```

### Redis Setup

If you plan to use caching or queues, install and start Redis:

```bash
# macOS
brew install redis
brew services start redis

# Ubuntu
sudo apt install redis-server
sudo systemctl start redis

# Windows (WSL)
sudo apt install redis-server
sudo service redis-server start
```

## Running Your Application

### Development Server

```bash
cargo run
```

Your application will be available at `http://localhost:3000`

### Watch Mode (Auto-reload)

Install cargo-watch for auto-reloading:

```bash
cargo install cargo-watch
cargo watch -x run
```

### Production Build

```bash
cargo build --release
./target/release/my-rustforge-app
```

## Next Steps

- [Quick Start Tutorial](quickstart.md)
- [Routing Guide](routing.md)
- [Database & ORM](models.md)
- [Authentication](authentication.md)

## Troubleshooting

### Cargo Build Fails

Make sure you have the latest Rust version:
```bash
rustup update
```

### Database Connection Error

1. Check your database is running
2. Verify credentials in `.env`
3. Ensure the database exists

### Port Already in Use

Change the port in `.env`:
```env
APP_PORT=3001
```

## Getting Help

- [Documentation](https://rustforge.dev/docs)
- [Discord Community](https://discord.gg/rustforge)
- [GitHub Issues](https://github.com/rustforge/rustforge/issues)
