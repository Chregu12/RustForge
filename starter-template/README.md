# RustForge Starter Template

A production-ready starter template for building REST APIs with Rust. Features a Laravel-inspired architecture with modern best practices, comprehensive authentication, database integration, and more.

## Features

### Core Features
- **Web Framework**: Axum with Tokio async runtime
- **Database**: SeaORM with migrations (PostgreSQL, MySQL, SQLite)
- **Authentication**: JWT-based auth with Argon2 password hashing
- **Validation**: Request validation with comprehensive rules
- **Middleware**: Logging, authentication, CORS
- **Configuration**: Environment-based config management
- **Testing**: Integration test setup
- **Development Tools**: Helper scripts and hot-reload support

### Architecture
- **MVC Structure**: Models, Controllers, and organized routing
- **Type Safety**: Leverages Rust's type system for reliability
- **Error Handling**: Comprehensive error types and responses
- **Logging**: Structured logging with tracing
- **Security**: Secure password hashing, JWT tokens, CORS

## Quick Start

### Prerequisites

- Rust 1.75 or higher
- Database (optional - uses SQLite by default)

### Installation

```bash
# 1. Clone or download the template
cd starter-template

# 2. Copy environment configuration
cp .env.example .env

# 3. Install dependencies and build
cargo build

# 4. Run the application
cargo run
```

Your API is now running on `http://localhost:3000` 🎉

### Using the Development Script

```bash
# Make the script executable (first time only)
chmod +x dev.sh

# Run with auto-reload
./dev.sh dev

# Other commands
./dev.sh build     # Build project
./dev.sh test      # Run tests
./dev.sh format    # Format code
./dev.sh lint      # Run linter
./dev.sh release   # Build release binary
```

## Project Structure

```
starter-template/
├── src/
│   ├── config/              # Configuration management
│   │   ├── database.rs      # Database config
│   │   ├── settings.rs      # App settings
│   │   └── mod.rs
│   ├── controllers/         # Request handlers (Laravel-style)
│   │   ├── auth_controller.rs    # Authentication
│   │   ├── post_controller.rs    # Post CRUD
│   │   ├── user_controller.rs    # User profile
│   │   └── mod.rs
│   ├── middleware/          # Custom middleware
│   │   ├── auth.rs          # JWT authentication
│   │   ├── logging.rs       # Request logging
│   │   └── mod.rs
│   ├── models/              # Database entities (SeaORM)
│   │   ├── user.rs          # User model
│   │   ├── post.rs          # Post model
│   │   └── mod.rs
│   └── main.rs              # Application entry point
├── database/
│   ├── migrations/          # Database migrations
│   │   ├── m20240101_000001_create_users_table.rs
│   │   ├── m20240101_000002_create_posts_table.rs
│   │   └── mod.rs
│   └── seeders/             # Database seeders (optional)
├── tests/                   # Integration tests
│   ├── integration_test.rs
│   └── README.md
├── .env.example             # Environment configuration template
├── .gitignore
├── Cargo.toml               # Dependencies
├── dev.sh                   # Development helper script
└── README.md                # This file
```

## API Endpoints

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | API information |
| GET | `/health` | Health check |
| POST | `/auth/register` | Register new user |
| POST | `/auth/login` | Login user |
| GET | `/api/posts` | List all posts |
| GET | `/api/posts/:id` | Get single post |

### Protected Endpoints (Require JWT Token)

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/posts` | Create new post |
| PUT | `/api/posts/:id` | Update post (owner only) |
| DELETE | `/api/posts/:id` | Delete post (owner only) |
| GET | `/api/profile` | Get user profile |

## Usage Examples

### 1. Register a New User

```bash
curl -X POST http://localhost:3000/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "password123",
    "name": "John Doe"
  }'
```

Response:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "user": {
    "id": 1,
    "email": "user@example.com",
    "name": "John Doe"
  }
}
```

### 2. Login

```bash
curl -X POST http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "password123"
  }'
```

### 3. Create a Post (Authenticated)

```bash
curl -X POST http://localhost:3000/api/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "title": "My First Post",
    "content": "This is the content of my post",
    "published": true
  }'
```

### 4. Get User Profile (Authenticated)

```bash
curl http://localhost:3000/api/profile \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### 5. Update a Post (Authenticated, Owner Only)

```bash
curl -X PUT http://localhost:3000/api/posts/1 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "title": "Updated Title",
    "published": true
  }'
```

### 6. Delete a Post (Authenticated, Owner Only)

```bash
curl -X DELETE http://localhost:3000/api/posts/1 \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

## Configuration

The application is configured via environment variables in the `.env` file.

### Key Configuration Options

```env
# Application
APP_NAME=RustForge App
APP_ENV=development
APP_DEBUG=true

# Server
HOST=0.0.0.0
PORT=3000

# Database (choose one)
DATABASE_URL=sqlite:./data.db                                    # SQLite
# DATABASE_URL=postgres://user:password@localhost:5432/dbname   # PostgreSQL
# DATABASE_URL=mysql://user:password@localhost:3306/dbname      # MySQL

# Authentication
JWT_SECRET=your-secret-key-min-32-characters-long
JWT_EXPIRATION_HOURS=24

# Logging
RUST_LOG=rustforge_app=debug,tower_http=debug,sea_orm=info
```

## Database

### Migrations

Migrations run automatically when the application starts. You can also run them manually:

```rust
// Migrations are in database/migrations/
// They run automatically via main.rs
```

### Supported Databases

- **SQLite** (default) - Great for development
- **PostgreSQL** - Recommended for production
- **MySQL** - Alternative production option

### Changing Database

1. Update `DATABASE_URL` in `.env`
2. Restart the application
3. Migrations will run automatically

Example for PostgreSQL:
```env
DATABASE_URL=postgres://user:password@localhost:5432/myapp
```

## Development

### Hot Reload

Install cargo-watch for automatic recompilation:

```bash
cargo install cargo-watch
./dev.sh dev
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run tests
cargo test

# Or use the helper script
./dev.sh format
./dev.sh lint
./dev.sh test
```

### Adding New Features

#### Adding a New Model

1. Create model in `src/models/`:

```rust
// src/models/comment.rs
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "comments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub content: String,
    pub post_id: i32,
    pub user_id: i32,
    pub created_at: DateTime,
}
```

2. Create migration in `database/migrations/`
3. Add to `src/models/mod.rs`

#### Adding a New Controller

1. Create controller in `src/controllers/`:

```rust
// src/controllers/comment_controller.rs
pub struct CommentController;

impl CommentController {
    pub async fn list(/* ... */) -> Result<Json<Vec<Comment>>, StatusCode> {
        // Implementation
    }
}
```

2. Add routes in `src/main.rs`:

```rust
.route("/api/comments", get(CommentController::list))
```

#### Adding Middleware

1. Create middleware in `src/middleware/`:

```rust
pub async fn my_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Pre-processing
    let response = next.run(request).await;
    // Post-processing
    response
}
```

2. Apply in `src/main.rs`:

```rust
.layer(axum_middleware::from_fn(my_middleware))
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Writing Tests

See `tests/integration_test.rs` for examples.

```rust
#[tokio::test]
async fn test_my_endpoint() {
    // Setup, execute, assert
}
```

## Deployment

### Building for Production

```bash
# Build optimized binary
cargo build --release

# Binary location
./target/release/rustforge-app
```

### Environment Variables

Ensure these are set in production:

```env
APP_ENV=production
APP_DEBUG=false
DATABASE_URL=postgres://...
JWT_SECRET=<strong-random-secret-32-chars-min>
```

### Docker Deployment

Create `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/rustforge-app /usr/local/bin/
EXPOSE 3000
CMD ["rustforge-app"]
```

Build and run:

```bash
docker build -t rustforge-app .
docker run -p 3000:3000 --env-file .env rustforge-app
```

## Security Considerations

### Production Checklist

- [ ] Change `JWT_SECRET` to a strong random value (32+ characters)
- [ ] Set `APP_ENV=production` and `APP_DEBUG=false`
- [ ] Use PostgreSQL or MySQL instead of SQLite
- [ ] Enable HTTPS/TLS
- [ ] Configure CORS appropriately for your domain
- [ ] Set up rate limiting
- [ ] Review and update database connection limits
- [ ] Set up proper logging and monitoring
- [ ] Regular security updates

### Password Security

- Passwords are hashed with Argon2 (industry standard)
- Minimum 8 characters enforced (customize in validation)
- Never logged or exposed in responses

### JWT Tokens

- Tokens expire after 24 hours (configurable)
- Secret key must be at least 32 characters
- Tokens include user_id and email claims

## Troubleshooting

### Common Issues

**Issue**: Database connection fails
```
Solution: Check DATABASE_URL in .env and ensure database is running
```

**Issue**: JWT token invalid
```
Solution: Ensure JWT_SECRET matches between registration and validation
```

**Issue**: Port already in use
```
Solution: Change PORT in .env or kill the process using the port
```

### Debug Mode

Enable verbose logging:

```env
RUST_LOG=trace
```

## Extending the Template

### Adding Email Support

1. Add dependency: `lettre = "0.11"`
2. Create email service in `src/services/email.rs`
3. Configure SMTP in `.env`

### Adding Redis Caching

1. Add dependency: `redis = "0.24"`
2. Create cache service in `src/services/cache.rs`
3. Add REDIS_URL to `.env`

### Adding File Uploads

1. Add multipart support (already in Axum)
2. Create upload controller
3. Configure storage (local/S3)

### Adding WebSockets

Axum has built-in WebSocket support:

```rust
use axum::extract::ws::{WebSocket, WebSocketUpgrade};

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}
```

## Resources

- [Axum Documentation](https://docs.rs/axum)
- [SeaORM Guide](https://www.sea-ql.org/SeaORM/docs/intro)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Rust Book](https://doc.rust-lang.org/book/)

## Contributing

This is a starter template. Feel free to customize it for your needs!

## License

MIT OR Apache-2.0

---

**Built with RustForge** 🚀

Happy Building!
