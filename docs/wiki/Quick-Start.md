# Quick Start Guide

This guide will walk you through building your first RustForge application - a simple blog API with posts, comments, and authentication.

## What You'll Build

By the end of this guide, you'll have:
- A RESTful API with CRUD operations
- Database models with relationships
- User authentication with JWT
- Input validation
- Database migrations

Estimated time: 30 minutes

## Prerequisites

- RustForge installed ([Installation Guide](Installation))
- Basic Rust knowledge
- A code editor (VS Code, IntelliJ IDEA, etc.)

## Step 1: Create a New Project

```bash
cargo new blog-api
cd blog-api
```

## Step 2: Add Dependencies

Edit `Cargo.toml`:

```toml
[package]
name = "blog-api"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge - All-in-one import
rf = "1.0.0"

# Database
sea-orm = { version = "0.12", features = ["sqlx-sqlite", "runtime-tokio-native-tls", "macros"] }

# Async Runtime
tokio = { version = "1.37", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Environment
dotenvy = "0.15"

# DateTime
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1.8", features = ["v4", "serde"] }
```

Now you can use simplified imports:

```rust
use rf::prelude::*;  // All common imports
// Or specific imports:
use rf::{Route, Auth, DB, Hash, Response};
```

## Step 3: Configure Environment

Create `.env`:

```env
APP_NAME=BlogAPI
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:8000

DATABASE_URL=sqlite://blog.db

JWT_SECRET=your-secret-key-change-in-production
JWT_EXPIRATION=3600

LOG_LEVEL=info
```

## Step 4: Create Database Models

### Create User Model

```bash
forge make:model User --migration
```

This creates two files:
- `src/models/user.rs`
- `database/migrations/YYYY_MM_DD_HHMMSS_create_users_table.rs`

Edit `src/models/user.rs`:

```rust
use rf_orm::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(unique)]
    pub email: String,

    pub name: String,

    #[serde(skip_serializing)]
    pub password: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::post::Entity")]
    Posts,
}

impl Related<super::post::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

### Create Post Model

```bash
forge make:model Post --migration
```

Edit `src/models/post.rs`:

```rust
use rf_orm::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub user_id: i32,

    pub title: String,

    pub content: String,

    pub published: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,

    #[sea_orm(has_many = "super::comment::Entity")]
    Comments,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::comment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Comments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

## Step 5: Create Migrations

Edit the migration files in `database/migrations/`:

### Users Migration

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Name).string().not_null())
                    .col(ColumnDef::new(Users::Password).string().not_null())
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Email,
    Name,
    Password,
    CreatedAt,
    UpdatedAt,
}
```

### Run Migrations

```bash
forge migrate
```

## Step 6: Create Controllers

### Auth Controller

Create `src/controllers/auth_controller.rs`:

Handlers are written in the AWAIT-FREE style: put `#[auto_await]` **once** on the
controller `mod` and the macro auto-inserts `.await` after framework calls
(`find`, `first`, `create`, `save`, `login`, ...) and rewrites `where(...)` →
`r#where(...)`. You write the bodies exactly like Laravel — no `.await`. The
`Auth` facade is a genuinely-sync facade, so its calls are await-free either way.

```rust
use rf::prelude::*;            // User model, Auth, Hash, Response, ...
use axum::extract::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use rf_validation::Validate;
use crate::models::user::User;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 3))]
    pub name: String,

    #[validate(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,

    pub password: String,
}

#[auto_await]  // <- Once here: bodies below are await-free, Laravel-style.
mod handlers {
    use super::*;

    pub async fn register(
        Json(payload): Json<RegisterRequest>,
    ) -> Result<ResponseBuilder, (StatusCode, String)> {
        // Validate input
        payload
            .validate()
            .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;

        // Check if user exists (no `.await` — the macro inserts it on `exists`)
        let taken = User::where("email", &payload.email).exists();
        if taken {
            return Err((StatusCode::BAD_REQUEST, "Email already registered".into()));
        }

        // Hash password (Hash::make is synchronous and returns a String)
        let password_hash = Hash::make(&payload.password);

        // Create user (no `.await` — the macro inserts it on `create`)
        let user = User::create(json!({
            "email": payload.email,
            "name": payload.name,
            "password": password_hash,
        }));

        // Login via the sync Auth facade
        Auth::login(user.clone()).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Response::json(&AuthResponse {
            message: "Registration successful".to_string(),
            user,
        }))
    }

    pub async fn login(
        Json(payload): Json<LoginRequest>,
    ) -> Result<ResponseBuilder, (StatusCode, String)> {
        // Validate input
        payload
            .validate()
            .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;

        // Find user (no `.await` — the macro inserts it on `first_or_fail`)
        let user = User::where("email", &payload.email)
            .first_or_fail()
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

        // Verify password (Hash::check is synchronous and returns a bool)
        if !Hash::check(&payload.password, &user.password) {
            return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into()));
        }

        // Login via the sync Auth facade
        Auth::login(user.clone()).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        Ok(Response::json(&AuthResponse {
            message: "Login successful".to_string(),
            user,
        }))
    }

    pub async fn logout() -> ResponseBuilder {
        // Logout via the sync Auth facade (returns unit)
        Auth::logout();

        Response::json(&json!({ "message": "Logged out successfully" }))
    }

    pub async fn me() -> Result<ResponseBuilder, (StatusCode, String)> {
        // Get the current user via the sync Auth facade
        if let Some(user) = Auth::user::<User>() {
            Ok(Response::json(&user))
        } else {
            Err((StatusCode::UNAUTHORIZED, "Not authenticated".into()))
        }
    }
}

pub use handlers::*;

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user: User,
}
```

`Response::json` takes a reference and returns a `ResponseBuilder`, which
implements `axum::response::IntoResponse`. Import it from the prelude with
`use rf::web::ResponseBuilder;` (or `use rf_response::ResponseBuilder;`).

### Post Controller

Create `src/controllers/post_controller.rs`:

There is no `AuthGuard` extractor. Read the authenticated user id inside the
handler via the sync `Auth` facade. `Auth::id()` returns `Option<u64>`; the
example casts it to `i32` to match the `user_id` column. As before, `#[auto_await]`
goes **once** on the `mod`, so model calls (`where`, `find_or_fail`, `create`,
`save`, `delete`, ...) are written without `.await`.

```rust
use rf::prelude::*;            // Post model, Auth, Response, ...
use axum::extract::{Json, Path};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::json;
use rf_validation::Validate;
use crate::models::post::Post;

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 3, max = 255))]
    pub title: String,

    #[validate(length(min = 10))]
    pub content: String,

    pub published: Option<bool>,
}

/// Resolve the authenticated user id (the `auth` middleware must run first).
fn current_user_id() -> Result<i32, (StatusCode, String)> {
    Auth::id()
        .map(|id| id as i32)
        .ok_or((StatusCode::UNAUTHORIZED, "Not authenticated".to_string()))
}

#[auto_await]  // <- Once here: bodies below are await-free, Laravel-style.
mod handlers {
    use super::*;

    pub async fn index() -> Result<ResponseBuilder, (StatusCode, String)> {
        // No `.await` — the macro inserts it on `get`
        let posts = Post::where("published", true).get();
        Ok(Response::json(&posts))
    }

    pub async fn show(
        Path(id): Path<i32>,
    ) -> Result<ResponseBuilder, (StatusCode, String)> {
        // No `.await` — the macro inserts it on `find_or_fail`
        let post = Post::find_or_fail(id)
            .map_err(|_| (StatusCode::NOT_FOUND, "Post not found".to_string()))?;

        Ok(Response::json(&post))
    }

    pub async fn store(
        Json(payload): Json<CreatePostRequest>,
    ) -> Result<ResponseBuilder, (StatusCode, String)> {
        // Validate input
        payload
            .validate()
            .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;

        let user_id = current_user_id()?;

        // Create post (no `.await` — the macro inserts it on `create`)
        let post = Post::create(json!({
            "user_id": user_id,
            "title": payload.title,
            "content": payload.content,
            "published": payload.published.unwrap_or(false),
        }));

        Ok(Response::json(&post).status(StatusCode::CREATED))
    }

    pub async fn update(
        Path(id): Path<i32>,
        Json(payload): Json<CreatePostRequest>,
    ) -> Result<ResponseBuilder, (StatusCode, String)> {
        let user_id = current_user_id()?;

        // Find post (no `.await` — the macro inserts it on `find_or_fail`)
        let mut post = Post::find_or_fail(id)
            .map_err(|_| (StatusCode::NOT_FOUND, "Post not found".to_string()))?;

        // Check ownership
        if post.user_id != user_id {
            return Err((StatusCode::FORBIDDEN, "Not your post".into()));
        }

        // Update fields and persist (no `.await` — the macro inserts it on `save`)
        post.title = payload.title;
        post.content = payload.content;
        post.published = payload.published.unwrap_or(post.published);
        post.save();

        Ok(Response::json(&post))
    }

    pub async fn destroy(
        Path(id): Path<i32>,
    ) -> Result<ResponseBuilder, (StatusCode, String)> {
        let user_id = current_user_id()?;

        // Find post (no `.await` — the macro inserts it on `find_or_fail`)
        let post = Post::find_or_fail(id)
            .map_err(|_| (StatusCode::NOT_FOUND, "Post not found".to_string()))?;

        // Check ownership
        if post.user_id != user_id {
            return Err((StatusCode::FORBIDDEN, "Not your post".into()));
        }

        // Delete post (no `.await` — the macro inserts it on `delete`)
        post.delete();

        Ok(Response::no_content())
    }
}

pub use handlers::*;
```

## Step 7: Set Up Routes

Edit `src/main.rs`. RustForge handlers are `axum` handlers, so wire them up with
an `axum::Router`, share the SeaORM `DatabaseConnection` with `.with_state(...)`,
and serve with `axum::serve`. The database connection is established with SeaORM's
`Database::connect` (there is no `rf_orm::Database` type — `rf_orm` re-exports
SeaORM and adds the `DB` facade / `DatabaseManager` on top).

```rust
mod models;
mod controllers;

use axum::routing::{delete, get, post, put};
use axum::Router;
use sea_orm::{Database, DatabaseConnection};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv()?;

    // Connect to the database (SeaORM connection, shared as axum state)
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL")?).await?;

    // Public routes
    let public = Router::new()
        .route("/auth/register", post(controllers::auth_controller::register))
        .route("/auth/login", post(controllers::auth_controller::login))
        .route("/posts", get(controllers::post_controller::index))
        .route("/posts/:id", get(controllers::post_controller::show));

    // Protected routes (attach your auth middleware layer here, e.g. via
    // `.layer(...)`, so the `Auth` facade is populated before the handler runs).
    let protected = Router::new()
        .route("/auth/logout", post(controllers::auth_controller::logout))
        .route("/auth/me", get(controllers::auth_controller::me))
        .route("/posts", post(controllers::post_controller::store))
        .route("/posts/:id", put(controllers::post_controller::update))
        .route("/posts/:id", delete(controllers::post_controller::destroy));

    let app = public.merge(protected).with_state(db);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;
    println!("Server running at http://localhost:8000");
    axum::serve(listener, app).await?;

    Ok(())
}
```

> The `Route` facade (`rf::Route`) registers routes by string handler name
> (`Route::get("/", "HomeController@index")`) for Laravel-style route tables; it
> does not wire up `axum` handler functions. For a runnable server that calls the
> handlers above, use the `axum::Router` setup shown here.

## Step 8: Run Your Application

```bash
# Run migrations
forge migrate

# Start the server
cargo run
```

## Step 9: Test Your API

### Register a User

```bash
curl -X POST http://localhost:8000/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "name": "Test User",
    "password": "password123"
  }'
```

Response:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "user": {
    "id": 1,
    "email": "test@example.com",
    "name": "Test User",
    "created_at": "2025-11-23T10:00:00Z",
    "updated_at": "2025-11-23T10:00:00Z"
  }
}
```

### Login

```bash
curl -X POST http://localhost:8000/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "password123"
  }'
```

### Create a Post (Authenticated)

```bash
TOKEN="your-jwt-token-from-login"

curl -X POST http://localhost:8000/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "title": "My First Post",
    "content": "This is the content of my first post!",
    "published": true
  }'
```

### Get All Posts

```bash
curl http://localhost:8000/posts
```

### Get Single Post

```bash
curl http://localhost:8000/posts/1
```

### Update Post (Authenticated)

```bash
curl -X PUT http://localhost:8000/posts/1 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "title": "Updated Title",
    "content": "Updated content",
    "published": true
  }'
```

### Delete Post (Authenticated)

```bash
curl -X DELETE http://localhost:8000/posts/1 \
  -H "Authorization: Bearer $TOKEN"
```

## Next Steps

Congratulations! You've built your first RustForge application. Here's what to explore next:

1. **Add More Features**:
   - Comments system
   - Categories and tags
   - File uploads
   - Pagination

2. **Learn More**:
   - [Features Guide](Features) - Explore all features
   - [Examples](Examples) - More code examples
   - [API Documentation](API-Documentation) - Detailed API reference

3. **Improve Your App**:
   - Add caching with Redis
   - Implement queue jobs
   - Add tests
   - Deploy to production

## Common Issues

### Database Connection Error

Make sure `DATABASE_URL` is correct and the database exists. For SQLite, the file will be created automatically.

### JWT Authentication Error

Ensure `JWT_SECRET` is set in `.env` and you're including the Bearer token in the Authorization header.

### Validation Errors

Check that your request payload matches the validation rules defined in the request structs.

---

Need help? Check the [Examples](Examples) page or open an issue on GitHub.
