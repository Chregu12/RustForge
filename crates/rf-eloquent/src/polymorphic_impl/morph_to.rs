//! # MorphTo Relationship
//!
//! Defines a polymorphic belongs-to relationship where a model can belong to
//! multiple other model types on a single association.
//!
//! ## Example
//!
//! ```rust,ignore
//! // Comment can belong to Post OR Video
//! pub struct Comment {
//!     pub id: i64,
//!     pub commentable_type: String,  // "Post" or "Video"
//!     pub commentable_id: i64,
//!     pub body: String,
//! }
//!
//! impl Comment {
//!     pub fn commentable<T: Model>(&self) -> MorphTo<T> {
//!         MorphTo::new(self.id, "commentable")
//!     }
//! }
//!
//! // Usage
//! let comment = Comment::find(1).await?;
//! let post = comment.commentable::<Post>().get(&db).await?;
//! ```

use super::polymorphic::{PolymorphicError, PolymorphicResult};
use super::type_registry::{TypeResolver, GLOBAL_TYPE_REGISTRY};
use async_trait::async_trait;
use sea_orm::{
    sea_query::{Expr, SimpleExpr},
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, FromQueryResult, QueryFilter,
};
use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;

/// MorphTo relationship - belongs to multiple model types
///
/// This relationship allows a model to belong to different types of parent models.
/// It uses two columns: {name}_type and {name}_id to determine which model to load.
#[derive(Debug, Clone)]
pub struct MorphTo<T> {
    /// ID of the model that owns this relationship
    owner_id: i64,
    /// Name of the morph relation (e.g., "commentable")
    relation_name: String,
    /// Phantom data for the target type
    _phantom: PhantomData<T>,
}

impl<T> MorphTo<T> {
    /// Create a new MorphTo relationship
    ///
    /// # Arguments
    ///
    /// * `owner_id` - The ID of the owning model
    /// * `relation_name` - The morph relation name (e.g., "commentable")
    pub fn new(owner_id: i64, relation_name: impl Into<String>) -> Self {
        Self {
            owner_id,
            relation_name: relation_name.into(),
            _phantom: PhantomData,
        }
    }

    /// Get the morph type column name
    pub fn morph_type_column(&self) -> String {
        format!("{}_type", self.relation_name)
    }

    /// Get the morph id column name
    pub fn morph_id_column(&self) -> String {
        format!("{}_id", self.relation_name)
    }

    /// Get the relation name
    pub fn relation_name(&self) -> &str {
        &self.relation_name
    }
}

/// MorphTo relationship query builder
impl<T> MorphTo<T>
where
    T: Send + Sync + 'static,
{
    /// Load the polymorphic parent model
    ///
    /// This requires:
    /// 1. The owner model to have {name}_type and {name}_id columns
    /// 2. The type to be registered in the GLOBAL_TYPE_REGISTRY
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let comment = Comment::find(1).await?;
    /// let parent = comment.commentable::<Post>().get(&db).await?;
    /// ```
    pub async fn get(
        &self,
        db: Arc<DatabaseConnection>,
        morph_type: &str,
        morph_id: i64,
    ) -> PolymorphicResult<Option<T>> {
        // Resolve the type from the registry
        let resolved = GLOBAL_TYPE_REGISTRY
            .resolve(morph_type, morph_id, db)
            .await?;

        // Downcast to the expected type
        match resolved.downcast::<T>() {
            Ok(model) => Ok(Some(*model)),
            Err(_) => Err(PolymorphicError::TypeMismatch {
                expected: std::any::type_name::<T>().to_string(),
                actual: morph_type.to_string(),
            }),
        }
    }

    /// Load the polymorphic parent using dynamic type resolution
    ///
    /// This method works with any registered type, using the type registry
    /// to dynamically resolve the correct model.
    pub async fn get_dynamic(
        &self,
        db: Arc<DatabaseConnection>,
        morph_type: &str,
        morph_id: i64,
    ) -> PolymorphicResult<Box<dyn Any + Send + Sync>> {
        GLOBAL_TYPE_REGISTRY.resolve(morph_type, morph_id, db).await
    }
}

/// Helper trait for models that can be the owner in a MorphTo relationship
pub trait HasMorphTo {
    /// Get the morph type value for a relation
    fn get_morph_type(&self, relation_name: &str) -> Option<String>;

    /// Get the morph ID value for a relation
    fn get_morph_id(&self, relation_name: &str) -> Option<i64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morph_to_new() {
        let morph_to = MorphTo::<String>::new(1, "commentable");
        assert_eq!(morph_to.owner_id, 1);
        assert_eq!(morph_to.relation_name, "commentable");
    }

    #[test]
    fn test_morph_to_column_names() {
        let morph_to = MorphTo::<String>::new(1, "commentable");
        assert_eq!(morph_to.morph_type_column(), "commentable_type");
        assert_eq!(morph_to.morph_id_column(), "commentable_id");
    }

    #[test]
    fn test_morph_to_relation_name() {
        let morph_to = MorphTo::<String>::new(1, "imageable");
        assert_eq!(morph_to.relation_name(), "imageable");
    }

    #[tokio::test]
    async fn test_morph_to_get_type_not_registered() {
        let morph_to = MorphTo::<String>::new(1, "commentable");
        let db = Arc::new(DatabaseConnection::default());

        let result = morph_to.get(db, "UnknownType", 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_morph_to_with_registry() {
        // Register a test type
        GLOBAL_TYPE_REGISTRY
            .register("TestModel", |id, _db| {
                Box::pin(async move {
                    Ok(Box::new(format!("Model-{}", id)) as Box<dyn Any + Send + Sync>)
                })
            })
            .await;

        let morph_to = MorphTo::<String>::new(1, "testable");
        let db = Arc::new(DatabaseConnection::default());

        let result = morph_to.get(db, "TestModel", 42).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), "Model-42");
    }

    #[tokio::test]
    async fn test_morph_to_get_dynamic() {
        // Register a test type
        GLOBAL_TYPE_REGISTRY
            .register("DynamicModel", |id, _db| {
                Box::pin(async move { Ok(Box::new(id * 2) as Box<dyn Any + Send + Sync>) })
            })
            .await;

        let morph_to = MorphTo::<i64>::new(1, "testable");
        let db = Arc::new(DatabaseConnection::default());

        let result = morph_to.get_dynamic(db, "DynamicModel", 21).await;
        assert!(result.is_ok());

        let value = result.unwrap().downcast::<i64>().unwrap();
        assert_eq!(*value, 42);
    }
}
