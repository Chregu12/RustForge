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
# RustForge Core
rf-core = "1.0.0"
rf-orm = "1.0.0"
rf-http = "1.0.0"
rf-auth = "1.0.0"
rf-validation = "1.0.0"

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

```rust
use rf_http::{Request, Response, Json};
use rf_auth::Hash;
use rf_auth_facade::Auth;
use rf_validation::Validate;
use serde::{Deserialize, Serialize};
use crate::models::user;

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

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user: user::Model,
}

pub async fn register(
    Json(payload): Json<RegisterRequest>,
    db: Database,
) -> Result<Response, Error> {
    // Validate input
    payload.validate()?;

    // Check if user exists
    let existing = user::Entity::find()
        .filter(user::Column::Email.eq(&payload.email))
        .one(&db)
        .await?;

    if existing.is_some() {
        return Err(Error::BadRequest("Email already registered".into()));
    }

    // Hash password
    let password_hash = Hash::make(&payload.password)?;

    // Create user
    let user = user::ActiveModel {
        email: Set(payload.email),
        name: Set(payload.name),
        password: Set(password_hash),
        ..Default::default()
    };

    let user = user.insert(&db).await?;

    // Login user using Laravel-style Auth facade
    Auth::login(user.clone()).await?;

    Ok(Response::json(AuthResponse {
        message: "Registration successful".to_string(),
        user
    }))
}

pub async fn login(
    Json(payload): Json<LoginRequest>,
    db: Database,
) -> Result<Response, Error> {
    // Validate input
    payload.validate()?;

    // Find user
    let user = user::Entity::find()
        .filter(user::Column::Email.eq(&payload.email))
        .one(&db)
        .await?
        .ok_or_else(|| Error::Unauthorized("Invalid credentials".into()))?;

    // Verify password
    if !Hash::check(&payload.password, &user.password)? {
        return Err(Error::Unauthorized("Invalid credentials".into()));
    }

    // Login using Laravel-style Auth facade
    Auth::login(user.clone()).await?;

    Ok(Response::json(AuthResponse {
        message: "Login successful".to_string(),
        user
    }))
}

pub async fn logout() -> Result<Response, Error> {
    // Logout using Laravel-style Auth facade
    Auth::logout().await;

    Ok(Response::json(json!({ "message": "Logged out successfully" })))
}

pub async fn me() -> Result<Response, Error> {
    // Get current user using Auth facade
    if let Some(user) = Auth::user::<user::Model>().await {
        Ok(Response::json(user))
    } else {
        Err(Error::Unauthorized("Not authenticated".into()))
    }
}
```

### Post Controller

Create `src/controllers/post_controller.rs`:

```rust
use rf_http::{Request, Response, Json};
use rf_auth::AuthGuard;
use rf_validation::Validate;
use serde::{Deserialize, Serialize};
use crate::models::post;

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 3, max = 255))]
    pub title: String,

    #[validate(length(min = 10))]
    pub content: String,

    pub published: Option<bool>,
}

pub async fn index(db: Database) -> Result<Response, Error> {
    let posts = post::Entity::find()
        .filter(post::Column::Published.eq(true))
        .all(&db)
        .await?;

    Ok(Response::json(posts))
}

pub async fn show(
    Path(id): Path<i32>,
    db: Database,
) -> Result<Response, Error> {
    let post = post::Entity::find_by_id(id)
        .one(&db)
        .await?
        .ok_or_else(|| Error::NotFound("Post not found".into()))?;

    Ok(Response::json(post))
}

pub async fn store(
    auth: AuthGuard,
    Json(payload): Json<CreatePostRequest>,
    db: Database,
) -> Result<Response, Error> {
    // Validate input
    payload.validate()?;

    // Create post
    let post = post::ActiveModel {
        user_id: Set(auth.user_id()),
        title: Set(payload.title),
        content: Set(payload.content),
        published: Set(payload.published.unwrap_or(false)),
        ..Default::default()
    };

    let post = post.insert(&db).await?;

    Ok(Response::json(post).status(201))
}

pub async fn update(
    auth: AuthGuard,
    Path(id): Path<i32>,
    Json(payload): Json<CreatePostRequest>,
    db: Database,
) -> Result<Response, Error> {
    // Find post
    let post = post::Entity::find_by_id(id)
        .one(&db)
        .await?
        .ok_or_else(|| Error::NotFound("Post not found".into()))?;

    // Check ownership
    if post.user_id != auth.user_id() {
        return Err(Error::Forbidden("Not your post".into()));
    }

    // Update post
    let mut post: post::ActiveModel = post.into();
    post.title = Set(payload.title);
    post.content = Set(payload.content);
    post.published = Set(payload.published.unwrap_or(post.published.unwrap()));

    let post = post.update(&db).await?;

    Ok(Response::json(post))
}

pub async fn destroy(
    auth: AuthGuard,
    Path(id): Path<i32>,
    db: Database,
) -> Result<Response, Error> {
    // Find post
    let post = post::Entity::find_by_id(id)
        .one(&db)
        .await?
        .ok_or_else(|| Error::NotFound("Post not found".into()))?;

    // Check ownership
    if post.user_id != auth.user_id() {
        return Err(Error::Forbidden("Not your post".into()));
    }

    // Delete post
    post.delete(&db).await?;

    Ok(Response::no_content())
}
```

## Step 7: Set Up Routes

Edit `src/main.rs`:

```rust
mod models;
mod controllers;

use rf_core::Application;
use rf_route_facade::Route;
use rf_http::middleware;
use rf_orm::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv()?;

    // Initialize application
    let app = Application::new();

    // Connect to database
    let db = Database::connect(&std::env::var("DATABASE_URL")?).await?;

    // Public routes using Laravel-style Route facade
    Route::post("/auth/register", controllers::auth_controller::register);
    Route::post("/auth/login", controllers::auth_controller::login);

    // Post routes (public)
    Route::get("/posts", controllers::post_controller::index);
    Route::get("/posts/:id", controllers::post_controller::show);

    // Protected routes (require authentication)
    Route::middleware(&["auth"]).group(|| {
        Route::post("/auth/logout", controllers::auth_controller::logout);
        Route::get("/auth/me", controllers::auth_controller::me);

        Route::post("/posts", controllers::post_controller::store);
        Route::put("/posts/:id", controllers::post_controller::update);
        Route::delete("/posts/:id", controllers::post_controller::destroy);
    });

    // Start server
    println!("Server running at http://localhost:8000");
    app.serve(Route::router()).with_database(db).await?;

    Ok(())
}
```

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
