//! # rf-orm: Laravel-Inspired Type-Safe Database ORM
//!
//! A Laravel-Eloquent-like ORM built on top of SeaORM with:
//! - Laravel-style Query Builder API
//! - Eloquent-style relationships (BelongsTo, HasMany, etc.)
//! - Model events (creating, created, updating, etc.)
//! - Transaction support with automatic rollback
//! - Eager loading to prevent N+1 queries
//! - Connection pooling and management
//! - Soft delete trait
//! - Migration support with advanced features
//! - Database sharding for horizontal scaling
//! - Testing utilities
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_orm::prelude::*;
//! use sea_orm::entity::prelude::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to database
//! let db = DatabaseManager::connect(DatabaseConfig {
//!     url: "sqlite::memory:".to_string(),
//!     ..Default::default()
//! }).await?;
//!
//! // Laravel-style query
//! let posts = Post::query(db.connection().clone())
//!     .where_eq(post::Column::Published, true)
//!     .where_gt(post::Column::Views, 100)
//!     .order_by_desc(post::Column::CreatedAt)
//!     .limit(10)
//!     .get()
//!     .await?;
//!
//! // Relationships
//! for post in &posts {
//!     let author = post.belongs_to::<User>(db.connection()).await?;
//!     let comments = post.has_many::<Comment>(db.connection()).await?;
//! }
//!
//! // Transactions
//! db.connection().transaction(|tx| async move {
//!     User::create(tx, user_data).await?;
//!     Profile::create(tx, profile_data).await?;
//!     Ok(())
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! ### Query Builder
//!
//! ```rust,no_run
//! # use rf_orm::prelude::*;
//! # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
//! // Laravel-style method chaining
//! let users = User::query(db)
//!     .where_eq(user::Column::Active, true)
//!     .where_like(user::Column::Name, "%John%")
//!     .order_by("created_at", "desc")
//!     .paginate(1, 15)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Relationships
//!
//! ```rust,no_run
//! # use rf_orm::prelude::*;
//! # async fn example(db: &DatabaseConnection, post: &Post) -> Result<(), Box<dyn std::error::Error>> {
//! // BelongsTo
//! let author = post.belongs_to::<User>(db).await?;
//!
//! // HasMany
//! let posts = user.has_many::<Post>(db).await?;
//!
//! // HasOne
//! let profile = user.has_one::<Profile>(db).await?;
//!
//! // BelongsToMany (many-to-many)
//! let tags = post.belongs_to_many::<Tag>(db).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Model Events
//!
//! ```rust,no_run
//! # use rf_orm::prelude::*;
//! # use async_trait::async_trait;
//! #[async_trait]
//! impl ModelEvents for post::ActiveModel {
//!     async fn before_create(&mut self) -> EventResult {
//!         // Auto-generate slug
//!         self.slug = Set(slugify(&self.title));
//!         Ok(())
//!     }
//!
//!     async fn after_create(&self) -> EventResult {
//!         // Send notification
//!         notify_new_post(self).await?;
//!         Ok(())
//!     }
//! }
//! ```

pub mod collection;
pub mod config;
pub mod error;
pub mod events;
pub mod manager;
pub mod migrations;
pub mod model;
pub mod polymorphic;
pub mod query;
pub mod query_builder;
pub mod relationships;
pub mod schema_builder;
pub mod scopes;
pub mod soft_delete;
pub mod transaction;

// Advanced features
pub mod advanced_migrations;
pub mod sharding;

// Performance optimization modules
pub mod pool_optimizer;
// pub mod query_cache; // Temporarily disabled - requires rf-cache API migration

#[cfg(test)]
pub mod testing;

// Re-exports
pub use collection::{Collection, IntoCollection};
pub use config::DatabaseConfig;
pub use error::{DbError, DbResult};
pub use events::{EventObserver, ModelEvent, ModelEvents};
pub use manager::DatabaseManager;
pub use migrations::{
    BatchResult, Migration, MigrationError, MigrationResult, MigrationStatus, Migrator,
    SchemaContext,
};
pub use model::Model;
pub use polymorphic::{
    morph_many, morph_one, morph_to, MorphMany, MorphOne, MorphTo, MorphToMany, Morphable,
    PolymorphicQueryBuilder, PolymorphicResult,
};
pub use query::{
    aggregations::{Aggregate, AggregateType, AggregationBuilder, WithAggregates},
    subquery::{Subquery, SubqueryBuilder},
};
pub use query_builder::QueryBuilder;
pub use relationships::{
    basic::{eager_load, RelationshipHelpers},
    loading::{CollectionExt, EagerLoad, LazyLoad, LoadResult, SupportsEagerLoading},
    morph_to_many::{attach_morph, detach_morph, sync_morph, toggle_morph, MorphToManyResult},
    through::{has_many_through, has_one_through, HasManyThrough, HasOneThrough, ThroughResult},
};
pub use schema_builder::{Blueprint, Column, ColumnType, DatabaseType, ForeignKey, Index, Schema};
pub use scopes::{HasScopes, ScopeExt, ScopeFn, ScopeRegistry};
pub use soft_delete::SoftDelete;
pub use transaction::{IsolationLevel, IsolationLevelExt, Savepoint, Transaction, TransactionExt};

// Advanced features
pub use advanced_migrations::{
    AdvancedMigrationBuilder, AdvancedMigrationError, AdvancedMigrationResult, ForeignKeyAction,
};
pub use sharding::{
    manager::{ShardError, ShardManager, ShardResult, ShardStrategy},
    strategies::{GeographicStrategy, HashStrategy, RangeStrategy, TenantStrategy},
};

// Re-export SeaORM types for convenience
pub use sea_orm::{
    self, ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait,
    DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait, IntoActiveModel, ModelTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Related, RelationTrait, Set,
};

// Re-export model macros for Laravel-like syntax
pub use rf_model_macro::{model, relations};

// Re-export commonly used types
pub mod prelude {
    pub use super::{
        eager_load, morph_many, morph_one, morph_to, AdvancedMigrationBuilder,
        AdvancedMigrationError, AdvancedMigrationResult, BatchResult, Blueprint, Collection,
        Column, ColumnType, DatabaseConfig, DatabaseManager, DatabaseType, DbError, DbResult,
        EventObserver, ForeignKey, ForeignKeyAction, GeographicStrategy, HasScopes, HashStrategy,
        Index, IntoCollection, IsolationLevel, IsolationLevelExt, Migration, MigrationError,
        MigrationResult, MigrationStatus, Migrator, Model, ModelEvent, ModelEvents, MorphMany,
        MorphOne, MorphTo, MorphToMany, Morphable, PolymorphicQueryBuilder, PolymorphicResult,
        QueryBuilder, RangeStrategy, RelationshipHelpers, Savepoint, Schema, SchemaContext,
        ScopeExt, ScopeFn, ScopeRegistry, ShardError, ShardManager, ShardResult, ShardStrategy,
        SoftDelete, TenantStrategy, Transaction, TransactionExt,
    };
    pub use sea_orm::{
        ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection,
        DatabaseTransaction, DbErr, EntityTrait, IntoActiveModel, ModelTrait, PaginatorTrait,
        QueryFilter, QueryOrder, QuerySelect, QueryTrait, Related, RelationTrait, Set,
    };
    // Re-export model macros
    pub use rf_model_macro::{model, relations};
}
