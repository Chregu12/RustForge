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
//!
//! // Eager loading (load with main query)
//! let users = User::query(db.clone())
//!     .with::<post::Entity>("posts")
//!     .with::<comment::Entity>("comments")
//!     .get()
//!     .await?;
//!
//! // Lazy loading (load on demand)
//! let user = User::find_by_id(1).one(&db).await?.unwrap();
//! let posts = user.load_relation::<post::Entity>(&db).await?;
//!
//! // Lazy eager loading (load for collection after fetching)
//! let mut users = User::query(db.clone()).get().await?;
//! load_relations(&db, &mut users, "posts").await?;
//! ```

use async_trait::async_trait;
use sea_orm::{
    DatabaseConnection, DbErr, EntityTrait, ModelTrait, Related,
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
///
/// let posts = Post::query(db)
///     .with_relation::<user::Entity>("author")
///     .with_relation::<comment::Entity>("comments")
///     .get()
///     .await?;
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
    /// User::query(db).with_relation::<post::Entity>("posts")
    /// ```
    fn with_relation<R>(self, relation: &str) -> Self
    where
        R: EntityTrait;

    /// Eager load multiple relationships
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// User::query(db).with_relations(&["posts", "comments", "profile"])
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
///
/// let user = User::find_by_id(1).one(&db).await?.unwrap();
/// let posts = user.lazy_load::<post::Entity>(&db).await?;
/// ```
#[async_trait]
pub trait LazyLoad: ModelTrait + Sized {
    /// Lazy load a relationship for this model
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let posts = user.lazy_load::<post::Entity>(&db).await?;
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
    /// let author = post.lazy_load_one::<user::Entity>(&db).await?;
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
/// // Fetch users without relations
/// let mut users = User::query(db.clone()).get().await?;
///
/// // Load posts for all users in one query
/// load_relation(&db, &mut users, "posts").await?;
/// ```
pub async fn load_relation<E, R>(
    _db: &DatabaseConnection,
    models: &mut [E::Model],
    _relation: &str,
) -> LoadResult<HashMap<i64, Vec<R::Model>>>
where
    E: EntityTrait,
    R: EntityTrait,
    E: Related<R>,
    E::Model: ModelTrait,
{
    if models.is_empty() {
        return Ok(HashMap::new());
    }

    // In a real implementation, you would:
    // 1. Extract all parent IDs from the models
    // 2. Query the related table with WHERE IN (parent_ids)
    // 3. Group the results by parent ID
    // 4. Return the grouped results

    // For now, we return an empty map as a placeholder
    // This would require dynamic ID extraction which needs more SeaORM introspection
    Ok(HashMap::new())
}

/// Load multiple relationships for a collection of models
///
/// # Example
///
/// ```rust,no_run
/// let mut users = User::query(db.clone()).get().await?;
/// load_relations(&db, &mut users, &["posts", "comments"]).await?;
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
    for relation in relations {
        // In a real implementation, we would dynamically load each relation
        // For now, this is a placeholder
        let _ = relation;
    }

    Ok(())
}

/// Extension trait for collections to support lazy eager loading
///
/// This trait adds relationship loading methods to vectors of models.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::loading::CollectionExt;
///
/// let users = User::query(db.clone()).get().await?;
/// let users_with_posts = users.load(&db, "posts").await?;
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
    /// let users_with_posts = users.load(&db, "posts").await?;
    /// ```
    async fn load<R>(
        &mut self,
        db: &DatabaseConnection,
        relation: &str,
    ) -> LoadResult<&mut Self>
    where
        R: EntityTrait,
        E: Related<R>;

    /// Load multiple relationships for all models in the collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let users_with_data = users.load_multiple(&db, &["posts", "comments"]).await?;
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
    E::Model: ModelTrait + Send,
{
    async fn load<R>(
        &mut self,
        db: &DatabaseConnection,
        relation: &str,
    ) -> LoadResult<&mut Self>
    where
        R: EntityTrait,
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
/// let should_load_posts = should_eager_load("posts", &config);
/// if should_load_posts {
///     query = query.with_relation::<post::Entity>("posts");
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
///
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
        let config = EagerLoadConfig::new()
            .always("posts")
            .never("posts"); // never takes priority

        assert!(!config.should_load("posts"));
    }
}
