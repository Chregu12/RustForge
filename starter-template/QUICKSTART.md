# Quick Start Guide

Get your RustForge API up and running in 5 minutes!

## Prerequisites

- Rust 1.75+ installed ([rustup.rs](https://rustup.rs))
- A terminal/command prompt

## 1. Setup (30 seconds)

```bash
# Navigate to the template
cd starter-template

# Copy environment configuration
cp .env.example .env

# Build the project (first build takes longer)
cargo build
```

## 2. Run (10 seconds)

```bash
cargo run
```

You should see:
```
🚀 Starting rustforge-app v0.1.0
📝 Environment: development
📦 Connecting to database...
✅ Database connected
🔄 Running database migrations...
✅ Migrations completed
✅ Server started successfully on http://0.0.0.0:3000
```

## 3. Test (2 minutes)

### Test the health endpoint

```bash
curl http://localhost:3000/health
```

Expected response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00Z",
  "service": "rustforge-app",
  "version": "0.1.0"
}
```

### Register a user

```bash
curl -X POST http://localhost:3000/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "password123",
    "name": "Test User"
  }'
```

Expected response:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": 1,
    "email": "test@example.com",
    "name": "Test User"
  }
}
```

**Save the token!** You'll need it for authenticated requests.

### Create a post (authenticated)

```bash
# Replace YOUR_TOKEN with the token from registration
curl -X POST http://localhost:3000/api/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "title": "My First Post",
    "content": "Hello from RustForge!",
    "published": true
  }'
```

### List all posts

```bash
curl http://localhost:3000/api/posts
```

### Get your profile (authenticated)

```bash
curl http://localhost:3000/api/profile \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## 4. Development Workflow

### Hot reload (auto-restart on code changes)

```bash
# Install cargo-watch (one-time)
cargo install cargo-watch

# Run with hot reload
./dev.sh dev
```

### Other useful commands

```bash
./dev.sh build     # Build project
./dev.sh test      # Run tests
./dev.sh format    # Format code
./dev.sh lint      # Check code quality
./dev.sh clean     # Clean build artifacts
```

## What's Next?

### Learn the Structure

Check out the organized project structure:
- `src/models/` - Database models (User, Post)
- `src/controllers/` - Request handlers (Auth, Posts, Users)
- `src/middleware/` - Custom middleware (Auth, Logging)
- `src/config/` - Configuration management
- `database/migrations/` - Database migrations

### Add Your Features

1. **Add a new model**: Create in `src/models/`
2. **Add a new controller**: Create in `src/controllers/`
3. **Add routes**: Update `src/main.rs`
4. **Add middleware**: Create in `src/middleware/`

See the full README.md for detailed guides!

### Switch to PostgreSQL (Production)

1. Install PostgreSQL
2. Create a database
3. Update `.env`:
   ```env
   DATABASE_URL=postgres://user:password@localhost:5432/myapp
   ```
4. Restart the app

### Deploy to Production

```bash
# Build release binary
cargo build --release

# Binary is at: ./target/release/rustforge-app
```

## Troubleshooting

### Port already in use?

Change the port in `.env`:
```env
PORT=8080
```

### Database error?

Reset the database:
```bash
./dev.sh db:reset
```

### Need help?

Check the full documentation in [README.md](README.md)

---

**You're all set!** Start building your API. 🚀
