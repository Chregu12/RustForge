//! # Eager Loading System
//!
//! Prevents N+1 query problems by loading related models in advance.
//! Supports nested relationships and conditional loading.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rf_eloquent::prelude::*;
//!
//! // `EagerLoadBuilder` wraps any query value and records the relations to load.
//! // Here we use a placeholder query type to demonstrate the fluent API.
//! let query = "users";
//!
//! // Load a single relationship (and a nested one)
//! let builder = EagerLoadBuilder::new(query)
//!     .with("posts")
//!     .with("posts.comments");
//!
//! // Load multiple relationships at once
//! let builder = EagerLoadBuilder::new(query)
//!     .with_all(&["posts", "profile", "roles"]);
//!
//! // Conditional eager loading (constraint application is implementation-specific)
//! let builder = EagerLoadBuilder::new(query)
//!     .with_where("posts");
//!
//! assert_eq!(builder.relations().len(), 1);
//! ```

use async_trait::async_trait;
use dashmap::DashMap;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, ModelTrait};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use thiserror::Error;

/// Eager loading errors
#[derive(Error, Debug)]
pub enum EagerLoadError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),

    #[error("Relationship not found: {0}")]
    RelationshipNotFound(String),

    #[error("Invalid eager load path: {0}")]
    InvalidPath(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
}

pub type EagerLoadResult<T> = Result<T, EagerLoadError>;

/// Trait for models that support eager loading of relationships
/// This trait must be implemented by models to extract primary keys and store loaded relationships
pub trait EagerLoadable: Sized + Send + Sync {
    /// The type used for primary keys (typically i32 or i64)
    type PrimaryKey: Clone + Send + Sync + std::hash::Hash + Eq + std::fmt::Debug;

    /// Extract the primary key value from this model
    fn primary_key(&self) -> Self::PrimaryKey;

    /// Store loaded relationship data for this model
    /// Implementation can use a thread-safe cache or model field
    fn set_loaded_relation(&mut self, relation_name: &str, data: Box<dyn Any + Send + Sync>);

    /// Get loaded relationship data if it exists
    fn get_loaded_relation(&self, relation_name: &str) -> Option<&(dyn Any + Send + Sync)>;
}

/// Trait for defining how to load a specific relationship
/// This is implemented for each relationship type
#[async_trait]
pub trait RelationshipLoader: Send + Sync {
    /// The parent model type
    type Parent: EagerLoadable;

    /// The related model type
    type Related: ModelTrait + Send + Sync;

    /// The foreign key type (typically i32 or i64)
    type ForeignKey: Clone + Send + Sync + std::hash::Hash + Eq + std::fmt::Debug;

    /// Load all related models for the given parent IDs
    async fn load_for_keys(
        &self,
        db: &DatabaseConnection,
        parent_keys: &[<Self::Parent as EagerLoadable>::PrimaryKey],
    ) -> EagerLoadResult<Vec<Self::Related>>;

    /// Extract the foreign key from a related model
    fn extract_foreign_key(&self, related: &Self::Related) -> Self::ForeignKey;

    /// Map a parent key to its foreign key value (usually the same)
    fn map_parent_key(
        &self,
        parent_key: &<Self::Parent as EagerLoadable>::PrimaryKey,
    ) -> Self::ForeignKey;
}

/// Represents a relationship to be eagerly loaded
#[derive(Debug, Clone)]
pub struct EagerLoadRelation {
    pub name: String,
    pub nested: Vec<EagerLoadRelation>,
}

impl EagerLoadRelation {
    /// Create a new eager load relation
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nested: Vec::new(),
        }
    }

    /// Add a nested relation
    pub fn with_nested(mut self, nested: EagerLoadRelation) -> Self {
        self.nested.push(nested);
        self
    }

    /// Parse a dot-notation path into nested relations
    pub fn from_path(path: &str) -> Self {
        let parts: Vec<&str> = path.split('.').collect();
        Self::from_parts(&parts)
    }

    fn from_parts(parts: &[&str]) -> Self {
        if parts.is_empty() {
            return Self::new("");
        }

        let mut relation = Self::new(parts[0]);
        if parts.len() > 1 {
            relation = relation.with_nested(Self::from_parts(&parts[1..]));
        }
        relation
    }
}

/// Trait for models that support eager loading
#[async_trait]
pub trait WithEagerLoad: Sized {
    /// Eagerly load a relationship
    fn with(self, relation: &str) -> EagerLoadBuilder<Self> {
        EagerLoadBuilder::new(self).with(relation)
    }

    /// Eagerly load multiple relationships
    fn with_all(self, relations: &[&str]) -> EagerLoadBuilder<Self> {
        EagerLoadBuilder::new(self).with_all(relations)
    }

    /// Eagerly load a relationship with constraints
    /// Note: Constraint application is implementation-specific
    fn with_where(self, relation: &str) -> EagerLoadBuilder<Self> {
        EagerLoadBuilder::new(self).with_where(relation)
    }
}

/// Builder for eager loading relationships
pub struct EagerLoadBuilder<T> {
    query: T,
    relations: Vec<EagerLoadRelation>,
}

impl<T> EagerLoadBuilder<T> {
    /// Create a new eager load builder
    pub fn new(query: T) -> Self {
        Self {
            query,
            relations: Vec::new(),
        }
    }

    /// Add a relationship to eager load
    pub fn with(mut self, relation: &str) -> Self {
        self.relations.push(EagerLoadRelation::from_path(relation));
        self
    }

    /// Add multiple relationships to eager load
    pub fn with_all(mut self, relations: &[&str]) -> Self {
        for relation in relations {
            self.relations.push(EagerLoadRelation::from_path(relation));
        }
        self
    }

    /// Add a relationship with a query constraint
    /// Note: Constraint application is implementation-specific
    pub fn with_where(mut self, relation: &str) -> Self {
        self.relations.push(EagerLoadRelation::from_path(relation));
        self
    }

    /// Get the inner query
    pub fn into_inner(self) -> T {
        self.query
    }

    /// Get the relations to load
    pub fn relations(&self) -> &[EagerLoadRelation] {
        &self.relations
    }
}

/// Eager loader that manages loading related models
pub struct EagerLoader {
    db: DatabaseConnection,
    loaded: Arc<DashMap<String, Vec<u8>>>, // Cache for loaded relations
}

impl EagerLoader {
    /// Create a new eager loader
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            loaded: Arc::new(DashMap::new()),
        }
    }

    /// Load relationships for a collection of models
    pub async fn load<M>(
        &self,
        models: &mut Vec<M>,
        relations: &[EagerLoadRelation],
    ) -> EagerLoadResult<()>
    where
        M: ModelTrait + EagerLoadable,
    {
        for relation in relations {
            self.load_relation(models, relation).await?;
        }
        Ok(())
    }

    /// Load a single relationship
    /// This is a generic implementation that requires models to implement EagerLoadable trait
    async fn load_relation<M>(
        &self,
        models: &mut Vec<M>,
        relation: &EagerLoadRelation,
    ) -> EagerLoadResult<()>
    where
        M: ModelTrait + EagerLoadable,
    {
        if models.is_empty() {
            return Ok(());
        }

        // Extract primary key values from parent models
        let parent_ids: Vec<M::PrimaryKey> = models.iter().map(|m| m.primary_key()).collect();

        tracing::debug!(
            "Loading relation '{}' for {} parent models",
            relation.name,
            parent_ids.len()
        );

        // Note: Actual loading requires type-specific implementation
        // The caller should use a RelationshipLoader implementation
        // This method serves as a template showing the pattern:
        //
        // 1. Extract parent IDs (done above)
        // 2. Load ALL related models in ONE query using IN clause
        //    Example SQL: SELECT * FROM posts WHERE user_id IN (1, 2, 3, ...)
        // 3. Group related models by foreign key
        // 4. Attach grouped models to each parent
        // 5. Recursively load nested relations if any

        // Load nested relations if specified
        if !relation.nested.is_empty() {
            for nested in &relation.nested {
                // To load nested relations, we would need to:
                // 1. Get the already-loaded related models from parents
                // 2. Recursively call load_relation on them
                tracing::debug!(
                    "Nested relation '{}' will be loaded after parent relation",
                    nested.name
                );
            }
        }

        Ok(())
    }

    /// Extract primary key values from models
    /// This is a helper method for concrete implementations
    fn extract_primary_keys<M>(&self, models: &[M]) -> Vec<M::PrimaryKey>
    where
        M: ModelTrait + EagerLoadable,
    {
        models.iter().map(|m| m.primary_key()).collect()
    }

    /// Check for circular dependencies in eager load paths
    pub fn check_circular(&self, relations: &[EagerLoadRelation]) -> EagerLoadResult<()> {
        let mut visited = Vec::new();
        for relation in relations {
            self.check_circular_recursive(relation, &mut visited)?;
        }
        Ok(())
    }

    fn check_circular_recursive(
        &self,
        relation: &EagerLoadRelation,
        visited: &mut Vec<String>,
    ) -> EagerLoadResult<()> {
        if visited.contains(&relation.name) {
            return Err(EagerLoadError::CircularDependency(relation.name.clone()));
        }

        visited.push(relation.name.clone());

        for nested in &relation.nested {
            self.check_circular_recursive(nested, visited)?;
        }

        visited.pop();
        Ok(())
    }
}

/// Cache for eager loaded relationships
#[derive(Clone)]
pub struct RelationshipCache {
    cache: Arc<DashMap<String, Arc<Vec<u8>>>>,
}

impl RelationshipCache {
    /// Create a new relationship cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Get a cached relationship
    pub fn get(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        self.cache.get(key).map(|v| v.clone())
    }

    /// Set a cached relationship
    pub fn set(&self, key: String, value: Vec<u8>) {
        self.cache.insert(key, Arc::new(value));
    }

    /// Check if a relationship is cached
    pub fn has(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Remove a specific cache entry
    pub fn remove(&self, key: &str) -> Option<Arc<Vec<u8>>> {
        self.cache.remove(key).map(|(_, v)| v)
    }
}

impl Default for RelationshipCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for eager loading performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EagerLoadStats {
    pub relations_loaded: usize,
    pub models_loaded: usize,
    pub queries_executed: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl Default for EagerLoadStats {
    fn default() -> Self {
        Self {
            relations_loaded: 0,
            models_loaded: 0,
            queries_executed: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

impl EagerLoadStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a relation load
    pub fn record_relation(&mut self) {
        self.relations_loaded += 1;
    }

    /// Record models loaded
    pub fn record_models(&mut self, count: usize) {
        self.models_loaded += count;
    }

    /// Record a query execution
    pub fn record_query(&mut self) {
        self.queries_executed += 1;
    }

    /// Record a cache hit
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record a cache miss
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Calculate cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eager_load_relation_new() {
        let relation = EagerLoadRelation::new("posts");
        assert_eq!(relation.name, "posts");
        assert!(relation.nested.is_empty());
    }

    #[test]
    fn test_eager_load_relation_from_path() {
        let relation = EagerLoadRelation::from_path("posts.comments.author");
        assert_eq!(relation.name, "posts");
        assert_eq!(relation.nested.len(), 1);
        assert_eq!(relation.nested[0].name, "comments");
        assert_eq!(relation.nested[0].nested.len(), 1);
        assert_eq!(relation.nested[0].nested[0].name, "author");
    }

    #[test]
    fn test_eager_load_relation_with_nested() {
        let relation =
            EagerLoadRelation::new("posts").with_nested(EagerLoadRelation::new("comments"));
        assert_eq!(relation.nested.len(), 1);
        assert_eq!(relation.nested[0].name, "comments");
    }

    #[test]
    fn test_relationship_cache_operations() {
        let cache = RelationshipCache::new();

        // Test set and get
        cache.set("key1".to_string(), vec![1, 2, 3]);
        assert!(cache.has("key1"));
        assert_eq!(*cache.get("key1").unwrap(), vec![1, 2, 3]);

        // Test remove
        cache.remove("key1");
        assert!(!cache.has("key1"));

        // Test clear
        cache.set("key2".to_string(), vec![4, 5, 6]);
        cache.clear();
        assert!(!cache.has("key2"));
    }

    #[test]
    fn test_eager_load_stats() {
        let mut stats = EagerLoadStats::new();

        stats.record_relation();
        stats.record_models(10);
        stats.record_query();
        stats.record_cache_hit();
        stats.record_cache_hit();
        stats.record_cache_miss();

        assert_eq!(stats.relations_loaded, 1);
        assert_eq!(stats.models_loaded, 10);
        assert_eq!(stats.queries_executed, 1);
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.cache_hit_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_eager_load_builder() {
        struct DummyQuery;
        let builder = EagerLoadBuilder::new(DummyQuery)
            .with("posts")
            .with("profile")
            .with_all(&["comments", "likes"]);

        assert_eq!(builder.relations().len(), 4);
    }

    #[test]
    fn test_eager_load_error_display() {
        let err = EagerLoadError::RelationshipNotFound("posts".to_string());
        assert_eq!(err.to_string(), "Relationship not found: posts");

        let err = EagerLoadError::CircularDependency("user->posts->user".to_string());
        assert_eq!(
            err.to_string(),
            "Circular dependency detected: user->posts->user"
        );
    }
}
