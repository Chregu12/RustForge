//! # rf-eloquent: Laravel Eloquent-Style ORM for RustForge
//!
//! A comprehensive ORM layer that brings Laravel Eloquent's powerful features to Rust.
//! Built on top of SeaORM with additional features for relationships, eager loading,
//! attribute casting, accessors/mutators, and model events.
//!
//! ## Features
//!
//! - **Relationships**: BelongsTo, HasOne, HasMany, BelongsToMany, HasOneThrough, HasManyThrough
//! - **Eager Loading**: Prevent N+1 queries with automatic relationship loading
//! - **Attribute Casting**: Automatic type conversion (JSON, DateTime, Encrypted, etc.)
//! - **Accessors & Mutators**: Transform data on get/set operations
//! - **Model Events**: Lifecycle hooks (creating, created, updating, updated, etc.)
//! - **Type-Safe**: Full Rust type safety with compile-time guarantees
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_eloquent::prelude::*;
//! use async_trait::async_trait;
//!
//! #[derive(Clone, Debug)]
//! struct User {
//!     id: i64,
//!     name: String,
//!     email: String,
//! }
//!
//! #[derive(Clone, Debug)]
//! struct Post {
//!     id: i64,
//!     user_id: i64,
//!     title: String,
//! }
//!
//! # async fn example(db: &sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
//! // Relationships
//! // let user = User::find(1).await?;
//! // let posts = user.has_many::<Post, _>(db, "user_id").get().await?;
//!
//! // Eager loading
//! // let users = User::query()
//! //     .with("posts")
//! //     .with("profile")
//! //     .get()
//! //     .await?;
//!
//! # Ok(())
//! # }
//!
//! #[async_trait]
//! impl ModelEvents for User {
//!     async fn creating(&mut self) -> EventResult {
//!         // Auto-generate timestamps
//!         Ok(())
//!     }
//! }
//!
//! impl HasCasts for User {
//!     fn casts() -> CastRegistry {
//!         CastRegistry::new()
//!             .cast("created_at", CastType::DateTime)
//!     }
//! }
//! ```
//!
//! ## Relationship Examples
//!
//! ### One-to-Many (Has Many)
//!
//! ```rust,no_run
//! # use rf_eloquent::prelude::*;
//! # use rf_eloquent::relationships::has_many_builder;
//! # use sea_orm::Order;
//! # fn main() {}
//! # mod post {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "posts")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32, pub published: bool }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # async fn example(user_id: i32, db: sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
//! // User has many Posts (fluent builder)
//! let posts = has_many_builder::<post::Entity, _>(db, post::Column::UserId, user_id)
//!     .order_by(post::Column::Id, Order::Desc)
//!     .limit(10)
//!     .get()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Belongs To (Inverse)
//!
//! ```rust,no_run
//! # use rf_eloquent::prelude::*;
//! # use rf_eloquent::belongs_to;
//! # fn main() {}
//! # mod user {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "users")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # async fn example(post_user_id: i32, db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
//! // Post belongs to User
//! let author = belongs_to::<user::Entity, user::Model, _>(db, post_user_id, user::Column::Id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Many-to-Many (Belongs To Many)
//!
//! ```rust,no_run
//! # use rf_eloquent::prelude::*;
//! # use rf_eloquent::belongs_to_many;
//! # fn main() {}
//! # mod tag {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "tags")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # mod post_tag {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "post_tag")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub post_id: i32, #[sea_orm(primary_key)] pub tag_id: i32 }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # async fn example(post_id: i32, db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
//! // Post belongs to many Tags (through the `post_tag` pivot table)
//! let tags = belongs_to_many::<tag::Entity, post_tag::Entity, tag::Model, _>(
//!     db,
//!     post_id,
//!     post_tag::Column::PostId,   // foreign key
//!     post_tag::Column::TagId,    // related key
//!     tag::Column::Id,
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Eager Loading Examples
//!
//! ```rust,no_run
//! # use rf_eloquent::prelude::*;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # struct User;
//! // Load single relationship
//! // let users = User::query().with("posts").get().await?;
//!
//! // Load nested relationships
//! // let users = User::query()
//! //     .with("posts.comments")
//! //     .get()
//! //     .await?;
//!
//! // Load multiple relationships
//! // let users = User::query()
//! //     .with_all(&["posts", "profile", "roles"])
//! //     .get()
//! //     .await?;
//!
//! // Conditional eager loading
//! // let users = User::query()
//! //     .with_where("posts", |q| q.where_("published", true))
//! //     .get()
//! //     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Attribute Casting
//!
//! ```rust,no_run
//! # use rf_eloquent::prelude::*;
//! struct Post {
//!     id: i64,
//!     metadata: serde_json::Value,
//!     published_at: chrono::DateTime<chrono::Utc>,
//! }
//!
//! impl HasCasts for Post {
//!     fn casts() -> CastRegistry {
//!         CastRegistry::new()
//!             .cast("metadata", CastType::Json)
//!             .cast("published_at", CastType::DateTime)
//!     }
//! }
//! ```
//!
//! ## Model Events
//!
//! ```rust,no_run
//! # use rf_eloquent::prelude::*;
//! # use async_trait::async_trait;
//! struct User {
//!     id: i64,
//!     email: String,
//! }
//!
//! #[async_trait]
//! impl ModelEvents for User {
//!     async fn creating(&mut self) -> EventResult {
//!         // Validate before creating
//!         if self.email.is_empty() {
//!             return Err(EventError::ValidationFailed("Email required".into()));
//!         }
//!         Ok(())
//!     }
//!
//!     async fn created(&self) -> EventResult {
//!         // Send welcome email after creation
//!         // send_welcome_email(&self.email).await?;
//!         Ok(())
//!     }
//! }
//! ```

pub mod accessors;
pub mod casting;
pub mod observer;
pub mod eager_loading;
pub mod eager_loading_impl;
pub mod eager_loading_optimized;
pub mod events;
pub mod polymorphic_impl;
pub mod query_helpers;
pub mod relationships;
pub mod scopes;
pub mod soft_deletes;

// Phase 19: Automatic eager loading detection
pub mod auto_eager_load;

// Polymorphic relationships submodule
pub use polymorphic_impl as polymorphic;

// Re-exports for convenience
pub use accessors::{
    common_accessors, common_mutators, AttributeBag, AttributeError, AttributeResult,
    AttributeValue, HasAccessors, HasMutators,
};
pub use casting::{
    cast_value, register_caster, uncast_value, Castable, CastError, CastRegistry, CastResult,
    CastType, CastedValue, CustomCasterRegistry, HasCasts,
};
pub use eager_loading::{
    EagerLoadBuilder, EagerLoadError, EagerLoadRelation, EagerLoadResult, EagerLoadStats,
    EagerLoadable, EagerLoader, RelationshipCache, RelationshipLoader, WithEagerLoad,
};
pub use eager_loading_impl::{ConcreteEagerLoader, GroupBy, GroupedModels};
pub use events::{
    EventContext, EventDispatcher, EventError, EventListener, EventObserver, EventResult,
    ModelEvent, ModelEvents,
};
pub use observer::{dispatch_observers, observe, Observer, ObserverRegistry, GLOBAL_OBSERVERS};
pub use query_helpers::{
    attach, belongs_to, belongs_to_many, detach, has_many, has_many_through, has_one, sync,
};
pub use relationships::{
    belongs_to_builder, has_many_builder, has_one_builder, BelongsTo, BelongsToBuilder,
    BelongsToMany, HasMany, HasManyBuilder, HasManyThrough, HasOne, HasOneBuilder, HasOneThrough,
    HasRelationships, RelationshipError, RelationshipKind, RelationshipResult,
};
pub use scopes::{
    add_global_scope, apply_global_scopes, remove_global_scope, without_global_scopes,
    CommonScopes, GlobalScopeRegistry, HasScopes, ScopeBuilder, ScopeError, ScopeResult,
    ScopedQuery,
};
pub use soft_deletes::{
    clear_deleted_at, set_deleted_at, ForceDelete, SoftDeleteEntity, SoftDeleteScope, SoftDeletes,
};

// Re-export polymorphic relationships
pub mod polymorphic_relationships {
    pub use crate::polymorphic_impl::{
        morph_many::{MorphMany, MorphManyBuilder},
        morph_one::{MorphOne, MorphOneBuilder},
        morph_to::{HasMorphTo, MorphTo},
        morph_to_many::{MorphToMany, MorphToManyBuilder},
        morphed_by_many::{MorphedByMany, MorphedByManyBuilder},
        polymorphic::{
            MorphableModel, MorphableMutator, Polymorphic, PolymorphicError, PolymorphicRelation,
            PolymorphicResult,
        },
        type_registry::{TypeRegistry, TypeResolver, GLOBAL_TYPE_REGISTRY},
    };
}

// Re-export commonly used types from SeaORM
pub use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    ModelTrait, QueryFilter, QueryOrder, QuerySelect,
};

/// Prelude module with commonly used types
pub mod prelude {
    pub use super::accessors::{
        common_accessors, common_mutators, AttributeBag, AttributeError, AttributeResult,
        AttributeValue, HasAccessors, HasMutators,
    };
    pub use super::casting::{
        cast_value, register_caster, uncast_value, Castable, CastError, CastRegistry, CastResult,
        CastType, CastedValue, CustomCasterRegistry, HasCasts,
    };
    pub use super::eager_loading::{
        EagerLoadBuilder, EagerLoadError, EagerLoadRelation, EagerLoadResult, EagerLoadStats,
        EagerLoadable, EagerLoader, RelationshipCache, RelationshipLoader, WithEagerLoad,
    };
    pub use super::eager_loading_impl::{ConcreteEagerLoader, GroupBy, GroupedModels};
    pub use super::events::{
        EventContext, EventDispatcher, EventError, EventListener, EventObserver, EventResult,
        ModelEvent, ModelEvents,
    };
    pub use super::query_helpers::{
        attach, belongs_to, belongs_to_many, detach, has_many, has_many_through, has_one, sync,
    };
    pub use super::relationships::{
        belongs_to_builder, has_many_builder, has_one_builder, BelongsTo, BelongsToBuilder,
        BelongsToMany, HasMany, HasManyBuilder, HasManyThrough, HasOne, HasOneBuilder,
        HasOneThrough, HasRelationships, RelationshipError, RelationshipKind, RelationshipResult,
    };
    pub use super::scopes::{
        add_global_scope, apply_global_scopes, remove_global_scope, without_global_scopes,
        CommonScopes, GlobalScopeRegistry, HasScopes, ScopeBuilder, ScopeError, ScopeResult,
        ScopedQuery,
    };
    pub use super::soft_deletes::{
        clear_deleted_at, set_deleted_at, ForceDelete, SoftDeleteEntity, SoftDeleteScope,
        SoftDeletes,
    };

    // Polymorphic relationships
    pub use super::polymorphic_relationships::*;

    // Automatic eager loading detection
    pub use super::auto_eager_load::{NPlusOnePattern, QueryLog, QueryStats, QueryTracker};

    // SeaORM re-exports
    pub use sea_orm::{
        ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
        ModelTrait, QueryFilter, QueryOrder, QuerySelect,
    };
}

/// Result type used throughout the crate
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        // This test just ensures the prelude module compiles
        use prelude::*;

        // Create some basic types to verify imports work
        let _registry = CastRegistry::new();
        let _bag = AttributeBag::new();
        let _stats = EagerLoadStats::new();
    }
}
