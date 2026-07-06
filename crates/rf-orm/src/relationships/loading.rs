//! # Lazy vs Eager Loading Control
//!
//! Laravel-style eager loading and lazy loading for relationships.
//!
//! ## Overview
//!
//! This module provides control over when and how relationships are loaded:
//! - **Eager Loading**: Load relationships upfront with the main query (prevents N+1)
//! - **Lazy Loading**: Load relationships on-demand when accessed
//! - **Lazy Eager Loading**: Load relationships after the main query for collections
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::relationships::loading::*;
//! use sea_orm::EntityTrait;
//! # fn main() {}
//! # mod user {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "users")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
//! #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { Post }
//! #     impl RelationTrait for Relation {
//! #         fn def(&self) -> RelationDef { Entity::has_many(super::post::Entity).into() }
//! #     }
//! #     impl Related<super::post::Entity> for Entity {
//! #         fn to() -> RelationDef { Relation::Post.def() }
//! #     }
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # mod post {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "posts")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
//! #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
//! #     impl RelationTrait for Relation {
//! #         fn def(&self) -> RelationDef {
//! #             Entity::belongs_to(super::user::Entity)
//! #                 .from(Column::UserId).to(super::user::Column::Id).into()
//! #         }
//! #     }
//! #     impl Related<super::user::Entity> for Entity {
//! #         fn to() -> RelationDef { Relation::User.def() }
//! #     }
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
//! // Lazy loading (load on demand) for a single model
//! let user = user::Entity::find_by_id(1).one(&db).await?.unwrap();
//! let posts = user.lazy_load::<post::Entity>(&db).await?;
//!
//! // Lazy eager loading (load a relation for a whole collection)
//! let mut users = user::Entity::find().all(&db).await?;
//! load_relation::<user::Entity, post::Entity>(&db, &mut users, "posts").await?;
//! load_relations::<user::Entity>(&db, &mut users, &["posts"]).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use sea_orm::{
    DatabaseConnection, DbErr, EntityTrait, Iterable, LoaderTrait, ModelTrait, PrimaryKeyToColumn,
    Related, Value,
};
use std::collections::HashMap;

/// Result type for loading operations
pub type LoadResult<T> = Result<T, DbErr>;

/// Trait for eager loading relationships in queries
///
/// This trait extends the query builder with eager loading capabilities.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::loading::EagerLoad;
/// # mod user { use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {} }
/// # mod comment { use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "comments")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {} }
/// // A query builder implementing `EagerLoad` can chain relation loads:
/// async fn run<Q: EagerLoad>(query: Q) -> Q {
///     query
///         .with_relation::<user::Entity>("author")
///         .with_relation::<comment::Entity>("comments")
/// }
/// ```
#[async_trait]
pub trait EagerLoad: Sized {
    /// The model type being queried
    type Model;

    /// Eager load a relationship
    ///
    /// # Arguments
    ///
    /// * `relation` - The name of the relation to load
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::loading::EagerLoad;
    /// # mod post { use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {} }
    /// fn add_posts<Q: EagerLoad>(query: Q) -> Q {
    ///     query.with_relation::<post::Entity>("posts")
    /// }
    /// ```
    fn with_relation<R>(self, relation: &str) -> Self
    where
        R: EntityTrait;

    /// Eager load multiple relationships
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::loading::EagerLoad;
    /// fn add_many<Q: EagerLoad>(query: Q) -> Q {
    ///     query.with_relations(&["posts", "comments", "profile"])
    /// }
    /// ```
    fn with_relations(self, relations: &[&str]) -> Self;

    /// Execute the query and load all eager loaded relationships
    async fn get_with_relations(self) -> LoadResult<Vec<(Self::Model, RelationshipData)>>;
}

/// Trait for lazy loading relationships on a model
///
/// This trait allows loading relationships on-demand for a single model.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::loading::LazyLoad;
/// use sea_orm::EntityTrait;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { Post }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef { Entity::has_many(super::post::Entity).into() }
/// #     }
/// #     impl Related<super::post::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::Post.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef {
/// #             Entity::belongs_to(super::user::Entity)
/// #                 .from(Column::UserId).to(super::user::Column::Id).into()
/// #         }
/// #     }
/// #     impl Related<super::user::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::User.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// let user = user::Entity::find_by_id(1).one(&db).await?.unwrap();
/// let posts = user.lazy_load::<post::Entity>(&db).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait LazyLoad: ModelTrait + Sized {
    /// Lazy load a relationship for this model
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::loading::LazyLoad;
    /// use sea_orm::EntityTrait;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { Post }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef { Entity::has_many(super::post::Entity).into() }
    /// #     }
    /// #     impl Related<super::post::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::Post.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef {
    /// #             Entity::belongs_to(super::user::Entity)
    /// #                 .from(Column::UserId).to(super::user::Column::Id).into()
    /// #         }
    /// #     }
    /// #     impl Related<super::user::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::User.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// # let user = user::Entity::find_by_id(1).one(&db).await?.unwrap();
    /// let posts = user.lazy_load::<post::Entity>(&db).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn lazy_load<R>(&self, db: &DatabaseConnection) -> LoadResult<Vec<R::Model>>
    where
        R: EntityTrait,
        Self::Entity: Related<R>,
    {
        self.find_related(R::default()).all(db).await
    }

    /// Lazy load a belongs-to relationship (returns Option)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::loading::LazyLoad;
    /// use sea_orm::EntityTrait;
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
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef {
    /// #             Entity::belongs_to(super::user::Entity)
    /// #                 .from(Column::UserId).to(super::user::Column::Id).into()
    /// #         }
    /// #     }
    /// #     impl Related<super::user::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::User.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// # let post = post::Entity::find_by_id(1).one(&db).await?.unwrap();
    /// let author = post.lazy_load_one::<user::Entity>(&db).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn lazy_load_one<R>(&self, db: &DatabaseConnection) -> LoadResult<Option<R::Model>>
    where
        R: EntityTrait,
        Self::Entity: Related<R>,
    {
        self.find_related(R::default()).one(db).await
    }
}

// Blanket implementation for all ModelTrait types
impl<T> LazyLoad for T where T: ModelTrait {}

/// Container for relationship data loaded with a model
///
/// This stores all relationships that have been eager loaded or
/// lazy eager loaded for a model.
#[derive(Debug, Clone, Default)]
pub struct RelationshipData {
    /// Map of relation name to loaded data (as JSON or similar)
    data: HashMap<String, Vec<u8>>,
}

impl RelationshipData {
    /// Create a new empty relationship data container
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Add relationship data
    pub fn add(&mut self, relation: &str, data: Vec<u8>) {
        self.data.insert(relation.to_string(), data);
    }

    /// Get relationship data
    pub fn get(&self, relation: &str) -> Option<&Vec<u8>> {
        self.data.get(relation)
    }

    /// Check if a relation has been loaded
    pub fn has(&self, relation: &str) -> bool {
        self.data.contains_key(relation)
    }

    /// Get all loaded relation names
    pub fn relations(&self) -> Vec<&str> {
        self.data.keys().map(|s| s.as_str()).collect()
    }
}

/// Load a relationship for a collection of models (lazy eager loading)
///
/// This function loads a relationship for multiple models in a single query,
/// avoiding N+1 query problems.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `models` - Collection of models
/// * `relation` - Name of the relation to load
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::loading::load_relation;
/// use sea_orm::EntityTrait;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { Post }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef { Entity::has_many(super::post::Entity).into() }
/// #     }
/// #     impl Related<super::post::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::Post.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef {
/// #             Entity::belongs_to(super::user::Entity)
/// #                 .from(Column::UserId).to(super::user::Column::Id).into()
/// #         }
/// #     }
/// #     impl Related<super::user::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::User.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// // Fetch users without relations
/// let mut users = user::Entity::find().all(&db).await?;
///
/// // Load posts for all users in one query
/// load_relation::<user::Entity, post::Entity>(&db, &mut users, "posts").await?;
/// # Ok(())
/// # }
/// ```
pub async fn load_relation<E, R>(
    db: &DatabaseConnection,
    models: &mut [E::Model],
    _relation: &str,
) -> LoadResult<HashMap<i64, Vec<R::Model>>>
where
    E: EntityTrait,
    R: EntityTrait,
    E: Related<R>,
    E::Model: ModelTrait + Sync,
    R::Model: Send + Sync,
{
    if models.is_empty() {
        return Ok(HashMap::new());
    }

    // Batch-load the related rows for every parent in a single query (no N+1),
    // using SeaORM's LoaderTrait which resolves the relation from `E: Related<R>`.
    // The result is aligned by index with `models`.
    let slice: &[E::Model] = models;
    let related: Vec<Vec<R::Model>> = slice.load_many(R::default(), db).await?;

    // Group the related rows by their parent's primary-key value.
    let mut grouped: HashMap<i64, Vec<R::Model>> = HashMap::new();
    for (model, children) in models.iter().zip(related.into_iter()) {
        if let Some(id) = primary_key_i64::<E>(model) {
            grouped.entry(id).or_default().extend(children);
        }
    }

    Ok(grouped)
}

/// Extract a model's (single-column) primary key as an `i64`, if it is an
/// integer key. Returns `None` for non-integer primary keys, which the `i64`-keyed
/// grouping used by [`load_relation`] cannot represent.
fn primary_key_i64<E>(model: &E::Model) -> Option<i64>
where
    E: EntityTrait,
    E::Model: ModelTrait,
{
    let pk_column = <E::PrimaryKey as Iterable>::iter().next()?.into_column();
    match model.get(pk_column) {
        Value::TinyInt(Some(v)) => Some(v as i64),
        Value::SmallInt(Some(v)) => Some(v as i64),
        Value::Int(Some(v)) => Some(v as i64),
        Value::BigInt(Some(v)) => Some(v),
        Value::TinyUnsigned(Some(v)) => Some(v as i64),
        Value::SmallUnsigned(Some(v)) => Some(v as i64),
        Value::Unsigned(Some(v)) => Some(v as i64),
        Value::BigUnsigned(Some(v)) => Some(v as i64),
        _ => None,
    }
}

/// Load multiple relationships for a collection of models, dispatched by their
/// string names.
///
/// # Real batch loading vs. string dispatch
///
/// The actual batch load (`WHERE parent_id IN (...)`, no N+1) is implemented and
/// proven by the *typed* entrypoints [`load_relation`] and
/// [`crate::relationships::eager_load`], which resolve the child entity `R` from
/// the `E: Related<R>` bound at compile time.
///
/// This function instead takes relation names as **strings**. Rust cannot map a
/// runtime `&str` to a static `EntityTrait` type without codegen, so — rather
/// than silently pretending to load — it returns an error naming the relations
/// it was asked for. A future `#[derive]` on the model can generate the
/// name → type registry that would make this string-based form dispatch to the
/// real typed loader.
///
/// An empty relation list is a no-op and returns `Ok(())`.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::loading::load_relations;
/// use sea_orm::EntityTrait;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// let mut users = user::Entity::find().all(&db).await?;
/// // For the real batch load, prefer the typed API:
/// //   load_relation::<user::Entity, post::Entity>(&db, &mut users, "posts").await?;
/// // The string-based multi form reports that it needs a derive to dispatch:
/// let names: &[&str] = &[];
/// load_relations::<user::Entity>(&db, &mut users, names).await?; // Ok: nothing requested
/// # Ok(())
/// # }
/// ```
pub async fn load_relations<E>(
    _db: &DatabaseConnection,
    _models: &mut [E::Model],
    relations: &[&str],
) -> LoadResult<()>
where
    E: EntityTrait,
    E::Model: ModelTrait,
{
    if relations.is_empty() {
        return Ok(());
    }

    // We deliberately do NOT return a fake `Ok(())` here: string → entity-type
    // dispatch is impossible without generated code, so report it honestly.
    Err(DbErr::Custom(format!(
        "load_relations: cannot dispatch relations {:?} from string names without a \
         generated name->type registry. Use the typed batch loader instead: \
         `load_relation::<Parent, Child>(db, &mut models, \"{}\")` or \
         `relationships::eager_load::<Parent, Child>(models, db)`.",
        relations,
        relations.first().copied().unwrap_or("posts"),
    )))
}

/// Extension trait for collections to support lazy eager loading
///
/// This trait adds relationship loading methods to vectors of models.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::loading::CollectionExt;
/// use sea_orm::EntityTrait;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { Post }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef { Entity::has_many(super::post::Entity).into() }
/// #     }
/// #     impl Related<super::post::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::Post.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef {
/// #             Entity::belongs_to(super::user::Entity)
/// #                 .from(Column::UserId).to(super::user::Column::Id).into()
/// #         }
/// #     }
/// #     impl Related<super::user::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::User.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// let mut users = user::Entity::find().all(&db).await?;
/// CollectionExt::<user::Entity>::load::<post::Entity>(&mut users, &db, "posts").await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait CollectionExt<E>
where
    E: EntityTrait,
    E::Model: ModelTrait,
{
    /// Load a relationship for all models in the collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::loading::CollectionExt;
    /// use sea_orm::EntityTrait;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { Post }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef { Entity::has_many(super::post::Entity).into() }
    /// #     }
    /// #     impl Related<super::post::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::Post.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef {
    /// #             Entity::belongs_to(super::user::Entity)
    /// #                 .from(Column::UserId).to(super::user::Column::Id).into()
    /// #         }
    /// #     }
    /// #     impl Related<super::user::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::User.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut users = user::Entity::find().all(&db).await?;
    /// CollectionExt::<user::Entity>::load::<post::Entity>(&mut users, &db, "posts").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load<R>(&mut self, db: &DatabaseConnection, relation: &str) -> LoadResult<&mut Self>
    where
        R: EntityTrait,
        R::Model: Send + Sync,
        E: Related<R>;

    /// Load multiple relationships for all models in the collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::loading::CollectionExt;
    /// use sea_orm::EntityTrait;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut users = user::Entity::find().all(&db).await?;
    /// CollectionExt::<user::Entity>::load_multiple(&mut users, &db, &["posts", "comments"]).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load_multiple(
        &mut self,
        db: &DatabaseConnection,
        relations: &[&str],
    ) -> LoadResult<&mut Self>;
}

#[async_trait]
impl<E> CollectionExt<E> for Vec<E::Model>
where
    E: EntityTrait,
    E::Model: ModelTrait + Send + Sync,
{
    async fn load<R>(&mut self, db: &DatabaseConnection, relation: &str) -> LoadResult<&mut Self>
    where
        R: EntityTrait,
        R::Model: Send + Sync,
        E: Related<R>,
    {
        load_relation::<E, R>(db, self, relation).await?;
        Ok(self)
    }

    async fn load_multiple(
        &mut self,
        db: &DatabaseConnection,
        relations: &[&str],
    ) -> LoadResult<&mut Self> {
        load_relations::<E>(db, self, relations).await?;
        Ok(self)
    }
}

/// Helper to determine if a relationship should be eager loaded
///
/// This can be used to conditionally eager load based on configuration or context.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::loading::should_eager_load;
/// let config = ["posts", "comments"];
/// let should_load_posts = should_eager_load("posts", &config);
/// if should_load_posts {
///     // e.g. add the relation to the query here
/// }
/// ```
pub fn should_eager_load(relation: &str, eager_load_config: &[&str]) -> bool {
    eager_load_config.contains(&relation)
}

/// Configuration for eager loading behavior
///
/// This struct allows you to configure default eager loading behavior
/// for specific entities or globally.
#[derive(Debug, Clone)]
pub struct EagerLoadConfig {
    /// Relations that should always be eager loaded
    pub always_load: Vec<String>,
    /// Relations that should never be eager loaded
    pub never_load: Vec<String>,
    /// Maximum depth for nested eager loading
    pub max_depth: usize,
}

impl Default for EagerLoadConfig {
    fn default() -> Self {
        Self {
            always_load: Vec::new(),
            never_load: Vec::new(),
            max_depth: 3,
        }
    }
}

impl EagerLoadConfig {
    /// Create a new eager load configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a relation to always eager load
    pub fn always(mut self, relation: impl Into<String>) -> Self {
        self.always_load.push(relation.into());
        self
    }

    /// Add a relation to never eager load
    pub fn never(mut self, relation: impl Into<String>) -> Self {
        self.never_load.push(relation.into());
        self
    }

    /// Set the maximum depth for nested eager loading
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Check if a relation should be eager loaded according to this config
    pub fn should_load(&self, relation: &str) -> bool {
        if self.never_load.iter().any(|r| r == relation) {
            return false;
        }

        if self.always_load.iter().any(|r| r == relation) {
            return true;
        }

        false
    }
}

/// Marker trait for entities that support eager loading
///
/// Implement this trait to mark an entity as supporting eager loading.
pub trait SupportsEagerLoading: EntityTrait {
    /// Get the eager load configuration for this entity
    fn eager_load_config() -> EagerLoadConfig {
        EagerLoadConfig::default()
    }

    /// Get the list of relations available for eager loading
    fn available_relations() -> Vec<&'static str> {
        Vec::new()
    }
}

/// Macro to implement eager loading support for an entity
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::supports_eager_loading;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// supports_eager_loading!(
///     user::Entity,
///     relations: ["posts", "comments", "profile"],
///     always: ["profile"],
///     never: ["sessions"]
/// );
/// ```
#[macro_export]
macro_rules! supports_eager_loading {
    (
        $entity:ty,
        relations: [$($relation:literal),*],
        always: [$($always:literal),*],
        never: [$($never:literal),*]
    ) => {
        impl $crate::relationships::loading::SupportsEagerLoading for $entity {
            fn eager_load_config() -> $crate::relationships::loading::EagerLoadConfig {
                $crate::relationships::loading::EagerLoadConfig::new()
                    $(.always($always))*
                    $(.never($never))*
            }

            fn available_relations() -> Vec<&'static str> {
                vec![$($relation),*]
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_data() {
        let mut data = RelationshipData::new();
        assert!(!data.has("posts"));

        data.add("posts", vec![1, 2, 3]);
        assert!(data.has("posts"));
        assert_eq!(data.get("posts"), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_should_eager_load() {
        let config = vec!["posts", "comments"];
        assert!(should_eager_load("posts", &config));
        assert!(!should_eager_load("profile", &config));
    }

    #[test]
    fn test_eager_load_config() {
        let config = EagerLoadConfig::new()
            .always("profile")
            .never("sessions")
            .max_depth(5);

        assert!(config.should_load("profile"));
        assert!(!config.should_load("sessions"));
        assert_eq!(config.max_depth, 5);
    }

    #[test]
    fn test_eager_load_config_priority() {
        let config = EagerLoadConfig::new().always("posts").never("posts"); // never takes priority

        assert!(!config.should_load("posts"));
    }
}
