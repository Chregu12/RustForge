//! # MorphToMany (Polymorphic Many-to-Many Relationships)
//!
//! Support for Laravel-style polymorphic many-to-many relationships, allowing
//! a model to belong to multiple other model types through a pivot table.
//!
//! ## Overview
//!
//! Polymorphic many-to-many relationships are useful for features like:
//! - Tag systems where Posts, Videos, and Articles can all be tagged
//! - Like systems where users can like different content types
//! - Comment systems where comments can be attached to various models
//!
//! ## Pattern
//!
//! The pivot table stores:
//! - `{related}_id` - The ID of the related model (e.g., tag_id)
//! - `{name}_type` - The type of the parent model (e.g., "Post", "Video")
//! - `{name}_id` - The ID of the parent model
//!
//! ## Database Schema Example
//!
//! ```sql
//! -- Tags table
//! CREATE TABLE tags (
//!     id BIGINT PRIMARY KEY,
//!     name VARCHAR(255)
//! );
//!
//! -- Taggables pivot table
//! CREATE TABLE taggables (
//!     tag_id BIGINT,
//!     taggable_type VARCHAR(255),  -- "Post", "Video", etc.
//!     taggable_id BIGINT,
//!     created_at TIMESTAMP,
//!     FOREIGN KEY (tag_id) REFERENCES tags(id)
//! );
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::relationships::morph_to_many::*;
//!
//! // Mark entities as morphable
//! morphable!(post::Entity, "Post");
//! morphable!(video::Entity, "Video");
//!
//! // In Post model implementation
//! impl post::Model {
//!     pub async fn tags(&self, db: &DatabaseConnection) -> MorphToManyResult<Vec<tag::Model>> {
//!         morph_to_many::<tag::Entity>(
//!             db,
//!             "Post",
//!             self.id,
//!             "taggables",
//!             "taggable",
//!         ).await
//!     }
//!
//!     pub async fn attach_tag(&self, db: &DatabaseConnection, tag_id: i64) -> MorphToManyResult<()> {
//!         attach_morph(
//!             db,
//!             "Post",
//!             self.id,
//!             "taggables",
//!             "taggable",
//!             "tag_id",
//!             tag_id,
//!         ).await
//!     }
//! }
//! ```

use async_trait::async_trait;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, Statement,
};

/// Result type for MorphToMany operations
pub type MorphToManyResult<T> = Result<T, DbErr>;

/// Trait for models that can have polymorphic many-to-many relationships
///
/// This trait defines the methods available for polymorphic many-to-many
/// relationships, including attaching, detaching, and syncing related models.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::morph_to_many::MorphToMany;
///
/// // Posts can have tags
/// impl MorphToMany<tag::Entity> for post::Model {
///     // Implementation provided by helper functions
/// }
/// ```
#[async_trait]
pub trait MorphToMany<T: EntityTrait>: Sized {
    /// Load all related models through the polymorphic pivot table
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    /// * `pivot_table` - Name of the pivot table (e.g., "taggables")
    /// * `morph_name` - Prefix for morph columns (e.g., "taggable")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let tags = post.morph_to_many(&db, "taggables", "taggable").await?;
    /// ```
    async fn morph_to_many(
        &self,
        db: &DatabaseConnection,
        pivot_table: &str,
        morph_name: &str,
    ) -> MorphToManyResult<Vec<T::Model>>;

    /// Attach a related model through the pivot table
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    /// * `pivot_table` - Name of the pivot table
    /// * `morph_name` - Prefix for morph columns
    /// * `related_id` - ID of the model to attach
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// post.attach_morph(&db, "taggables", "taggable", tag_id).await?;
    /// ```
    async fn attach_morph(
        &self,
        db: &DatabaseConnection,
        pivot_table: &str,
        morph_name: &str,
        related_id: i64,
    ) -> MorphToManyResult<()>;

    /// Detach a related model from the pivot table
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// post.detach_morph(&db, "taggables", "taggable", tag_id).await?;
    /// ```
    async fn detach_morph(
        &self,
        db: &DatabaseConnection,
        pivot_table: &str,
        morph_name: &str,
        related_id: i64,
    ) -> MorphToManyResult<()>;

    /// Sync related models (detach all and attach new ones)
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    /// * `pivot_table` - Name of the pivot table
    /// * `morph_name` - Prefix for morph columns
    /// * `related_ids` - IDs of models to sync
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// post.sync_morph(&db, "taggables", "taggable", &[1, 2, 3]).await?;
    /// ```
    async fn sync_morph(
        &self,
        db: &DatabaseConnection,
        pivot_table: &str,
        morph_name: &str,
        related_ids: &[i64],
    ) -> MorphToManyResult<()>;

    /// Toggle a related model (attach if not attached, detach if attached)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// post.toggle_morph(&db, "taggables", "taggable", tag_id).await?;
    /// ```
    async fn toggle_morph(
        &self,
        db: &DatabaseConnection,
        pivot_table: &str,
        morph_name: &str,
        related_id: i64,
    ) -> MorphToManyResult<bool>;
}

/// Load all related models through a polymorphic pivot table
///
/// # Arguments
///
/// * `db` - Database connection
/// * `morph_type` - The type name of the parent model (e.g., "Post")
/// * `morph_id` - The ID of the parent model
/// * `pivot_table` - Name of the pivot table (e.g., "taggables")
/// * `morph_name` - Prefix for morph columns (e.g., "taggable")
///
/// # Example
///
/// ```rust,no_run
/// // Get all tags for a post
/// let tags = morph_to_many::<tag::Entity>(
///     &db,
///     "Post",
///     post_id,
///     "taggables",
///     "taggable",
/// ).await?;
/// ```
pub async fn morph_to_many<T>(
    db: &DatabaseConnection,
    morph_type: &str,
    morph_id: i64,
    pivot_table: &str,
    morph_name: &str,
) -> MorphToManyResult<Vec<T::Model>>
where
    T: EntityTrait,
{
    let target_entity = T::default();
    let target_table = target_entity.table_name();
    let target_table_str = target_table.to_string();
    let related_key = format!("{}_id", target_table_str.trim_end_matches('s'));
    let morph_type_col = format!("{}_type", morph_name);
    let morph_id_col = format!("{}_id", morph_name);

    // Build SQL query:
    // SELECT target.*
    // FROM target
    // INNER JOIN pivot ON target.id = pivot.{related}_id
    // WHERE pivot.{morph}_type = ? AND pivot.{morph}_id = ?
    let sql = format!(
        "SELECT {}.*
         FROM {}
         INNER JOIN {} ON {}.id = {}.{}
         WHERE {}.{} = ? AND {}.{} = ?",
        target_table,
        target_table,
        pivot_table,
        target_table,
        pivot_table,
        related_key,
        pivot_table,
        morph_type_col,
        pivot_table,
        morph_id_col
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![
            sea_orm::Value::String(Some(Box::new(morph_type.to_string()))),
            sea_orm::Value::BigInt(Some(morph_id)),
        ],
    );

    T::find().from_raw_sql(stmt).all(db).await
}

/// Attach a related model through a polymorphic pivot table
///
/// # Arguments
///
/// * `db` - Database connection
/// * `morph_type` - The type name of the parent model
/// * `morph_id` - The ID of the parent model
/// * `pivot_table` - Name of the pivot table
/// * `morph_name` - Prefix for morph columns
/// * `related_key` - Column name for the related ID
/// * `related_id` - ID of the model to attach
///
/// # Example
///
/// ```rust,no_run
/// attach_morph(
///     &db,
///     "Post",
///     post_id,
///     "taggables",
///     "taggable",
///     "tag_id",
///     tag_id,
/// ).await?;
/// ```
pub async fn attach_morph(
    db: &DatabaseConnection,
    morph_type: &str,
    morph_id: i64,
    pivot_table: &str,
    morph_name: &str,
    related_key: &str,
    related_id: i64,
) -> MorphToManyResult<()> {
    let morph_type_col = format!("{}_type", morph_name);
    let morph_id_col = format!("{}_id", morph_name);

    // Check if already attached
    let exists = check_morph_exists(
        db,
        pivot_table,
        morph_type,
        morph_id,
        morph_name,
        related_key,
        related_id,
    )
    .await?;

    if exists {
        return Ok(());
    }

    // INSERT INTO pivot (related_id, morph_type, morph_id, created_at)
    // VALUES (?, ?, ?, NOW())
    let sql = format!(
        "INSERT INTO {} ({}, {}, {}, created_at)
         VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
        pivot_table, related_key, morph_type_col, morph_id_col
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![
            sea_orm::Value::BigInt(Some(related_id)),
            sea_orm::Value::String(Some(Box::new(morph_type.to_string()))),
            sea_orm::Value::BigInt(Some(morph_id)),
        ],
    );

    db.execute(stmt).await?;
    Ok(())
}

/// Detach a related model from a polymorphic pivot table
///
/// # Example
///
/// ```rust,no_run
/// detach_morph(
///     &db,
///     "Post",
///     post_id,
///     "taggables",
///     "taggable",
///     "tag_id",
///     tag_id,
/// ).await?;
/// ```
pub async fn detach_morph(
    db: &DatabaseConnection,
    morph_type: &str,
    morph_id: i64,
    pivot_table: &str,
    morph_name: &str,
    related_key: &str,
    related_id: i64,
) -> MorphToManyResult<()> {
    let morph_type_col = format!("{}_type", morph_name);
    let morph_id_col = format!("{}_id", morph_name);

    // DELETE FROM pivot
    // WHERE morph_type = ? AND morph_id = ? AND related_id = ?
    let sql = format!(
        "DELETE FROM {}
         WHERE {} = ? AND {} = ? AND {} = ?",
        pivot_table, morph_type_col, morph_id_col, related_key
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![
            sea_orm::Value::String(Some(Box::new(morph_type.to_string()))),
            sea_orm::Value::BigInt(Some(morph_id)),
            sea_orm::Value::BigInt(Some(related_id)),
        ],
    );

    db.execute(stmt).await?;
    Ok(())
}

/// Sync related models (detach all current and attach new ones)
///
/// # Example
///
/// ```rust,no_run
/// // Replace all tags with new ones
/// sync_morph(
///     &db,
///     "Post",
///     post_id,
///     "taggables",
///     "taggable",
///     "tag_id",
///     &[1, 2, 3],
/// ).await?;
/// ```
pub async fn sync_morph(
    db: &DatabaseConnection,
    morph_type: &str,
    morph_id: i64,
    pivot_table: &str,
    morph_name: &str,
    related_key: &str,
    related_ids: &[i64],
) -> MorphToManyResult<()> {
    let morph_type_col = format!("{}_type", morph_name);
    let morph_id_col = format!("{}_id", morph_name);

    // First, detach all current relations
    let sql = format!(
        "DELETE FROM {}
         WHERE {} = ? AND {} = ?",
        pivot_table, morph_type_col, morph_id_col
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![
            sea_orm::Value::String(Some(Box::new(morph_type.to_string()))),
            sea_orm::Value::BigInt(Some(morph_id)),
        ],
    );

    db.execute(stmt).await?;

    // Then, attach all new relations
    for &related_id in related_ids {
        attach_morph(
            db,
            morph_type,
            morph_id,
            pivot_table,
            morph_name,
            related_key,
            related_id,
        )
        .await?;
    }

    Ok(())
}

/// Toggle a related model (attach if not exists, detach if exists)
///
/// Returns `true` if attached, `false` if detached
///
/// # Example
///
/// ```rust,no_run
/// let attached = toggle_morph(
///     &db,
///     "Post",
///     post_id,
///     "taggables",
///     "taggable",
///     "tag_id",
///     tag_id,
/// ).await?;
///
/// if attached {
///     println!("Tag attached");
/// } else {
///     println!("Tag detached");
/// }
/// ```
pub async fn toggle_morph(
    db: &DatabaseConnection,
    morph_type: &str,
    morph_id: i64,
    pivot_table: &str,
    morph_name: &str,
    related_key: &str,
    related_id: i64,
) -> MorphToManyResult<bool> {
    let exists = check_morph_exists(
        db,
        pivot_table,
        morph_type,
        morph_id,
        morph_name,
        related_key,
        related_id,
    )
    .await?;

    if exists {
        detach_morph(
            db,
            morph_type,
            morph_id,
            pivot_table,
            morph_name,
            related_key,
            related_id,
        )
        .await?;
        Ok(false)
    } else {
        attach_morph(
            db,
            morph_type,
            morph_id,
            pivot_table,
            morph_name,
            related_key,
            related_id,
        )
        .await?;
        Ok(true)
    }
}

/// Check if a morph relation exists in the pivot table
async fn check_morph_exists(
    db: &DatabaseConnection,
    pivot_table: &str,
    morph_type: &str,
    morph_id: i64,
    morph_name: &str,
    related_key: &str,
    related_id: i64,
) -> MorphToManyResult<bool> {
    let morph_type_col = format!("{}_type", morph_name);
    let morph_id_col = format!("{}_id", morph_name);

    let sql = format!(
        "SELECT COUNT(*) as count FROM {}
         WHERE {} = ? AND {} = ? AND {} = ?",
        pivot_table, morph_type_col, morph_id_col, related_key
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![
            sea_orm::Value::String(Some(Box::new(morph_type.to_string()))),
            sea_orm::Value::BigInt(Some(morph_id)),
            sea_orm::Value::BigInt(Some(related_id)),
        ],
    );

    let result = db.query_one(stmt).await?;

    if let Some(row) = result {
        let count: i64 = row.try_get("", "count").unwrap_or(0);
        Ok(count > 0)
    } else {
        Ok(false)
    }
}

/// Query builder for polymorphic many-to-many relationships
///
/// Provides a fluent interface for building complex morph-to-many queries.
///
/// # Example
///
/// ```rust,no_run
/// let tags = MorphToManyBuilder::<tag::Entity>::new(
///     db.clone(),
///     "Post",
///     post_id,
///     "taggables",
///     "taggable",
/// )
/// .where_raw("tags.status = 'active'")
/// .order_by("tags.name", "asc")
/// .limit(10)
/// .get()
/// .await?;
/// ```
pub struct MorphToManyBuilder<T>
where
    T: EntityTrait,
{
    db: DatabaseConnection,
    morph_type: String,
    morph_id: i64,
    pivot_table: String,
    morph_name: String,
    where_clauses: Vec<String>,
    order_by: Option<(String, String)>,
    limit_value: Option<u64>,
    offset_value: Option<u64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> MorphToManyBuilder<T>
where
    T: EntityTrait,
{
    /// Create a new morph-to-many query builder
    pub fn new(
        db: DatabaseConnection,
        morph_type: &str,
        morph_id: i64,
        pivot_table: &str,
        morph_name: &str,
    ) -> Self {
        Self {
            db,
            morph_type: morph_type.to_string(),
            morph_id,
            pivot_table: pivot_table.to_string(),
            morph_name: morph_name.to_string(),
            where_clauses: Vec::new(),
            order_by: None,
            limit_value: None,
            offset_value: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a raw WHERE clause
    pub fn where_raw(mut self, clause: &str) -> Self {
        self.where_clauses.push(clause.to_string());
        self
    }

    /// Add an ORDER BY clause
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by = Some((column.to_string(), direction.to_string()));
        self
    }

    /// Add a LIMIT clause
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Add an OFFSET clause
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Execute the query and return all results
    pub async fn get(self) -> MorphToManyResult<Vec<T::Model>> {
        let target_entity = T::default();
        let target_table = target_entity.table_name();
        let target_table_str = target_table.to_string();
        let related_key = format!("{}_id", target_table_str.trim_end_matches('s'));
        let morph_type_col = format!("{}_type", self.morph_name);
        let morph_id_col = format!("{}_id", self.morph_name);

        let mut sql = format!(
            "SELECT {}.*
             FROM {}
             INNER JOIN {} ON {}.id = {}.{}
             WHERE {}.{} = ? AND {}.{} = ?",
            target_table,
            target_table,
            self.pivot_table,
            target_table,
            self.pivot_table,
            related_key,
            self.pivot_table,
            morph_type_col,
            self.pivot_table,
            morph_id_col
        );

        // Add WHERE clauses
        for clause in &self.where_clauses {
            sql.push_str(&format!(" AND {}", clause));
        }

        // Add ORDER BY
        if let Some((column, direction)) = &self.order_by {
            sql.push_str(&format!(" ORDER BY {} {}", column, direction.to_uppercase()));
        }

        // Add LIMIT
        if let Some(limit) = self.limit_value {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // Add OFFSET
        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &sql,
            vec![
                sea_orm::Value::String(Some(Box::new(self.morph_type.clone()))),
                sea_orm::Value::BigInt(Some(self.morph_id)),
            ],
        );

        T::find().from_raw_sql(stmt).all(&self.db).await
    }

    /// Execute the query and return the first result
    pub async fn first(mut self) -> MorphToManyResult<Option<T::Model>> {
        self.limit_value = Some(1);
        let mut results = self.get().await?;
        Ok(results.pop())
    }

    /// Count the number of results
    pub async fn count(self) -> MorphToManyResult<u64> {
        let results = self.get().await?;
        Ok(results.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morph_to_many_builder() {
        // Verify builder can be created
        // Would need actual entities for full test
    }

    #[test]
    fn test_builder_chaining() {
        // Verify all methods return Self for chaining
    }
}
