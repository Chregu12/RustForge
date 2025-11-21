//! # MorphMany Relationship
//!
//! Defines a one-to-many polymorphic relationship where a model has many
//! of another model that can belong to multiple parent types.
//!
//! ## Example
//!
//! ```rust,no_run
//! // Post has many Comments (polymorphic)
//! impl Post {
//!     pub fn comments(&self) -> MorphMany<Comment> {
//!         MorphMany::new(self.id, "Post", "commentable")
//!     }
//! }
//!
//! // Video has many Comments (polymorphic)
//! impl Video {
//!     pub fn comments(&self) -> MorphMany<Comment> {
//!         MorphMany::new(self.id, "Video", "commentable")
//!     }
//! }
//!
//! // Usage
//! let post = Post::find(1).await?;
//! let comments = post.comments().get(&db).await?;
//! ```

use super::polymorphic::{PolymorphicError, PolymorphicResult};
use async_trait::async_trait;
use sea_orm::{
    sea_query::{Expr, SimpleExpr},
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Selector,
};
use std::marker::PhantomData;

/// MorphMany relationship - has many of a polymorphic model
///
/// This is the inverse of MorphTo for one-to-many relationships.
/// The parent model has many instances of a polymorphic child model.
#[derive(Debug, Clone)]
pub struct MorphMany<T> {
    /// ID of the parent model
    parent_id: i64,
    /// Type name of the parent (e.g., "Post", "Video")
    parent_type: String,
    /// Name of the morph relation (e.g., "commentable")
    relation_name: String,
    /// Phantom data for the related type
    _phantom: PhantomData<T>,
}

impl<T> MorphMany<T> {
    /// Create a new MorphMany relationship
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The ID of the parent model
    /// * `parent_type` - The type name of the parent (e.g., "Post")
    /// * `relation_name` - The morph relation name (e.g., "commentable")
    pub fn new(
        parent_id: i64,
        parent_type: impl Into<String>,
        relation_name: impl Into<String>,
    ) -> Self {
        Self {
            parent_id,
            parent_type: parent_type.into(),
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

    /// Get the parent type
    pub fn parent_type(&self) -> &str {
        &self.parent_type
    }

    /// Get the parent ID
    pub fn parent_id(&self) -> i64 {
        self.parent_id
    }

    /// Get the relation name
    pub fn relation_name(&self) -> &str {
        &self.relation_name
    }
}

/// Query builder for MorphMany relationships
impl<T> MorphMany<T> {
    /// Create a where condition for this morph relationship
    ///
    /// This creates: WHERE {name}_type = parent_type AND {name}_id = parent_id
    pub fn where_condition<C>(&self, type_column: C, id_column: C) -> Condition
    where
        C: ColumnTrait,
    {
        Condition::all()
            .add(type_column.eq(&self.parent_type))
            .add(id_column.eq(self.parent_id))
    }
}

/// Advanced query methods for MorphMany
impl<T> MorphMany<T>
where
    T: Send + Sync,
{
    /// Get all related models
    ///
    /// This executes a query with WHERE {name}_type = parent_type AND {name}_id = parent_id
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let post = Post::find(1).await?;
    /// let comments = post.comments().get(&db).await?;
    /// ```
    pub async fn get<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        type_column: E::Column,
        id_column: E::Column,
    ) -> PolymorphicResult<Vec<T>>
    where
        E: EntityTrait,
        T: FromQueryResult,
        E::Column: ColumnTrait,
    {
        let results = E::find()
            .filter(type_column.eq(&self.parent_type))
            .filter(id_column.eq(self.parent_id))
            .into_model::<T>()
            .all(db)
            .await
            .map_err(PolymorphicError::DatabaseError)?;

        Ok(results)
    }

    /// Count related models
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let post = Post::find(1).await?;
    /// let count = post.comments().count(&db).await?;
    /// ```
    pub async fn count<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        type_column: E::Column,
        id_column: E::Column,
    ) -> PolymorphicResult<u64>
    where
        E: EntityTrait,
        E::Column: ColumnTrait,
    {
        // Count by fetching all and getting the length
        // TODO: This is inefficient, should use COUNT(*) query
        let results = E::find()
            .filter(type_column.eq(&self.parent_type))
            .filter(id_column.eq(self.parent_id))
            .all(db)
            .await
            .map_err(PolymorphicError::DatabaseError)?;

        let count = results.len() as u64;

        Ok(count)
    }

    /// Check if any related models exist
    pub async fn exists<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        type_column: E::Column,
        id_column: E::Column,
    ) -> PolymorphicResult<bool>
    where
        E: EntityTrait,
        E::Column: ColumnTrait,
    {
        let count = self.count(db, entity, type_column, id_column).await?;
        Ok(count > 0)
    }
}

/// Builder pattern for advanced queries
pub struct MorphManyBuilder<T> {
    relationship: MorphMany<T>,
    order_by: Vec<(String, String)>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl<T> MorphManyBuilder<T> {
    /// Create a new builder from a MorphMany relationship
    pub fn new(relationship: MorphMany<T>) -> Self {
        Self {
            relationship,
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Add an order by clause
    pub fn order_by(mut self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by.push((column.into(), direction.into()));
        self
    }

    /// Set a limit
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set an offset
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Get the underlying relationship
    pub fn relationship(&self) -> &MorphMany<T> {
        &self.relationship
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morph_many_new() {
        let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
        assert_eq!(morph_many.parent_id, 1);
        assert_eq!(morph_many.parent_type, "Post");
        assert_eq!(morph_many.relation_name, "commentable");
    }

    #[test]
    fn test_morph_many_column_names() {
        let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
        assert_eq!(morph_many.morph_type_column(), "commentable_type");
        assert_eq!(morph_many.morph_id_column(), "commentable_id");
    }

    #[test]
    fn test_morph_many_getters() {
        let morph_many = MorphMany::<String>::new(42, "Video", "imageable");
        assert_eq!(morph_many.parent_type(), "Video");
        assert_eq!(morph_many.parent_id(), 42);
        assert_eq!(morph_many.relation_name(), "imageable");
    }

    #[test]
    fn test_morph_many_builder_new() {
        let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
        let builder = MorphManyBuilder::new(morph_many);
        assert_eq!(builder.relationship.parent_id, 1);
        assert!(builder.order_by.is_empty());
        assert!(builder.limit.is_none());
        assert!(builder.offset.is_none());
    }

    #[test]
    fn test_morph_many_builder_order_by() {
        let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
        let builder = MorphManyBuilder::new(morph_many).order_by("created_at", "desc");
        assert_eq!(builder.order_by.len(), 1);
        assert_eq!(builder.order_by[0].0, "created_at");
        assert_eq!(builder.order_by[0].1, "desc");
    }

    #[test]
    fn test_morph_many_builder_limit() {
        let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
        let builder = MorphManyBuilder::new(morph_many).limit(10);
        assert_eq!(builder.limit, Some(10));
    }

    #[test]
    fn test_morph_many_builder_offset() {
        let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
        let builder = MorphManyBuilder::new(morph_many).offset(20);
        assert_eq!(builder.offset, Some(20));
    }

    #[test]
    fn test_morph_many_builder_chaining() {
        let morph_many = MorphMany::<String>::new(1, "Post", "commentable");
        let builder = MorphManyBuilder::new(morph_many)
            .order_by("created_at", "desc")
            .limit(10)
            .offset(5);

        assert_eq!(builder.order_by.len(), 1);
        assert_eq!(builder.limit, Some(10));
        assert_eq!(builder.offset, Some(5));
    }
}
