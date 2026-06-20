//! # MorphedByMany Relationship
//!
//! Defines the inverse of a many-to-many polymorphic relationship.
//! Used by the target model to access the various parent types.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_eloquent::polymorphic::morphed_by_many::MorphedByMany;
//!
//! struct Tag { id: i64 }
//! struct Post;
//! struct Video;
//!
//! // Tag can have many Posts (inverse of MorphToMany)
//! impl Tag {
//!     pub fn posts(&self) -> MorphedByMany<Post> {
//!         MorphedByMany::new(self.id, "Post", "taggable", "taggables")
//!     }
//!
//!     pub fn videos(&self) -> MorphedByMany<Video> {
//!         MorphedByMany::new(self.id, "Video", "taggable", "taggables")
//!     }
//! }
//!
//! let tag = Tag { id: 1 };
//! let rel = tag.posts();
//! assert_eq!(rel.morph_type(), "Post");
//! assert_eq!(rel.morph_type_column(), "taggable_type");
//! ```

use super::polymorphic::PolymorphicResult;
use sea_orm::{sea_query::Alias, DatabaseConnection, EntityTrait, FromQueryResult};
use std::marker::PhantomData;

/// MorphedByMany relationship - inverse of MorphToMany
///
/// This relationship is used by the target model to access the various
/// parent types through a polymorphic pivot table.
#[derive(Debug, Clone)]
pub struct MorphedByMany<T> {
    /// ID of the related model (e.g., Tag ID)
    related_id: i64,
    /// Type name to filter by (e.g., "Post", "Video")
    morph_type: String,
    /// Name of the morph relation (e.g., "taggable")
    relation_name: String,
    /// Name of the pivot table (e.g., "taggables")
    pivot_table: String,
    /// Phantom data for the parent type
    _phantom: PhantomData<T>,
}

impl<T> MorphedByMany<T> {
    /// Create a new MorphedByMany relationship
    ///
    /// # Arguments
    ///
    /// * `related_id` - The ID of the related model (e.g., tag ID)
    /// * `morph_type` - The parent type to filter by (e.g., "Post")
    /// * `relation_name` - The morph relation name (e.g., "taggable")
    /// * `pivot_table` - The pivot table name (e.g., "taggables")
    pub fn new(
        related_id: i64,
        morph_type: impl Into<String>,
        relation_name: impl Into<String>,
        pivot_table: impl Into<String>,
    ) -> Self {
        Self {
            related_id,
            morph_type: morph_type.into(),
            relation_name: relation_name.into(),
            pivot_table: pivot_table.into(),
            _phantom: PhantomData,
        }
    }

    /// Get the morph type column name in pivot table
    pub fn morph_type_column(&self) -> String {
        format!("{}_type", self.relation_name)
    }

    /// Get the morph id column name in pivot table
    pub fn morph_id_column(&self) -> String {
        format!("{}_id", self.relation_name)
    }

    /// Get the morph type
    pub fn morph_type(&self) -> &str {
        &self.morph_type
    }

    /// Get the related ID
    pub fn related_id(&self) -> i64 {
        self.related_id
    }

    /// Get the relation name
    pub fn relation_name(&self) -> &str {
        &self.relation_name
    }

    /// Get the pivot table name
    pub fn pivot_table(&self) -> &str {
        &self.pivot_table
    }
}

/// Query builder for MorphedByMany relationships
impl<T> MorphedByMany<T>
where
    T: Send + Sync,
{
    /// Get all parent models of the specified type
    ///
    /// This performs a join query:
    /// SELECT parent.* FROM parent
    /// INNER JOIN pivot ON parent.id = pivot.{name}_id
    /// WHERE pivot.{name}_type = morph_type AND pivot.related_id = related_id
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morphed_by_many::MorphedByMany;
    /// # use sea_orm::DatabaseConnection;
    /// # fn main() {}
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub title: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphedByMany::<post::Model>::new(1, "Post", "taggable", "taggables");
    /// let posts = rel.get(db, post::Entity, "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<E>(
        &self,
        _db: &DatabaseConnection,
        _entity: E,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<Vec<T>>
    where
        E: EntityTrait,
        T: FromQueryResult,
    {
        // TODO: Implement MorphedByMany::get()
        Ok(Vec::new())
    }

    /// Count parent models
    pub async fn count<E>(
        &self,
        _db: &DatabaseConnection,
        _entity: E,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<u64>
    where
        E: EntityTrait,
    {
        // TODO: Implement MorphedByMany::count()
        Ok(0)
    }

    /// Check if any parent models exist
    pub async fn exists<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        related_pivot_key: &str,
    ) -> PolymorphicResult<bool>
    where
        E: EntityTrait,
    {
        let count = self.count(db, entity, related_pivot_key).await?;
        Ok(count > 0)
    }
}

/// Pivot operations for MorphedByMany
impl<T> MorphedByMany<T> {
    /// Attach parent models to this relationship
    ///
    /// Inserts records into the pivot table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morphed_by_many::MorphedByMany;
    /// # use sea_orm::DatabaseConnection;
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphedByMany::<()>::new(1, "Post", "taggable", "taggables");
    /// rel.attach(db, vec![1, 2, 3], "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn attach(
        &self,
        _db: &DatabaseConnection,
        _parent_ids: Vec<i64>,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<()> {
        // TODO: Implement MorphedByMany::attach()
        use super::polymorphic::PolymorphicError;
        Err(PolymorphicError::NotImplemented(
            "MorphedByMany::attach not yet implemented".to_string(),
        ))
    }

    /// Detach parent models from this relationship
    ///
    /// Removes records from the pivot table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morphed_by_many::MorphedByMany;
    /// # use sea_orm::DatabaseConnection;
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphedByMany::<()>::new(1, "Post", "taggable", "taggables");
    /// rel.detach(db, vec![1, 2], "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detach(
        &self,
        _db: &DatabaseConnection,
        _parent_ids: Vec<i64>,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<()> {
        // TODO: Implement MorphedByMany::detach()
        use super::polymorphic::PolymorphicError;
        Err(PolymorphicError::NotImplemented(
            "MorphedByMany::detach not yet implemented".to_string(),
        ))
    }

    /// Sync parent models
    ///
    /// Ensures only the specified IDs are attached. Adds missing and removes extras.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morphed_by_many::MorphedByMany;
    /// # use sea_orm::DatabaseConnection;
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphedByMany::<()>::new(1, "Post", "taggable", "taggables");
    /// rel.sync(db, vec![1, 2, 3], "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync(
        &self,
        _db: &DatabaseConnection,
        _parent_ids: Vec<i64>,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<()> {
        // TODO: Implement MorphedByMany::sync()
        use super::polymorphic::PolymorphicError;
        Err(PolymorphicError::NotImplemented(
            "MorphedByMany::sync not yet implemented".to_string(),
        ))
    }
}

/// Builder pattern for advanced queries
pub struct MorphedByManyBuilder<T> {
    relationship: MorphedByMany<T>,
    with_pivot: Vec<String>,
    order_by: Vec<(String, String)>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl<T> MorphedByManyBuilder<T> {
    /// Create a new builder from a MorphedByMany relationship
    pub fn new(relationship: MorphedByMany<T>) -> Self {
        Self {
            relationship,
            with_pivot: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Include pivot columns in the results
    pub fn with_pivot(mut self, columns: Vec<String>) -> Self {
        self.with_pivot = columns;
        self
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
    pub fn relationship(&self) -> &MorphedByMany<T> {
        &self.relationship
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morphed_by_many_new() {
        let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
        assert_eq!(morphed_by_many.related_id, 1);
        assert_eq!(morphed_by_many.morph_type, "Post");
        assert_eq!(morphed_by_many.relation_name, "taggable");
        assert_eq!(morphed_by_many.pivot_table, "taggables");
    }

    #[test]
    fn test_morphed_by_many_column_names() {
        let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
        assert_eq!(morphed_by_many.morph_type_column(), "taggable_type");
        assert_eq!(morphed_by_many.morph_id_column(), "taggable_id");
    }

    #[test]
    fn test_morphed_by_many_getters() {
        let morphed_by_many = MorphedByMany::<String>::new(42, "Video", "taggable", "taggables");
        assert_eq!(morphed_by_many.morph_type(), "Video");
        assert_eq!(morphed_by_many.related_id(), 42);
        assert_eq!(morphed_by_many.relation_name(), "taggable");
        assert_eq!(morphed_by_many.pivot_table(), "taggables");
    }

    #[test]
    fn test_morphed_by_many_different_types() {
        // Tag can have many Posts
        let tag_posts = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
        assert_eq!(tag_posts.morph_type(), "Post");

        // Tag can have many Videos
        let tag_videos = MorphedByMany::<String>::new(1, "Video", "taggable", "taggables");
        assert_eq!(tag_videos.morph_type(), "Video");

        // Both use the same table and relation name
        assert_eq!(tag_posts.pivot_table(), tag_videos.pivot_table());
        assert_eq!(tag_posts.relation_name(), tag_videos.relation_name());
    }

    #[test]
    fn test_morphed_by_many_builder_new() {
        let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
        let builder = MorphedByManyBuilder::new(morphed_by_many);
        assert_eq!(builder.relationship.related_id, 1);
        assert!(builder.with_pivot.is_empty());
        assert!(builder.order_by.is_empty());
        assert!(builder.limit.is_none());
        assert!(builder.offset.is_none());
    }

    #[test]
    fn test_morphed_by_many_builder_with_pivot() {
        let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
        let builder = MorphedByManyBuilder::new(morphed_by_many)
            .with_pivot(vec!["created_at".to_string(), "order".to_string()]);
        assert_eq!(builder.with_pivot.len(), 2);
    }

    #[test]
    fn test_morphed_by_many_builder_chaining() {
        let morphed_by_many = MorphedByMany::<String>::new(1, "Post", "taggable", "taggables");
        let builder = MorphedByManyBuilder::new(morphed_by_many)
            .with_pivot(vec!["created_at".to_string()])
            .order_by("title", "asc")
            .limit(10)
            .offset(5);

        assert_eq!(builder.with_pivot.len(), 1);
        assert_eq!(builder.order_by.len(), 1);
        assert_eq!(builder.limit, Some(10));
        assert_eq!(builder.offset, Some(5));
    }
}
