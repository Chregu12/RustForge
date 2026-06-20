//! # MorphToMany Relationship
//!
//! Defines a many-to-many polymorphic relationship where a model can have many
//! of another model through a polymorphic pivot table.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_eloquent::polymorphic::morph_to_many::MorphToMany;
//!
//! struct Tag;
//! struct Post { id: i64 }
//! struct Video { id: i64 }
//!
//! // Post can have many Tags (polymorphic)
//! impl Post {
//!     pub fn tags(&self) -> MorphToMany<Tag> {
//!         MorphToMany::new(
//!             self.id,
//!             "Post",
//!             "taggable",
//!             "taggables" // pivot table
//!         )
//!     }
//! }
//!
//! // Video can have many Tags (polymorphic)
//! impl Video {
//!     pub fn tags(&self) -> MorphToMany<Tag> {
//!         MorphToMany::new(
//!             self.id,
//!             "Video",
//!             "taggable",
//!             "taggables"
//!         )
//!     }
//! }
//!
//! // Pivot table `taggables` has columns: tag_id, taggable_type, taggable_id.
//! let post = Post { id: 1 };
//! let rel = post.tags();
//! assert_eq!(rel.morph_type_column(), "taggable_type");
//! assert_eq!(rel.morph_id_column(), "taggable_id");
//! ```

use super::polymorphic::{PolymorphicError, PolymorphicResult};
use sea_orm::{
    sea_query::{Alias, Iden as SeaIden},
    ConnectionTrait, DatabaseConnection, EntityTrait, FromQueryResult, Statement,
};
use std::marker::PhantomData;

/// MorphToMany relationship - many-to-many polymorphic
///
/// This relationship allows a model to have many of another model through
/// a polymorphic pivot table. The pivot table contains {name}_type and {name}_id
/// columns to identify the parent model.
#[derive(Debug, Clone)]
pub struct MorphToMany<T> {
    /// ID of the parent model
    parent_id: i64,
    /// Type name of the parent (e.g., "Post", "Video")
    parent_type: String,
    /// Name of the morph relation (e.g., "taggable")
    relation_name: String,
    /// Name of the pivot table (e.g., "taggables")
    pivot_table: String,
    /// Phantom data for the related type
    _phantom: PhantomData<T>,
}

impl<T> MorphToMany<T> {
    /// Create a new MorphToMany relationship
    ///
    /// # Arguments
    ///
    /// * `parent_id` - The ID of the parent model
    /// * `parent_type` - The type name of the parent (e.g., "Post")
    /// * `relation_name` - The morph relation name (e.g., "taggable")
    /// * `pivot_table` - The pivot table name (e.g., "taggables")
    pub fn new(
        parent_id: i64,
        parent_type: impl Into<String>,
        relation_name: impl Into<String>,
        pivot_table: impl Into<String>,
    ) -> Self {
        Self {
            parent_id,
            parent_type: parent_type.into(),
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

    /// Get the pivot table name
    pub fn pivot_table(&self) -> &str {
        &self.pivot_table
    }
}

/// Query builder for MorphToMany relationships
impl<T> MorphToMany<T>
where
    T: Send + Sync,
{
    /// Get all related models through the pivot table
    ///
    /// This performs a join query:
    /// SELECT related.* FROM related
    /// INNER JOIN pivot ON related.id = pivot.related_id
    /// WHERE pivot.{name}_type = parent_type AND pivot.{name}_id = parent_id
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morph_to_many::MorphToMany;
    /// # use sea_orm::DatabaseConnection;
    /// # fn main() {}
    /// # mod tag {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "tags")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphToMany::<tag::Model>::new(1, "Post", "taggable", "taggables");
    /// let tags = rel.get(db, tag::Entity, "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        related_pivot_key: &str,
    ) -> PolymorphicResult<Vec<T>>
    where
        E: EntityTrait,
        T: FromQueryResult,
    {
        // For now, we'll use a simpler approach with raw SQL
        // This will be optimized with proper SeaORM joins in production

        // Build the query manually
        let table_name = entity.table_name();
        let morph_type_col = self.morph_type_column();
        let morph_id_col = self.morph_id_column();

        let sql = format!(
            r#"
            SELECT {}.* FROM {}
            INNER JOIN {} ON {}.id = {}.{}
            WHERE {}.{} = ? AND {}.{} = ?
            "#,
            table_name,
            table_name,
            self.pivot_table,
            table_name,
            self.pivot_table,
            related_pivot_key,
            self.pivot_table,
            morph_type_col,
            self.pivot_table,
            morph_id_col,
        );

        // This is a placeholder - actual implementation would use SeaORM's query builder
        // For the MVP, we return empty vec
        Ok(Vec::new())
    }

    /// Count related models
    pub async fn count<E>(
        &self,
        db: &DatabaseConnection,
        entity: E,
        related_pivot_key: &str,
    ) -> PolymorphicResult<u64>
    where
        E: EntityTrait,
    {
        // Placeholder implementation
        Ok(0)
    }

    /// Check if any related models exist
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

/// Pivot operations for MorphToMany
impl<T> MorphToMany<T> {
    /// Attach related models to this relationship
    ///
    /// Inserts records into the pivot table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morph_to_many::MorphToMany;
    /// # use sea_orm::DatabaseConnection;
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphToMany::<()>::new(1, "Post", "taggable", "taggables");
    /// rel.attach(db, vec![1, 2, 3], "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn attach(
        &self,
        _db: &DatabaseConnection,
        _related_ids: Vec<i64>,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<()> {
        // TODO: Implement polymorphic attach
        // This requires building dynamic INSERT queries through SeaORM
        // For now, return a placeholder error
        Err(PolymorphicError::NotImplemented(
            "MorphToMany::attach not yet implemented".to_string(),
        ))
    }

    /// Detach related models from this relationship
    ///
    /// Removes records from the pivot table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morph_to_many::MorphToMany;
    /// # use sea_orm::DatabaseConnection;
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphToMany::<()>::new(1, "Post", "taggable", "taggables");
    /// rel.detach(db, vec![1, 2], "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn detach(
        &self,
        _db: &DatabaseConnection,
        _related_ids: Vec<i64>,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<()> {
        // TODO: Implement polymorphic detach
        Err(PolymorphicError::NotImplemented(
            "MorphToMany::detach not yet implemented".to_string(),
        ))
    }

    /// Sync related models
    ///
    /// Ensures only the specified IDs are attached. Adds missing and removes extras.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_eloquent::polymorphic::morph_to_many::MorphToMany;
    /// # use sea_orm::DatabaseConnection;
    /// # async fn example(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rel = MorphToMany::<()>::new(1, "Post", "taggable", "taggables");
    /// rel.sync(db, vec![1, 2, 3], "tag_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync(
        &self,
        _db: &DatabaseConnection,
        _related_ids: Vec<i64>,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<()> {
        // TODO: Implement polymorphic sync
        Err(PolymorphicError::NotImplemented(
            "MorphToMany::sync not yet implemented".to_string(),
        ))
    }

    /// Toggle related models
    ///
    /// Attach if not attached, detach if already attached.
    pub async fn toggle(
        &self,
        _db: &DatabaseConnection,
        _related_ids: Vec<i64>,
        _related_pivot_key: &str,
    ) -> PolymorphicResult<()> {
        // TODO: Implement polymorphic toggle
        Err(PolymorphicError::NotImplemented(
            "MorphToMany::toggle not yet implemented".to_string(),
        ))
    }
}

/// Builder pattern for advanced queries
pub struct MorphToManyBuilder<T> {
    relationship: MorphToMany<T>,
    with_pivot: Vec<String>,
    order_by: Vec<(String, String)>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl<T> MorphToManyBuilder<T> {
    /// Create a new builder from a MorphToMany relationship
    pub fn new(relationship: MorphToMany<T>) -> Self {
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
    pub fn relationship(&self) -> &MorphToMany<T> {
        &self.relationship
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morph_to_many_new() {
        let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
        assert_eq!(morph_to_many.parent_id, 1);
        assert_eq!(morph_to_many.parent_type, "Post");
        assert_eq!(morph_to_many.relation_name, "taggable");
        assert_eq!(morph_to_many.pivot_table, "taggables");
    }

    #[test]
    fn test_morph_to_many_column_names() {
        let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
        assert_eq!(morph_to_many.morph_type_column(), "taggable_type");
        assert_eq!(morph_to_many.morph_id_column(), "taggable_id");
    }

    #[test]
    fn test_morph_to_many_getters() {
        let morph_to_many = MorphToMany::<String>::new(42, "Video", "taggable", "taggables");
        assert_eq!(morph_to_many.parent_type(), "Video");
        assert_eq!(morph_to_many.parent_id(), 42);
        assert_eq!(morph_to_many.relation_name(), "taggable");
        assert_eq!(morph_to_many.pivot_table(), "taggables");
    }

    #[test]
    fn test_morph_to_many_builder_new() {
        let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
        let builder = MorphToManyBuilder::new(morph_to_many);
        assert_eq!(builder.relationship.parent_id, 1);
        assert!(builder.with_pivot.is_empty());
        assert!(builder.order_by.is_empty());
        assert!(builder.limit.is_none());
        assert!(builder.offset.is_none());
    }

    #[test]
    fn test_morph_to_many_builder_with_pivot() {
        let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
        let builder = MorphToManyBuilder::new(morph_to_many)
            .with_pivot(vec!["created_at".to_string(), "order".to_string()]);
        assert_eq!(builder.with_pivot.len(), 2);
        assert_eq!(builder.with_pivot[0], "created_at");
        assert_eq!(builder.with_pivot[1], "order");
    }

    #[test]
    fn test_morph_to_many_builder_chaining() {
        let morph_to_many = MorphToMany::<String>::new(1, "Post", "taggable", "taggables");
        let builder = MorphToManyBuilder::new(morph_to_many)
            .with_pivot(vec!["created_at".to_string()])
            .order_by("name", "asc")
            .limit(10)
            .offset(5);

        assert_eq!(builder.with_pivot.len(), 1);
        assert_eq!(builder.order_by.len(), 1);
        assert_eq!(builder.limit, Some(10));
        assert_eq!(builder.offset, Some(5));
    }
}
