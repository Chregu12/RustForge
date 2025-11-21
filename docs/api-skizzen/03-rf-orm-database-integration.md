# API Sketch: rf-orm - Database Integration with SeaORM

**Component**: rf-orm
**Version**: 0.1.0
**Status**: Draft
**Date**: 2025-01-09

## Overview

Type-safe ORM integration using SeaORM with connection pooling, migrations, and query builder patterns. Provides Laravel-like Model and QueryBuilder APIs while leveraging SeaORM's compile-time safety.

## Goals

1. **Type-Safe Database Access**: Compile-time query validation via SeaORM entities
2. **Connection Pooling**: Managed database connections with configurable pool size
3. **Migration Support**: Schema versioning and rollback via sea-orm-migration
4. **Multiple Databases**: Support Postgres, MySQL, SQLite
5. **Query Builder**: Ergonomic query construction
6. **Transaction Support**: ACID transactions with rollback
7. **Soft Deletes**: Optional soft delete trait
8. **Pagination**: Built-in cursor and offset pagination

## Architecture

```
┌─────────────────────────────────────────┐
│          Application Code               │
└─────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│          rf-orm (Facade)                │
│  • DatabaseManager                      │
│  • Migration runner                     │
│  • Query builder helpers                │
└─────────────────────────────────────────┘
                  │
         ┌────────┴────────┐
         ▼                 ▼
┌──────────────┐   ┌──────────────┐
│   SeaORM     │   │  sqlx Pool   │
│  (Entities,  │   │ (Connection  │
│   Queries)   │   │  Management) │
└──────────────┘   └──────────────┘
         │                 │
         └────────┬────────┘
                  ▼
         ┌──────────────┐
         │   Database   │
         │ (Postgres,   │
         │ MySQL, SQLite)│
         └──────────────┘
```

## Core Components

### 1. DatabaseManager

Central database connection and configuration manager.

```rust
use rf_orm::{DatabaseManager, DatabaseConfig};

// Create manager from config
let config = DatabaseConfig {
    url: "postgres://localhost/myapp".into(),
    max_connections: 10,
    min_connections: 2,
    connect_timeout: Duration::from_secs(8),
    idle_timeout: Some(Duration::from_secs(600)),
    acquire_timeout: Duration::from_secs(30),
};

let db = DatabaseManager::connect(config).await?;

// Or from rf-config
let config = ConfigLoader::new().load::<AppConfig>()?;
let db = DatabaseManager::from_config(&config.database).await?;

// Get connection reference
let conn: &DatabaseConnection = db.connection();

// Health check
let is_healthy = db.ping().await.is_ok();

// Close gracefully
db.close().await?;
```

### 2. Entity Definition (SeaORM)

Define database models using SeaORM macros.

```rust
use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(unique)]
    pub email: String,

    pub name: String,
    pub password_hash: String,

    pub created_at: DateTime,
    pub updated_at: DateTime,

    #[sea_orm(nullable)]
    pub deleted_at: Option<DateTime>,
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

### 3. Query Builder

Ergonomic query construction with compile-time safety.

```rust
use rf_orm::prelude::*;
use entity::user::{self, Entity as User};

// Find by ID
let user = User::find_by_id(1)
    .one(db.connection())
    .await?;

// Find all
let users = User::find()
    .all(db.connection())
    .await?;

// Filter
let active_users = User::find()
    .filter(user::Column::DeletedAt.is_null())
    .filter(user::Column::Email.contains("@example.com"))
    .all(db.connection())
    .await?;

// Order and limit
let recent_users = User::find()
    .order_by_desc(user::Column::CreatedAt)
    .limit(10)
    .all(db.connection())
    .await?;

// Select specific columns
let emails = User::find()
    .select_only()
    .column(user::Column::Email)
    .into_tuple::<String>()
    .all(db.connection())
    .await?;

// Pagination (offset-based)
let page = User::find()
    .paginate(db.connection(), 20);  // 20 per page

let users = page.fetch_page(0).await?;  // Page 0
let total = page.num_items().await?;
let total_pages = page.num_pages().await?;
```

### 4. Insert, Update, Delete

CRUD operations with ActiveModel.

```rust
use entity::user::{self, ActiveModel, Entity as User};
use sea_orm::Set;

// Insert
let new_user = ActiveModel {
    email: Set("john@example.com".to_string()),
    name: Set("John Doe".to_string()),
    password_hash: Set("$2b$...".to_string()),
    created_at: Set(Utc::now()),
    updated_at: Set(Utc::now()),
    ..Default::default()
};

let result = User::insert(new_user)
    .exec(db.connection())
    .await?;

let user_id = result.last_insert_id;

// Update
let user = User::find_by_id(user_id)
    .one(db.connection())
    .await?
    .unwrap();

let mut user: ActiveModel = user.into();
user.name = Set("Jane Doe".to_string());
user.updated_at = Set(Utc::now());

let updated = user.update(db.connection()).await?;

// Delete
let result = User::delete_by_id(user_id)
    .exec(db.connection())
    .await?;

assert_eq!(result.rows_affected, 1);

// Delete many
let result = User::delete_many()
    .filter(user::Column::DeletedAt.is_not_null())
    .exec(db.connection())
    .await?;
```

### 5. Transactions

ACID transactions with automatic rollback on error.

```rust
use sea_orm::TransactionTrait;

// Manual transaction
let txn = db.connection().begin().await?;

let user = User::insert(new_user)
    .exec(&txn)
    .await?;

let post = Post::insert(new_post)
    .exec(&txn)
    .await?;

// Commit or rollback
if validation_ok {
    txn.commit().await?;
} else {
    txn.rollback().await?;
}

// Transaction closure (auto-rollback on error)
db.connection()
    .transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            let user = User::insert(new_user).exec(txn).await?;
            let post = Post::insert(new_post).exec(txn).await?;
            Ok(())
        })
    })
    .await?;
```

### 6. Relationships

Load related entities with eager/lazy loading.

```rust
// Eager loading (JOIN)
let users_with_posts = User::find()
    .find_with_related(Post)
    .all(db.connection())
    .await?;

for (user, posts) in users_with_posts {
    println!("User: {}, Posts: {}", user.name, posts.len());
}

// Lazy loading
let user = User::find_by_id(1)
    .one(db.connection())
    .await?
    .unwrap();

let posts = user.find_related(Post)
    .all(db.connection())
    .await?;

// Nested relationships
let users_with_posts_and_comments = User::find()
    .find_with_related(Post)
    .all(db.connection())
    .await?;
```

### 7. Raw SQL

Execute raw queries when needed.

```rust
use sea_orm::{FromQueryResult, Statement};

#[derive(Debug, FromQueryResult)]
struct UserCount {
    count: i64,
}

// Raw query
let result = UserCount::find_by_statement(
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT COUNT(*) as count FROM users WHERE created_at > $1",
        vec![start_date.into()],
    )
)
.one(db.connection())
.await?;

// Execute raw SQL
db.connection()
    .execute(Statement::from_string(
        DbBackend::Postgres,
        "CREATE INDEX idx_email ON users(email)".to_string(),
    ))
    .await?;
```

### 8. Migrations

Schema versioning with sea-orm-migration.

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
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(User::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(User::Name).string().not_null())
                    .col(ColumnDef::new(User::PasswordHash).string().not_null())
                    .col(ColumnDef::new(User::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(User::UpdatedAt).timestamp().not_null())
                    .col(ColumnDef::new(User::DeletedAt).timestamp())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum User {
    Table,
    Id,
    Email,
    Name,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
```

### 9. Migration Runner

CLI tool for running migrations.

```bash
# Run all pending migrations
cargo run --bin migrate up

# Rollback last migration
cargo run --bin migrate down

# Check migration status
cargo run --bin migrate status

# Reset database (rollback all, then up)
cargo run --bin migrate reset

# Fresh database (drop all, then up)
cargo run --bin migrate fresh
```

```rust
// Programmatic migration
use rf_orm::migration::Migrator;
use sea_orm_migration::MigratorTrait;

// Run migrations on startup
Migrator::up(db.connection(), None).await?;

// Rollback last batch
Migrator::down(db.connection(), Some(1)).await?;

// Get status
let status = Migrator::get_pending_migrations(db.connection()).await?;
```

### 10. Soft Deletes

Optional trait for soft delete support.

```rust
use rf_orm::SoftDelete;
use chrono::Utc;

impl SoftDelete for user::ActiveModel {
    fn soft_delete(&mut self) {
        self.deleted_at = Set(Some(Utc::now()));
    }

    fn restore(&mut self) {
        self.deleted_at = Set(None);
    }
}

// Soft delete
let mut user = user.into_active_model();
user.soft_delete();
user.update(db.connection()).await?;

// Query excluding soft-deleted
let active_users = User::find()
    .filter(user::Column::DeletedAt.is_null())
    .all(db.connection())
    .await?;

// Query only soft-deleted
let deleted_users = User::find()
    .filter(user::Column::DeletedAt.is_not_null())
    .all(db.connection())
    .await?;

// Restore
let mut user = user.into_active_model();
user.restore();
user.update(db.connection()).await?;

// Force delete (permanent)
User::delete_by_id(user_id)
    .exec(db.connection())
    .await?;
```

## Configuration

### Database Config Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "postgres://user:pass@localhost/db")
    pub url: String,

    /// Maximum number of connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of connections in pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// Idle connection timeout in seconds (None = no timeout)
    #[serde(default)]
    pub idle_timeout: Option<u64>,

    /// Acquire connection timeout in seconds
    #[serde(default = "default_acquire_timeout")]
    pub acquire_timeout: u64,

    /// Enable SQL query logging
    #[serde(default)]
    pub log_queries: bool,

    /// SQL log level (off, error, warn, info, debug, trace)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}
```

### TOML Configuration

```toml
[database]
url = "postgres://localhost/myapp"
max_connections = 20
min_connections = 5
connect_timeout = 8
idle_timeout = 600
acquire_timeout = 30
log_queries = true
log_level = "debug"
```

### Environment Variables

```bash
APP__DATABASE__URL="postgres://user:pass@localhost/myapp"
APP__DATABASE__MAX_CONNECTIONS=20
APP__DATABASE__LOG_QUERIES=true
```

## Error Handling

```rust
use rf_orm::DbError;

pub enum DbError {
    /// Database connection error
    ConnectionFailed { source: DbErr },

    /// Query execution error
    QueryFailed { query: String, source: DbErr },

    /// Entity not found
    NotFound { entity: String, id: String },

    /// Unique constraint violation
    UniqueViolation { field: String, value: String },

    /// Foreign key constraint violation
    ForeignKeyViolation { table: String, key: String },

    /// Transaction error
    TransactionFailed { source: DbErr },

    /// Migration error
    MigrationFailed { migration: String, source: DbErr },
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound { .. } => AppError::NotFound { /* ... */ },
            DbError::UniqueViolation { .. } => AppError::Conflict { /* ... */ },
            _ => AppError::Internal(err.into()),
        }
    }
}
```

## Testing

### Test Utilities

```rust
use rf_orm::testing::{TestDatabase, TestTransaction};

#[tokio::test]
async fn test_user_creation() {
    // Create test database (SQLite in-memory)
    let test_db = TestDatabase::new().await;

    // Run migrations
    test_db.migrate().await.unwrap();

    // Create user
    let user = create_user(&test_db, "test@example.com").await.unwrap();

    // Assert
    assert_eq!(user.email, "test@example.com");

    // Cleanup automatic (in-memory database dropped)
}

#[tokio::test]
async fn test_transaction_rollback() {
    let test_db = TestDatabase::new().await;
    test_db.migrate().await.unwrap();

    // Test transaction (auto-rollback)
    let txn = TestTransaction::new(&test_db).await;

    create_user(txn.connection(), "test@example.com").await.unwrap();

    // Transaction rolls back when txn is dropped
    drop(txn);

    // Verify user doesn't exist
    let users = User::find().all(test_db.connection()).await.unwrap();
    assert_eq!(users.len(), 0);
}
```

### Mock Database

```rust
use rf_orm::mock::MockDatabase;

#[tokio::test]
async fn test_repository_with_mock() {
    let mut mock_db = MockDatabase::new();

    // Setup expectations
    mock_db
        .expect_query::<User>()
        .with_id(1)
        .returning(Ok(Some(mock_user())));

    // Test repository
    let repo = UserRepository::new(&mock_db);
    let user = repo.find_by_id(1).await.unwrap();

    assert_eq!(user.id, 1);
    mock_db.verify();
}
```

## Performance Considerations

### Connection Pooling

- **Pool Size**: Default 10 connections (configurable)
- **Idle Timeout**: 10 minutes default
- **Acquire Timeout**: 30 seconds default
- **Min Connections**: Maintain 2 warm connections

### Query Optimization

```rust
// ❌ N+1 Query Problem
let users = User::find().all(db).await?;
for user in users {
    let posts = user.find_related(Post).all(db).await?; // N queries!
}

// ✅ Eager Loading (1 query)
let users_with_posts = User::find()
    .find_with_related(Post)
    .all(db)
    .await?;

// ✅ Select Only Needed Columns
let user_emails = User::find()
    .select_only()
    .column(user::Column::Email)
    .into_tuple::<String>()
    .all(db)
    .await?;

// ✅ Use Pagination for Large Sets
let paginator = User::find().paginate(db, 100);
let page = paginator.fetch_page(0).await?;
```

### Indexing

```rust
// Migration with indexes
manager.create_index(
    Index::create()
        .name("idx_users_email")
        .table(User::Table)
        .col(User::Email)
        .to_owned()
).await?;

// Composite index
manager.create_index(
    Index::create()
        .name("idx_posts_user_created")
        .table(Post::Table)
        .col(Post::UserId)
        .col(Post::CreatedAt)
        .to_owned()
).await?;
```

## Security

### SQL Injection Prevention

SeaORM uses parameterized queries by default:

```rust
// ✅ Safe (parameterized)
let users = User::find()
    .filter(user::Column::Email.eq(user_input))
    .all(db)
    .await?;

// ❌ Unsafe (only use for trusted input)
let users = User::find_by_statement(
    Statement::from_string(
        DbBackend::Postgres,
        format!("SELECT * FROM users WHERE email = '{}'", user_input)
    )
)
.all(db)
.await?;
```

### Connection Security

```toml
[database]
# Use SSL for production
url = "postgres://user:pass@localhost/db?sslmode=require"

# Or via environment
APP__DATABASE__URL="postgres://user:pass@localhost/db?sslmode=require"
```

## Integration with rf-core

```rust
use rf_core::AppError;
use rf_orm::DbError;

// Convert DbError to AppError
impl From<DbErr> for AppError {
    fn from(err: DbErr) -> Self {
        match err {
            DbErr::RecordNotFound(_) => AppError::NotFound {
                resource: "Entity".to_string(),
            },
            DbErr::Conn(_) => AppError::ServiceUnavailable {
                service: "database".to_string(),
            },
            _ => AppError::Internal(err.into()),
        }
    }
}
```

## Example: Complete CRUD API

```rust
use axum::{extract::Path, Json, Extension};
use rf_core::{AppResult, AppError};
use rf_orm::DatabaseManager;

// Create
async fn create_user(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Json(data): Json<CreateUserDto>,
) -> AppResult<Json<UserResponse>> {
    let user = ActiveModel {
        email: Set(data.email),
        name: Set(data.name),
        password_hash: Set(hash_password(&data.password)?),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };

    let result = User::insert(user)
        .exec(db.connection())
        .await?;

    let user = User::find_by_id(result.last_insert_id)
        .one(db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound { resource: "User".into() })?;

    Ok(Json(UserResponse::from(user)))
}

// Read
async fn get_user(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Path(id): Path<i32>,
) -> AppResult<Json<UserResponse>> {
    let user = User::find_by_id(id)
        .one(db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound { resource: "User".into() })?;

    Ok(Json(UserResponse::from(user)))
}

// Update
async fn update_user(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateUserDto>,
) -> AppResult<Json<UserResponse>> {
    let user = User::find_by_id(id)
        .one(db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound { resource: "User".into() })?;

    let mut user: ActiveModel = user.into();

    if let Some(name) = data.name {
        user.name = Set(name);
    }
    if let Some(email) = data.email {
        user.email = Set(email);
    }
    user.updated_at = Set(Utc::now());

    let user = user.update(db.connection()).await?;

    Ok(Json(UserResponse::from(user)))
}

// Delete
async fn delete_user(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Path(id): Path<i32>,
) -> AppResult<()> {
    let result = User::delete_by_id(id)
        .exec(db.connection())
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::NotFound { resource: "User".into() });
    }

    Ok(())
}

// List with pagination
async fn list_users(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<UserResponse>>> {
    let page = params.page.unwrap_or(0);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let paginator = User::find()
        .filter(user::Column::DeletedAt.is_null())
        .order_by_desc(user::Column::CreatedAt)
        .paginate(db.connection(), per_page);

    let users = paginator.fetch_page(page).await?;
    let total = paginator.num_items().await?;

    Ok(Json(PaginatedResponse {
        data: users.into_iter().map(UserResponse::from).collect(),
        page,
        per_page,
        total,
        total_pages: paginator.num_pages().await?,
    }))
}
```

## Summary

rf-orm provides:
- ✅ Type-safe database access via SeaORM
- ✅ Connection pooling with sqlx
- ✅ Migration support
- ✅ Transaction support
- ✅ Relationship loading
- ✅ Pagination helpers
- ✅ Soft delete trait
- ✅ Testing utilities
- ✅ Integration with rf-core and rf-config

Next: Implementation in `crates/rf-orm/`
