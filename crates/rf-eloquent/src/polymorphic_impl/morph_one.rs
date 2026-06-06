//! # MorphOne Relationship
//!
//! Defines a one-to-one polymorphic relationship where a model has one
//! of another model that can belong to multiple parent types.
//!
//! ## Example
//!
//! ```rust,ignore
//! // Post has one Image (polymorphic)
//! impl Post {
//!     pub fn image(&self) -> MorphOne<Image> {
//!         MorphOne::new(self.id, "Post", "imageable")
//!     }
//! }
//!
//! // User has one Avatar (polymorphic)
//! impl User {
//!     pub fn avatar(&self) -> MorphOne<Image> {
//!         MorphOne::new(self.id, "User", "imageable")
//!     }
//! }
//!
//! // Usage
//! let post = Post::find(1).await?;
//! let image = post.image().get(&db).await?;
//! ```

use super::polymorphic::{PolymorphicError, PolymorphicResult};
use async_trait::async_trait;
use sea_orm::{
    sea_query::{Expr, SimpleExpr},
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, FromQueryResult,
    PaginatorTrait, QueryFilter, Selector,
};
use std::marker::PhantomData;

/// MorphOne relationship - has one of a polymorphic model
///
/// This is similar to MorphMany but returns a single result instead of a collection.
/// The parent model has at most one instance of a polymorphic child model.
#[derive(Debug, Clone)]
pub struct MorphOne<T> {
    /// ID of the parent model
    parent_id: i64,
    /// Type name of the parent (e.g., "Post", "User")
    parent_type: String,
    /// Name of the morph relation (e.g., "imageable")
    relation_name: String,
    /// Phantom data for the related type
    _phantom: PhantomData<T>,
}

impl<T> MorphOne<T> {
    /// Create a new MorphOne relationship
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The ID of the parent model
    /// * `parent_type` - The type name of the parent (e.g., "Post")
    /// * `relation_name` - The morph relation name (e.g., "imageable")
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

/// Query builder for MorphOne relationships
impl<T> MorphOne<T> {
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

/// Advanced query methods for MorphOne
impl<T> MorphOne<T>
where
    T: Send + Sync,
{
    /// Get the related model
    ///
    /// This executes a query with WHERE {name}_type = parent_type AND {name}_id = parent_id
    /// Returns the first matching record or None.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let post = Post::find(1).await?;
    /// let image = post.image().get(&db).await?;
    /// ```
    pub async fn get<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        type_column: E::Column,
        id_column: E::Column,
    ) -> PolymorphicResult<Option<T>>
    where
        E: EntityTrait,
        T: FromQueryResult,
        E::Column: ColumnTrait,
    {
        let result = E::find()
            .filter(type_column.eq(&self.parent_type))
            .filter(id_column.eq(self.parent_id))
            .into_model::<T>()
            .one(db)
            .await
            .map_err(PolymorphicError::DatabaseError)?;

        Ok(result)
    }

    /// Check if a related model exists
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let post = Post::find(1).await?;
    /// let has_image = post.image().exists(&db).await?;
    /// ```
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
        // Count by fetching all and getting the length
        // TODO: This is inefficient, should use COUNT(*) query
        let results = E::find()
            .filter(type_column.eq(&self.parent_type))
            .filter(id_column.eq(self.parent_id))
            .all(db)
            .await
            .map_err(PolymorphicError::DatabaseError)?;

        let count = results.len() as u64;

        Ok(count > 0)
    }

    /// Get the related model or fail if not found
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let post = Post::find(1).await?;
    /// let image = post.image().get_or_fail(&db).await?;
    /// ```
    pub async fn get_or_fail<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        type_column: E::Column,
        id_column: E::Column,
    ) -> PolymorphicResult<T>
    where
        E: EntityTrait,
        T: FromQueryResult,
        E::Column: ColumnTrait,
    {
        self.get(db, entity, type_column, id_column)
            .await?
            .ok_or(PolymorphicError::MorphableNotFound)
    }
}

/// Builder pattern for advanced queries
pub struct MorphOneBuilder<T> {
    relationship: MorphOne<T>,
    order_by: Option<(String, String)>,
}

impl<T> MorphOneBuilder<T> {
    /// Create a new builder from a MorphOne relationship
    pub fn new(relationship: MorphOne<T>) -> Self {
        Self {
            relationship,
            order_by: None,
        }
    }

    /// Add an order by clause (useful if there might be multiple matches)
    pub fn order_by(mut self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by = Some((column.into(), direction.into()));
        self
    }

    /// Get the underlying relationship
    pub fn relationship(&self) -> &MorphOne<T> {
        &self.relationship
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morph_one_new() {
        let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
        assert_eq!(morph_one.parent_id, 1);
        assert_eq!(morph_one.parent_type, "Post");
        assert_eq!(morph_one.relation_name, "imageable");
    }

    #[test]
    fn test_morph_one_column_names() {
        let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
        assert_eq!(morph_one.morph_type_column(), "imageable_type");
        assert_eq!(morph_one.morph_id_column(), "imageable_id");
    }

    #[test]
    fn test_morph_one_getters() {
        let morph_one = MorphOne::<String>::new(42, "User", "avatareable");
        assert_eq!(morph_one.parent_type(), "User");
        assert_eq!(morph_one.parent_id(), 42);
        assert_eq!(morph_one.relation_name(), "avatareable");
    }

    #[test]
    fn test_morph_one_builder_new() {
        let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
        let builder = MorphOneBuilder::new(morph_one);
        assert_eq!(builder.relationship.parent_id, 1);
        assert!(builder.order_by.is_none());
    }

    #[test]
    fn test_morph_one_builder_order_by() {
        let morph_one = MorphOne::<String>::new(1, "Post", "imageable");
        let builder = MorphOneBuilder::new(morph_one).order_by("created_at", "desc");
        assert!(builder.order_by.is_some());
        let (col, dir) = builder.order_by.unwrap();
        assert_eq!(col, "created_at");
        assert_eq!(dir, "desc");
    }

    #[test]
    fn test_morph_one_different_types() {
        // Test with Post
        let post_image = MorphOne::<String>::new(1, "Post", "imageable");
        assert_eq!(post_image.parent_type(), "Post");

        // Test with User
        let user_avatar = MorphOne::<String>::new(2, "User", "imageable");
        assert_eq!(user_avatar.parent_type(), "User");

        // Both should use the same relation name
        assert_eq!(post_image.relation_name(), user_avatar.relation_name());
    }
}
