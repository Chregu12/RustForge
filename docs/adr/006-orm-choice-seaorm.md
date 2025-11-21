# ADR-006: ORM Choice (SeaORM)

**Status:** Accepted
**Date:** 2025-11-08
**Deciders:** Lead Architect

## Context

Für produktionsreife APIs benötigen wir:
- Type-Safe Query Builder
- Async/Await Support
- Migrations & Schema Management
- Relationship Handling (1:N, N:M)
- Connection Pooling

## Decision

**SeaORM** als ORM-Layer

### Begründung:

**SeaORM:**
- ✅ Async-first (Tokio/async-std)
- ✅ Active Record + Query Builder Pattern
- ✅ Compile-Time Query Validation
- ✅ Migrations via `sea-orm-migration`
- ✅ Multi-DB-Support (Postgres, MySQL, SQLite)
- ✅ Aktive Entwicklung (SeaQL-Team)

### API-Beispiel:

```rust
use sea_orm::entity::prelude::*;

// Entity Definition
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::post::Entity")]
    Post,
}

// Query Examples
let users = User::find()
    .filter(user::Column::Email.contains("example.com"))
    .order_by_asc(user::Column::CreatedAt)
    .all(&db)
    .await?;

let user = User::find_by_id(1)
    .one(&db)
    .await?
    .ok_or(AppError::NotFound)?;

// Insert
let new_user = user::ActiveModel {
    email: Set("user@example.com".to_owned()),
    password_hash: Set(hash_password("secret")?),
    ..Default::default()
};
let user = new_user.insert(&db).await?;

// Update
let mut user: user::ActiveModel = user.into();
user.email = Set("newemail@example.com".to_owned());
user.update(&db).await?;
```

### Alternativen (abgelehnt):

**Diesel:**
- ❌ Kein async Support (blocking)
- ❌ Makro-heavy, lange Compile-Zeiten
- ✅ Mature, große Community

**sqlx:**
- ❌ Kein ORM, nur Query Builder
- ❌ Manuelle Relationship-Handling
- ✅ Compile-time checked SQL (via macros)

**ORMLite:**
- ❌ Weniger Features
- ❌ Kleinere Community

## Consequences

**Positiv:**
- ✅ Type-Safe Queries
- ✅ Async Performance
- ✅ Laravel-ähnliche API (Active Record)
- ✅ Migrations integriert

**Negativ:**
- ❌ Jüngeres Projekt (weniger battle-tested als Diesel)
- ❌ Breaking Changes in frühen Versionen
- ❌ Weniger 3rd-Party-Plugins

## Implementation

### Migration Setup:

```rust
// migration/src/lib.rs
use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_users_table::Migration),
            Box::new(m20240101_000002_create_posts_table::Migration),
        ]
    }
}

// migration/src/m20240101_000001_create_users_table.rs
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
                    .col(ColumnDef::new(User::PasswordHash).string().not_null())
                    .col(
                        ColumnDef::new(User::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(User::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
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

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Email,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
}
```

### Connection Pool:

```rust
// rf-database/src/lib.rs
use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<DatabaseConnection> {
    let mut opt = ConnectOptions::new(database_url.to_owned());
    opt.max_connections(max_connections)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Debug);

    Database::connect(opt).await
}
```

### Repository Pattern:

```rust
// rf-database/src/repository.rs
use async_trait::async_trait;
use sea_orm::*;

#[async_trait]
pub trait Repository<T: EntityTrait> {
    async fn find_by_id(&self, id: i32) -> Result<Option<T::Model>, DbErr>;
    async fn find_all(&self) -> Result<Vec<T::Model>, DbErr>;
    async fn create(&self, model: T::ActiveModel) -> Result<T::Model, DbErr>;
    async fn update(&self, model: T::ActiveModel) -> Result<T::Model, DbErr>;
    async fn delete(&self, id: i32) -> Result<DeleteResult, DbErr>;
}

pub struct UserRepository {
    db: DatabaseConnection,
}

#[async_trait]
impl Repository<user::Entity> for UserRepository {
    async fn find_by_id(&self, id: i32) -> Result<Option<user::Model>, DbErr> {
        user::Entity::find_by_id(id).one(&self.db).await
    }

    async fn find_all(&self) -> Result<Vec<user::Model>, DbErr> {
        user::Entity::find().all(&self.db).await
    }

    async fn create(&self, model: user::ActiveModel) -> Result<user::Model, DbErr> {
        model.insert(&self.db).await
    }

    // ...
}
```
