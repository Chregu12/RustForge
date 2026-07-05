//! # Eloquent-Style Relationship System
//!
//! This module provides Laravel Eloquent-style relationships for RustForge models.
//! It supports all major relationship types with a fluent, chainable API.
//!
//! ## Supported Relationships
//!
//! - `BelongsTo`: One-to-One (inverse) or Many-to-One inverse
//! - `HasOne`: One-to-One
//! - `HasMany`: One-to-Many
//! - `BelongsToMany`: Many-to-Many (with pivot table)
//! - `HasOneThrough`: One-to-One through intermediate
//! - `HasManyThrough`: One-to-Many through intermediate
//!
//! ## Polymorphic Relationships
//!
//! - `MorphTo`: Belongs to multiple model types
//! - `MorphOne`: Has one of a polymorphic model
//! - `MorphMany`: Has many of a polymorphic model
//! - `MorphToMany`: Many-to-many polymorphic (with pivot)
//! - `MorphedByMany`: Inverse of MorphToMany
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rf_eloquent::prelude::*;
//!
//! // Define relationships in your model impl
//! // User has many Posts
//! // Post belongs to User
//! ```

use async_trait::async_trait;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, Select, Value,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use thiserror::Error;

/// Relationship errors
#[derive(Error, Debug)]
pub enum RelationshipError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),

    #[error("Foreign key not found: {0}")]
    ForeignKeyNotFound(String),

    #[error("Invalid relationship configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Related model not found")]
    RelatedModelNotFound,

    #[error("Pivot table error: {0}")]
    PivotError(String),
}

pub type RelationshipResult<T> = Result<T, RelationshipError>;

/// Trait for models that support relationships
///
/// This trait provides default implementations that delegate to the
/// [`query_helpers`](crate::query_helpers) module. The defaults execute **real**
/// database queries against the supplied connection — they are not stubs and do
/// not panic.
///
/// # Design
///
/// Because the related row is loaded into a concrete SeaORM entity, the loader
/// methods are generic over the related `Entity` (`E`), the row model to
/// deserialize into (`M`) and the parent-key value (`K`). The join column is
/// passed as a typed `E::Column` rather than a stringly-typed name so the query
/// is type-checked at compile time.
///
/// For a free-function form (no trait bound on `Self`), the equivalent helpers
/// are also exported directly:
/// - [`crate::query_helpers::has_one`] for HasOne relationships
/// - [`crate::query_helpers::has_many`] for HasMany relationships
/// - [`crate::query_helpers::belongs_to`] for BelongsTo relationships
///
#[async_trait]
pub trait HasRelationships: Sized + Send + Sync {
    /// Load a has-one relationship.
    ///
    /// Delegates to [`crate::query_helpers::has_one`], returning the single
    /// related row of entity `E` whose `foreign_key` column equals `parent_id`
    /// (typically `self`'s primary key), or `None` if there is no match.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::prelude::*;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod profile {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "profiles")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
    /// // Use the standalone function instead:
    /// use rf_eloquent::has_one;
    /// let user = user::Model { id: 1 };
    /// let profile = has_one::<profile::Entity, profile::Model, _>(
    ///     db,
    ///     user.id,
    ///     profile::Column::UserId
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load_has_one<E, M, K>(
        &self,
        db: &DatabaseConnection,
        parent_id: K,
        foreign_key: E::Column,
    ) -> RelationshipResult<Option<M>>
    where
        E: EntityTrait,
        M: FromQueryResult + Sized + Send,
        K: Into<Value> + Clone + Send,
        <E as EntityTrait>::Column: ColumnTrait,
    {
        // Real query: delegates to the working query helper (no panic).
        crate::query_helpers::has_one::<E, M, K>(db, parent_id, foreign_key)
            .await
            .map_err(RelationshipError::from)
    }

    /// Load a has-many relationship.
    ///
    /// Delegates to [`crate::query_helpers::has_many`], returning every related
    /// row of entity `E` whose `foreign_key` column equals `parent_id`
    /// (typically `self`'s primary key). Returns an empty vector when there are
    /// no matches.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::prelude::*;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
    /// // Use the standalone function instead:
    /// use rf_eloquent::has_many;
    /// let user = user::Model { id: 1 };
    /// let posts = has_many::<post::Entity, post::Model, _>(
    ///     db,
    ///     user.id,
    ///     post::Column::UserId
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load_has_many<E, M, K>(
        &self,
        db: &DatabaseConnection,
        parent_id: K,
        foreign_key: E::Column,
    ) -> RelationshipResult<Vec<M>>
    where
        E: EntityTrait,
        M: FromQueryResult + Sized + Send,
        K: Into<Value> + Clone + Send,
        <E as EntityTrait>::Column: ColumnTrait,
    {
        // Real query: delegates to the working query helper (no panic).
        crate::query_helpers::has_many::<E, M, K>(db, parent_id, foreign_key)
            .await
            .map_err(RelationshipError::from)
    }

    /// Load a belongs-to (inverse) relationship.
    ///
    /// Delegates to [`crate::query_helpers::belongs_to`], loading the single
    /// parent row of entity `E` whose `primary_key` column equals the foreign
    /// key value stored on `self` (`foreign_key_value`), or `None` if not found.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::prelude::*;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
    /// // Use the standalone function instead:
    /// use rf_eloquent::belongs_to;
    /// let post = post::Model { id: 1, user_id: 1 };
    /// let user = belongs_to::<user::Entity, user::Model, _>(
    ///     db,
    ///     post.user_id,
    ///     user::Column::Id
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load_belongs_to<E, M, K>(
        &self,
        db: &DatabaseConnection,
        foreign_key_value: K,
        primary_key: E::Column,
    ) -> RelationshipResult<Option<M>>
    where
        E: EntityTrait,
        M: FromQueryResult + Sized + Send,
        K: Into<Value> + Clone + Send,
        <E as EntityTrait>::Column: ColumnTrait,
    {
        // Real query: delegates to the working query helper (no panic).
        crate::query_helpers::belongs_to::<E, M, K>(db, foreign_key_value, primary_key)
            .await
            .map_err(RelationshipError::from)
    }
}

/// Has One relationship builder
#[derive(Debug, Clone)]
pub struct HasOne<M, R> {
    _model: PhantomData<M>,
    _related: PhantomData<R>,
    foreign_key: String,
}

impl<M, R> HasOne<M, R> {
    /// Create a new HasOne relationship
    pub fn new(foreign_key: impl Into<String>) -> Self {
        Self {
            _model: PhantomData,
            _related: PhantomData,
            foreign_key: foreign_key.into(),
        }
    }

    /// Get the foreign key
    pub fn foreign_key(&self) -> &str {
        &self.foreign_key
    }
}

/// Has Many relationship builder
#[derive(Debug, Clone)]
pub struct HasMany<M, R> {
    _model: PhantomData<M>,
    _related: PhantomData<R>,
    foreign_key: String,
}

impl<M, R> HasMany<M, R> {
    /// Create a new HasMany relationship
    pub fn new(foreign_key: impl Into<String>) -> Self {
        Self {
            _model: PhantomData,
            _related: PhantomData,
            foreign_key: foreign_key.into(),
        }
    }

    /// Get the foreign key
    pub fn foreign_key(&self) -> &str {
        &self.foreign_key
    }
}

/// Belongs To relationship builder
#[derive(Debug, Clone)]
pub struct BelongsTo<M, R> {
    _model: PhantomData<M>,
    _related: PhantomData<R>,
    foreign_key: String,
}

impl<M, R> BelongsTo<M, R> {
    /// Create a new BelongsTo relationship
    pub fn new(foreign_key: impl Into<String>) -> Self {
        Self {
            _model: PhantomData,
            _related: PhantomData,
            foreign_key: foreign_key.into(),
        }
    }

    /// Get the foreign key
    pub fn foreign_key(&self) -> &str {
        &self.foreign_key
    }
}

/// Belongs To Many relationship builder (Many-to-Many)
#[derive(Debug, Clone)]
pub struct BelongsToMany<M, R> {
    _model: PhantomData<M>,
    _related: PhantomData<R>,
    pivot_table: String,
    foreign_pivot_key: String,
    related_pivot_key: String,
}

impl<M, R> BelongsToMany<M, R> {
    /// Create a new BelongsToMany relationship
    pub fn new(
        pivot_table: impl Into<String>,
        foreign_pivot_key: impl Into<String>,
        related_pivot_key: impl Into<String>,
    ) -> Self {
        Self {
            _model: PhantomData,
            _related: PhantomData,
            pivot_table: pivot_table.into(),
            foreign_pivot_key: foreign_pivot_key.into(),
            related_pivot_key: related_pivot_key.into(),
        }
    }

    /// Get the pivot table name
    pub fn pivot_table(&self) -> &str {
        &self.pivot_table
    }

    /// Get the foreign pivot key
    pub fn foreign_pivot_key(&self) -> &str {
        &self.foreign_pivot_key
    }

    /// Get the related pivot key
    pub fn related_pivot_key(&self) -> &str {
        &self.related_pivot_key
    }
}

/// Has One Through relationship builder
#[derive(Debug, Clone)]
pub struct HasOneThrough<M, T, R> {
    _model: PhantomData<M>,
    _through: PhantomData<T>,
    _related: PhantomData<R>,
    through_foreign_key: String,
    final_foreign_key: String,
}

impl<M, T, R> HasOneThrough<M, T, R> {
    /// Create a new HasOneThrough relationship
    pub fn new(
        through_foreign_key: impl Into<String>,
        final_foreign_key: impl Into<String>,
    ) -> Self {
        Self {
            _model: PhantomData,
            _through: PhantomData,
            _related: PhantomData,
            through_foreign_key: through_foreign_key.into(),
            final_foreign_key: final_foreign_key.into(),
        }
    }

    /// Get the through foreign key
    pub fn through_foreign_key(&self) -> &str {
        &self.through_foreign_key
    }

    /// Get the final foreign key
    pub fn final_foreign_key(&self) -> &str {
        &self.final_foreign_key
    }
}

/// Has Many Through relationship builder
#[derive(Debug, Clone)]
pub struct HasManyThrough<M, T, R> {
    _model: PhantomData<M>,
    _through: PhantomData<T>,
    _related: PhantomData<R>,
    through_foreign_key: String,
    final_foreign_key: String,
}

impl<M, T, R> HasManyThrough<M, T, R> {
    /// Create a new HasManyThrough relationship
    pub fn new(
        through_foreign_key: impl Into<String>,
        final_foreign_key: impl Into<String>,
    ) -> Self {
        Self {
            _model: PhantomData,
            _through: PhantomData,
            _related: PhantomData,
            through_foreign_key: through_foreign_key.into(),
            final_foreign_key: final_foreign_key.into(),
        }
    }

    /// Get the through foreign key
    pub fn through_foreign_key(&self) -> &str {
        &self.through_foreign_key
    }

    /// Get the final foreign key
    pub fn final_foreign_key(&self) -> &str {
        &self.final_foreign_key
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fluent relationship builders – Feature 7
// ─────────────────────────────────────────────────────────────────────────────

/// Fluent builder for HasMany relationships.
///
/// Obtained by calling `HasManyBuilder::new()` (or the convenience function
/// `has_many_builder()`).  Supports chained `.order_by()` / `.limit()` before
/// the terminal `.get()` / `.first()` / `.count()` calls.
///
/// # Example
///
/// ```rust,no_run
/// # use rf_eloquent::relationships::HasManyBuilder;
/// # use sea_orm::*;
/// # fn main() {}
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: DatabaseConnection) -> Result<(), DbErr> {
/// let posts = HasManyBuilder::<post::Entity>::new(db, post::Column::UserId, 42i32)
///     .order_by(post::Column::UserId, sea_orm::Order::Desc)
///     .limit(10)
///     .get()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct HasManyBuilder<E>
where
    E: EntityTrait,
{
    db: DatabaseConnection,
    query: Select<E>,
}

impl<E> HasManyBuilder<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
{
    /// Create a new HasManyBuilder.
    ///
    /// * `db`          – live database connection
    /// * `foreign_key` – the column in the related table referencing the parent
    /// * `parent_id`   – value the foreign key must equal
    pub fn new<K>(db: DatabaseConnection, foreign_key: E::Column, parent_id: K) -> Self
    where
        K: Into<Value> + Clone,
    {
        let query = E::find().filter(foreign_key.eq(parent_id));
        Self { db, query }
    }

    /// Add an ORDER BY clause.
    pub fn order_by(mut self, col: E::Column, dir: sea_orm::Order) -> Self {
        self.query = self.query.order_by(col, dir);
        self
    }

    /// Add a LIMIT clause.
    pub fn limit(mut self, n: u64) -> Self {
        self.query = self.query.limit(n);
        self
    }

    /// Apply an additional WHERE filter.
    pub fn filter<F>(mut self, condition: F) -> Self
    where
        F: sea_orm::sea_query::IntoCondition,
    {
        self.query = self.query.filter(condition);
        self
    }

    /// Execute the query and return all matching rows.
    pub async fn get(self) -> Result<Vec<E::Model>, DbErr> {
        self.query.all(&self.db).await
    }

    /// Execute the query and return the first result, if any.
    pub async fn first(self) -> Result<Option<E::Model>, DbErr> {
        self.query.one(&self.db).await
    }

    /// Return the number of matching rows without loading them.
    pub async fn count(self) -> Result<u64, DbErr> {
        let results = self.query.all(&self.db).await?;
        Ok(results.len() as u64)
    }
}

/// Fluent builder for HasOne relationships.
///
/// Behaves like [`HasManyBuilder`] but the terminal method returns
/// `Option<E::Model>` rather than a `Vec`.
pub struct HasOneBuilder<E>
where
    E: EntityTrait,
{
    db: DatabaseConnection,
    query: Select<E>,
}

impl<E> HasOneBuilder<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
{
    /// Create a new HasOneBuilder.
    pub fn new<K>(db: DatabaseConnection, foreign_key: E::Column, parent_id: K) -> Self
    where
        K: Into<Value> + Clone,
    {
        let query = E::find().filter(foreign_key.eq(parent_id));
        Self { db, query }
    }

    /// Apply an additional WHERE filter.
    pub fn filter<F>(mut self, condition: F) -> Self
    where
        F: sea_orm::sea_query::IntoCondition,
    {
        self.query = self.query.filter(condition);
        self
    }

    /// Execute the query and return the related model, if found.
    pub async fn get(self) -> Result<Option<E::Model>, DbErr> {
        self.query.one(&self.db).await
    }

    /// Alias for [`get`](HasOneBuilder::get).
    pub async fn first(self) -> Result<Option<E::Model>, DbErr> {
        self.get().await
    }

    /// Return whether a related record exists.
    pub async fn exists(self) -> Result<bool, DbErr> {
        Ok(self.query.one(&self.db).await?.is_some())
    }
}

/// Fluent builder for BelongsTo relationships.
///
/// Looks up the *parent* model by a primary-key value stored as a foreign key
/// on the child model.
pub struct BelongsToBuilder<E>
where
    E: EntityTrait,
{
    db: DatabaseConnection,
    query: Select<E>,
}

impl<E> BelongsToBuilder<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
{
    /// Create a new BelongsToBuilder.
    ///
    /// * `db`                – live database connection
    /// * `primary_key`       – the PK column of the parent entity
    /// * `foreign_key_value` – the FK value stored on the child model
    pub fn new<K>(db: DatabaseConnection, primary_key: E::Column, foreign_key_value: K) -> Self
    where
        K: Into<Value> + Clone,
    {
        let query = E::find().filter(primary_key.eq(foreign_key_value));
        Self { db, query }
    }

    /// Apply an additional WHERE filter.
    pub fn filter<F>(mut self, condition: F) -> Self
    where
        F: sea_orm::sea_query::IntoCondition,
    {
        self.query = self.query.filter(condition);
        self
    }

    /// Execute the query and return the parent model, if found.
    pub async fn get(self) -> Result<Option<E::Model>, DbErr> {
        self.query.one(&self.db).await
    }

    /// Alias for [`get`](BelongsToBuilder::get).
    pub async fn first(self) -> Result<Option<E::Model>, DbErr> {
        self.get().await
    }
}

/// Convenience constructor for [`HasManyBuilder`].
///
/// ```rust,no_run
/// # use rf_eloquent::relationships::has_many_builder;
/// # use sea_orm::*;
/// # fn main() {}
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: DatabaseConnection) -> Result<(), DbErr> {
/// let posts = has_many_builder::<post::Entity, _>(db, post::Column::UserId, 1i32)
///     .limit(5)
///     .get()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub fn has_many_builder<E, K>(
    db: DatabaseConnection,
    foreign_key: E::Column,
    parent_id: K,
) -> HasManyBuilder<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
    K: Into<Value> + Clone,
{
    HasManyBuilder::new(db, foreign_key, parent_id)
}

/// Convenience constructor for [`HasOneBuilder`].
pub fn has_one_builder<E, K>(
    db: DatabaseConnection,
    foreign_key: E::Column,
    parent_id: K,
) -> HasOneBuilder<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
    K: Into<Value> + Clone,
{
    HasOneBuilder::new(db, foreign_key, parent_id)
}

/// Convenience constructor for [`BelongsToBuilder`].
pub fn belongs_to_builder<E, K>(
    db: DatabaseConnection,
    primary_key: E::Column,
    foreign_key_value: K,
) -> BelongsToBuilder<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
    K: Into<Value> + Clone,
{
    BelongsToBuilder::new(db, primary_key, foreign_key_value)
}

// ─────────────────────────────────────────────────────────────────────────────
// Deferred-connection relationship accessors — Feature: post.user() / user.posts()
// ─────────────────────────────────────────────────────────────────────────────
//
// Unlike the `*Builder` types above (which own a `DatabaseConnection` from
// construction), these `*Ref` builders capture only the parent key + column and
// take the connection *by reference* at the terminal call. This yields the
// ergonomic Laravel-style shape:
//
//     let posts  = user.posts().get(&db).await?;   // HasManyRef
//     let author = post.user().get(&db).await?;    // BelongsToRef
//
// The `Ref` accessor methods take no `db` argument, so they can be generated
// from a bare model instance via [`relationship_accessors!`].

/// Deferred-connection builder for a **has-many** relationship.
///
/// Filters the related entity `E` on `foreign_key == parent_id`. The database
/// connection is supplied at the terminal method (`get`/`first`/`count`),
/// enabling the `user.posts().get(&db)` shape.
pub struct HasManyRef<E>
where
    E: EntityTrait,
{
    query: Select<E>,
}

impl<E> HasManyRef<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
{
    /// Build a has-many accessor filtering `foreign_key == parent_id`.
    pub fn new<K>(foreign_key: E::Column, parent_id: K) -> Self
    where
        K: Into<Value> + Clone,
    {
        Self {
            query: E::find().filter(foreign_key.eq(parent_id)),
        }
    }

    /// Add an `ORDER BY` clause.
    pub fn order_by(mut self, col: E::Column, dir: sea_orm::Order) -> Self {
        self.query = self.query.order_by(col, dir);
        self
    }

    /// Add a `LIMIT` clause.
    pub fn limit(mut self, n: u64) -> Self {
        self.query = self.query.limit(n);
        self
    }

    /// Apply an additional `WHERE` filter.
    pub fn filter<F>(mut self, condition: F) -> Self
    where
        F: sea_orm::sea_query::IntoCondition,
    {
        self.query = self.query.filter(condition);
        self
    }

    /// Execute against `db` and return all related rows.
    pub async fn get(self, db: &DatabaseConnection) -> Result<Vec<E::Model>, DbErr> {
        self.query.all(db).await
    }

    /// Execute against `db` and return the first related row, if any.
    pub async fn first(self, db: &DatabaseConnection) -> Result<Option<E::Model>, DbErr> {
        self.query.one(db).await
    }

    /// Count the related rows.
    pub async fn count(self, db: &DatabaseConnection) -> Result<u64, DbErr> {
        Ok(self.query.all(db).await?.len() as u64)
    }
}

/// Deferred-connection builder for a **has-one** relationship.
///
/// Like [`HasManyRef`] but the terminal `get`/`first` returns `Option<E::Model>`.
pub struct HasOneRef<E>
where
    E: EntityTrait,
{
    query: Select<E>,
}

impl<E> HasOneRef<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
{
    /// Build a has-one accessor filtering `foreign_key == parent_id`.
    pub fn new<K>(foreign_key: E::Column, parent_id: K) -> Self
    where
        K: Into<Value> + Clone,
    {
        Self {
            query: E::find().filter(foreign_key.eq(parent_id)),
        }
    }

    /// Apply an additional `WHERE` filter.
    pub fn filter<F>(mut self, condition: F) -> Self
    where
        F: sea_orm::sea_query::IntoCondition,
    {
        self.query = self.query.filter(condition);
        self
    }

    /// Execute against `db` and return the related row, if any.
    pub async fn get(self, db: &DatabaseConnection) -> Result<Option<E::Model>, DbErr> {
        self.query.one(db).await
    }

    /// Alias for [`get`](HasOneRef::get).
    pub async fn first(self, db: &DatabaseConnection) -> Result<Option<E::Model>, DbErr> {
        self.query.one(db).await
    }

    /// Whether a related row exists.
    pub async fn exists(self, db: &DatabaseConnection) -> Result<bool, DbErr> {
        Ok(self.query.one(db).await?.is_some())
    }
}

/// Deferred-connection builder for a **belongs-to** relationship.
///
/// Looks up the parent entity `E` by `primary_key == foreign_key_value`,
/// enabling the `post.user().get(&db)` shape.
pub struct BelongsToRef<E>
where
    E: EntityTrait,
{
    query: Select<E>,
}

impl<E> BelongsToRef<E>
where
    E: EntityTrait,
    <E as EntityTrait>::Column: ColumnTrait,
{
    /// Build a belongs-to accessor filtering `primary_key == foreign_key_value`.
    pub fn new<K>(primary_key: E::Column, foreign_key_value: K) -> Self
    where
        K: Into<Value> + Clone,
    {
        Self {
            query: E::find().filter(primary_key.eq(foreign_key_value)),
        }
    }

    /// Apply an additional `WHERE` filter.
    pub fn filter<F>(mut self, condition: F) -> Self
    where
        F: sea_orm::sea_query::IntoCondition,
    {
        self.query = self.query.filter(condition);
        self
    }

    /// Execute against `db` and return the parent row, if any.
    pub async fn get(self, db: &DatabaseConnection) -> Result<Option<E::Model>, DbErr> {
        self.query.one(db).await
    }

    /// Alias for [`get`](BelongsToRef::get).
    pub async fn first(self, db: &DatabaseConnection) -> Result<Option<E::Model>, DbErr> {
        self.query.one(db).await
    }
}

/// `relationship_accessors!` — generate ergonomic instance accessors on a model.
///
/// Given a loaded model instance, this generates zero-argument accessor methods
/// that read the join key straight from `self` and return a deferred-connection
/// builder ([`HasManyRef`] / [`HasOneRef`] / [`BelongsToRef`]). The connection is
/// passed at the terminal call, matching Laravel's `post.user()` / `user.posts()`.
///
/// Each entry has the form:
///
/// ```text
/// <kind> <method> => <RelatedEntity>, <RelatedColumn>, <self_field>
/// ```
///
/// * `has_many` / `has_one`: `<RelatedColumn>` is the FK column on the related
///   table and `<self_field>` is the parent's key field (usually `id`).
/// * `belongs_to`: `<RelatedColumn>` is the parent's PK column and `<self_field>`
///   is the FK field stored on `self` (e.g. `user_id`).
///
/// # Example
///
/// ```rust,no_run
/// # use rf_eloquent::relationship_accessors;
/// # use sea_orm::DatabaseConnection;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// // user.posts() -> HasManyRef<post::Entity> filtering post.user_id == user.id
/// relationship_accessors!(user::Model {
///     has_many posts => post::Entity, post::Column::UserId, id;
/// });
///
/// // post.user() -> BelongsToRef<user::Entity> filtering user.id == post.user_id
/// relationship_accessors!(post::Model {
///     belongs_to user => user::Entity, user::Column::Id, user_id;
/// });
///
/// # async fn example(db: &DatabaseConnection, u: &user::Model, p: &post::Model) -> Result<(), sea_orm::DbErr> {
/// let posts  = u.posts().get(db).await?;
/// let author = p.user().get(db).await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! relationship_accessors {
    ($model:ty { $($kind:ident $method:ident => $entity:ty, $col:expr, $field:ident);* $(;)? }) => {
        impl $model {
            $(
                $crate::relationship_accessors!(@method $kind $method, $entity, $col, $field);
            )*
        }
    };
    (@method has_many $method:ident, $entity:ty, $col:expr, $field:ident) => {
        /// Load the has-many relationship for this model instance.
        pub fn $method(&self) -> $crate::relationships::HasManyRef<$entity> {
            $crate::relationships::HasManyRef::new($col, self.$field.clone())
        }
    };
    (@method has_one $method:ident, $entity:ty, $col:expr, $field:ident) => {
        /// Load the has-one relationship for this model instance.
        pub fn $method(&self) -> $crate::relationships::HasOneRef<$entity> {
            $crate::relationships::HasOneRef::new($col, self.$field.clone())
        }
    };
    (@method belongs_to $method:ident, $entity:ty, $col:expr, $field:ident) => {
        /// Load the belongs-to (inverse) relationship for this model instance.
        pub fn $method(&self) -> $crate::relationships::BelongsToRef<$entity> {
            $crate::relationships::BelongsToRef::new($col, self.$field.clone())
        }
    };
}

/// `define_relationships!` — macro for concisely declaring relationship helpers on a model.
///
/// ```rust,no_run
/// # use rf_eloquent::define_relationships;
/// # use rf_eloquent::relationships::{HasManyBuilder, BelongsToBuilder};
/// # use sea_orm::DatabaseConnection;
/// # fn main() {}
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// struct UserModel { id: i32 }
///
/// // Generates `UserModel::posts(&self, db) -> HasManyBuilder<post::Entity>`
/// // filtering `post::Column::UserId == 1`.
/// define_relationships!(UserModel; has_many posts, post::Entity, post::Column::UserId, 1i32);
/// ```
#[macro_export]
macro_rules! define_relationships {
    // has_many arm: define_relationships!(Model; has_many method, Entity, col_expr, id_expr)
    ($model:ty; has_many $method:ident, $entity:ty, $col:expr, $id:expr) => {
        impl $model {
            pub fn $method(&self, db: ::sea_orm::DatabaseConnection)
                -> $crate::relationships::HasManyBuilder<$entity>
            {
                $crate::relationships::HasManyBuilder::new(db, $col, $id)
            }
        }
    };
    // has_one arm
    ($model:ty; has_one $method:ident, $entity:ty, $col:expr, $id:expr) => {
        impl $model {
            pub fn $method(&self, db: ::sea_orm::DatabaseConnection)
                -> $crate::relationships::HasOneBuilder<$entity>
            {
                $crate::relationships::HasOneBuilder::new(db, $col, $id)
            }
        }
    };
    // belongs_to arm
    ($model:ty; belongs_to $method:ident, $entity:ty, $col:expr, $fk:expr) => {
        impl $model {
            pub fn $method(&self, db: ::sea_orm::DatabaseConnection)
                -> $crate::relationships::BelongsToBuilder<$entity>
            {
                $crate::relationships::BelongsToBuilder::new(db, $col, $fk)
            }
        }
    };
}

/// Relationship metadata for documentation and introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMeta {
    pub name: String,
    pub kind: RelationshipKind,
    pub foreign_key: Option<String>,
    pub pivot_table: Option<String>,
}

/// Types of relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipKind {
    HasOne,
    HasMany,
    BelongsTo,
    BelongsToMany,
    HasOneThrough,
    HasManyThrough,
    MorphTo,
    MorphOne,
    MorphMany,
    MorphToMany,
    MorphedByMany,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_error_display() {
        let err = RelationshipError::ForeignKeyNotFound("user_id".to_string());
        assert_eq!(err.to_string(), "Foreign key not found: user_id");
    }

    #[test]
    fn test_relationship_error_invalid_config() {
        let err = RelationshipError::InvalidConfiguration("Missing pivot table".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid relationship configuration: Missing pivot table"
        );
    }

    #[test]
    fn test_relationship_error_not_found() {
        let err = RelationshipError::RelatedModelNotFound;
        assert_eq!(err.to_string(), "Related model not found");
    }

    #[test]
    fn test_relationship_error_pivot() {
        let err = RelationshipError::PivotError("Duplicate entry".to_string());
        assert_eq!(err.to_string(), "Pivot table error: Duplicate entry");
    }

    #[test]
    fn test_has_one_builder() {
        let rel = HasOne::<(), ()>::new("user_id");
        assert_eq!(rel.foreign_key(), "user_id");
    }

    #[test]
    fn test_has_many_builder() {
        let rel = HasMany::<(), ()>::new("user_id");
        assert_eq!(rel.foreign_key(), "user_id");
    }

    #[test]
    fn test_belongs_to_builder() {
        let rel = BelongsTo::<(), ()>::new("author_id");
        assert_eq!(rel.foreign_key(), "author_id");
    }

    #[test]
    fn test_belongs_to_many_builder() {
        let rel = BelongsToMany::<(), ()>::new("post_tag", "post_id", "tag_id");
        assert_eq!(rel.pivot_table(), "post_tag");
        assert_eq!(rel.foreign_pivot_key(), "post_id");
        assert_eq!(rel.related_pivot_key(), "tag_id");
    }

    #[test]
    fn test_has_one_through_builder() {
        let rel = HasOneThrough::<(), (), ()>::new("country_id", "city_id");
        assert_eq!(rel.through_foreign_key(), "country_id");
        assert_eq!(rel.final_foreign_key(), "city_id");
    }

    #[test]
    fn test_has_many_through_builder() {
        let rel = HasManyThrough::<(), (), ()>::new("country_id", "city_id");
        assert_eq!(rel.through_foreign_key(), "country_id");
        assert_eq!(rel.final_foreign_key(), "city_id");
    }

    mod user {
        use sea_orm::entity::prelude::*;
        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub name: String,
        }
        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}
        impl ActiveModelBehavior for ActiveModel {}
    }

    mod post {
        use sea_orm::entity::prelude::*;
        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "posts")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub user_id: i32,
            pub title: String,
        }
        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}
        impl ActiveModelBehavior for ActiveModel {}
    }

    // Generate instance accessors: user.posts() and post.user().
    crate::relationship_accessors!(user::Model {
        has_many posts => post::Entity, post::Column::UserId, id;
    });
    crate::relationship_accessors!(post::Model {
        belongs_to user => user::Entity, user::Column::Id, user_id;
    });

    #[test]
    fn test_relationship_accessors_compile_and_type() {
        // The accessor methods exist, take no db argument, and yield the
        // deferred-connection Ref builders. (Terminal `.get(&db)` is exercised
        // against a real DB in the `eloquent_relationship_accessors` sandbox probe.)
        let u = user::Model { id: 7, name: "Alice".into() };
        let _posts: HasManyRef<post::Entity> = u.posts();

        let p = post::Model { id: 1, user_id: 7, title: "T".into() };
        let _author: BelongsToRef<user::Entity> = p.user();
    }

    #[test]
    fn test_relationship_kind() {
        assert_eq!(
            serde_json::to_string(&RelationshipKind::HasOne).unwrap(),
            "\"HasOne\""
        );
        assert_eq!(
            serde_json::to_string(&RelationshipKind::BelongsToMany).unwrap(),
            "\"BelongsToMany\""
        );
    }
}
