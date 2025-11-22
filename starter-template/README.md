# RustForge Starter Template

A minimal starter template for building REST APIs with RustForge-inspired architecture.

## Features

- ✅ Simple REST API with Axum
- ✅ In-memory database (easy to replace with PostgreSQL/MySQL)
- ✅ JSON endpoints
- ✅ Health check endpoint
- ✅ Structured logging with tracing
- ✅ Environment configuration with .env
- ✅ Production-ready error handling

## Quick Start

### 1. Setup

```bash
# Copy environment file
cp .env.example .env

# Install dependencies
cargo build
```

### 2. Run

```bash
cargo run
```

Your API is now running on `http://localhost:3000` 🎉

### 3. Test the API

```bash
# Root endpoint
curl http://localhost:3000/

# Health check
curl http://localhost:3000/health

# List all posts
curl http://localhost:3000/api/posts

# Get a specific post
curl http://localhost:3000/api/posts/1

# Create a new post
curl -X POST http://localhost:3000/api/posts \
  -H "Content-Type: application/json" \
  -d '{"title": "My First Post", "content": "Hello RustForge!"}'
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | Welcome message |
| GET | `/health` | Health check |
| GET | `/api/posts` | List all posts |
| GET | `/api/posts/:id` | Get a specific post |
| POST | `/api/posts` | Create a new post |

## Project Structure

```
starter-template/
├── src/
│   └── main.rs          # Main application
├── Cargo.toml           # Dependencies
├── .env.example         # Environment configuration
├── .gitignore          # Git ignore rules
└── README.md           # This file
```

## Customization

### Add Database Support

Replace the in-memory database with PostgreSQL:

```toml
# Add to Cargo.toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres"] }
```

### Add More Routes

```rust
// In src/main.rs
let app = Router::new()
    .route("/", get(root))
    .route("/api/users", get(list_users))
    // Add your routes here
    .with_state(db);
```

### Configure Logging

Edit `.env`:

```env
# Debug level
RUST_LOG=rustforge_app=debug

# Info level
RUST_LOG=rustforge_app=info

# Trace level
RUST_LOG=rustforge_app=trace
```

## Production Deployment

### Build Release Binary

```bash
cargo build --release
```

### Docker

Create a `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/rustforge-app /usr/local/bin/
CMD ["rustforge-app"]
```

Build and run:

```bash
docker build -t my-app .
docker run -p 3000:3000 my-app
```

## Next Steps

1. **Add Database**: Integrate PostgreSQL or MySQL
2. **Add Authentication**: JWT or session-based auth
3. **Add Validation**: Request validation with serde
4. **Add Tests**: Unit and integration tests
5. **Add Documentation**: API docs with OpenAPI/Swagger

## Resources

- [Axum Documentation](https://docs.rs/axum)
- [Tokio Guide](https://tokio.rs/tokio/tutorial)
- [Rust Book](https://doc.rust-lang.org/book/)

## License

MIT OR Apache-2.0

---

**Happy Building with RustForge!** 🚀
