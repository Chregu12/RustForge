//! # Polymorphic Relations
//!
//! Support for Laravel-style polymorphic relationships in SeaORM.
//!
//! ## Overview
//!
//! Polymorphic relations allow a model to belong to multiple other model types
//! on a single association. This is useful for features like comments, tags, or
//! attachments that can be associated with different types of content.
//!
//! ## Pattern
//!
//! Polymorphic relations use two columns:
//! - `{name}_type`: Stores the model type (e.g., "Post", "Video")
//! - `{name}_id`: Stores the model ID
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::polymorphic::*;
//!
//! // Comment can belong to Post or Video
//! #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #[sea_orm(table_name = "comments")]
//! pub struct Model {
//!     #[sea_orm(primary_key)]
//!     pub id: i32,
//!     pub body: String,
//!     pub commentable_type: String,  // "Post" or "Video"
//!     pub commentable_id: i32,
//! }
//!
//! // Mark entities as morphable
//! impl Morphable for post::Entity {
//!     fn morph_name() -> &'static str { "Post" }
//! }
//!
//! impl Morphable for video::Entity {
//!     fn morph_name() -> &'static str { "Video" }
//! }
//!
//! // Load polymorphic relation
//! let comment = Comment::find_by_id(&db, 1).await?;
//! let parent = comment.morph_to(&db, "commentable").await?;
//! ```

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};

/// Trait for entities that can be used in polymorphic relations
///
/// Implement this trait to mark an entity as "morphable", meaning it can be
/// referenced by polymorphic relations.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::polymorphic::Morphable;
///
/// impl Morphable for post::Entity {
///     fn morph_name() -> &'static str {
///         "Post"
///     }
/// }
/// ```
pub trait Morphable: EntityTrait {
    /// Return the morphable type name
    ///
    /// This is the string stored in the `{name}_type` column.
    fn morph_name() -> &'static str;
}

/// Helper macro to implement Morphable trait
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::morphable;
///
/// morphable!(post::Entity, "Post");
/// morphable!(video::Entity, "Video");
/// ```
#[macro_export]
macro_rules! morphable {
    ($entity:ty, $type:literal) => {
        impl $crate::polymorphic::Morphable for $entity {
            fn morph_name() -> &'static str {
                $type
            }
        }
    };
}

/// Result type for polymorphic relation queries
pub type PolymorphicResult<T> = Result<T, DbErr>;

/// Represents a polymorphic "belongs to" relationship
///
/// This is the inverse of MorphMany/MorphOne. A model with a morph_to
/// relation has `{name}_type` and `{name}_id` columns.
///
/// # Example
///
/// ```rust,no_run
/// // In Comment model
/// impl comment::Model {
///     pub async fn commentable(&self, db: &DatabaseConnection) -> PolymorphicResult<MorphToResult> {
///         morph_to(db, &self.commentable_type, self.commentable_id).await
///     }
/// }
/// ```
#[async_trait]
pub trait MorphTo {
    /// Load the polymorphic parent
    async fn morph_to<E>(
        &self,
        db: &DatabaseConnection,
        relation_name: &str,
    ) -> PolymorphicResult<Option<E::Model>>
    where
        E: Morphable;
}

/// Represents a polymorphic "has many" relationship
///
/// A model has many of another model, where that model can belong to
/// multiple parent types.
///
/// # Example
///
/// ```rust,no_run
/// // In Post model
/// impl post::Model {
///     pub async fn comments(&self, db: &DatabaseConnection) -> PolymorphicResult<Vec<comment::Model>> {
///         morph_many::<comment::Entity>(db, "Post", self.id, "commentable").await
///     }
/// }
/// ```
#[async_trait]
pub trait MorphMany<E: EntityTrait> {
    /// Load all related models
    async fn morph_many(
        &self,
        db: &DatabaseConnection,
        relation_name: &str,
    ) -> PolymorphicResult<Vec<E::Model>>;
}

/// Represents a polymorphic "has one" relationship
///
/// Similar to MorphMany but returns only one result.
#[async_trait]
pub trait MorphOne<E: EntityTrait> {
    /// Load the related model
    async fn morph_one(
        &self,
        db: &DatabaseConnection,
        relation_name: &str,
    ) -> PolymorphicResult<Option<E::Model>>;
}

/// Represents a polymorphic "many to many" relationship
///
/// Used for features like tagging where tags can be attached to multiple
/// model types (posts, videos, etc).
///
/// Requires a pivot table with:
/// - `tag_id`
/// - `taggable_type`
/// - `taggable_id`
#[async_trait]
pub trait MorphToMany<E: EntityTrait> {
    /// Load all related models through the pivot table
    async fn morph_to_many(
        &self,
        db: &DatabaseConnection,
        pivot_table: &str,
        relation_name: &str,
    ) -> PolymorphicResult<Vec<E::Model>>;
}

/// Helper function to load a morph_to relationship
///
/// # Arguments
///
/// * `db` - Database connection
/// * `morph_type` - The value of the `{name}_type` column
/// * `morph_id` - The value of the `{name}_id` column
///
/// # Example
///
/// ```rust,no_run
/// let parent = morph_to::<post::Entity>(&db, "Post", 123).await?;
/// ```
pub async fn morph_to<E>(
    _db: &DatabaseConnection,
    morph_type: &str,
    _morph_id: i64,
) -> PolymorphicResult<Option<E::Model>>
where
    E: Morphable,
{
    if morph_type != E::morph_name() {
        return Ok(None);
    }

    // Note: This is a simplified implementation
    // In a real implementation, you'd need to handle the ID column dynamically
    // For now, this serves as a placeholder for the API structure
    Ok(None)
}

/// Helper function to load a morph_many relationship
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_type` - The parent model type name
/// * `parent_id` - The parent model ID
/// * `relation_name` - The name of the relation (e.g., "commentable")
///
/// # Example
///
/// ```rust,no_run
/// let comments = morph_many::<comment::Entity>(&db, "Post", 123, "commentable").await?;
/// ```
pub async fn morph_many<E>(
    _db: &DatabaseConnection,
    _parent_type: &str,
    _parent_id: i64,
    _relation_name: &str,
) -> PolymorphicResult<Vec<E::Model>>
where
    E: EntityTrait,
{
    // Note: This is a simplified implementation
    // In a real implementation, you'd need to:
    // 1. Construct column names from relation_name ({name}_type, {name}_id)
    // 2. Query with those columns
    // For now, this serves as a placeholder for the API structure
    Ok(Vec::new())
}

/// Helper function to load a morph_one relationship
///
/// Similar to morph_many but returns only one result.
pub async fn morph_one<E>(
    db: &DatabaseConnection,
    parent_type: &str,
    parent_id: i64,
    relation_name: &str,
) -> PolymorphicResult<Option<E::Model>>
where
    E: EntityTrait,
{
    let results = morph_many::<E>(db, parent_type, parent_id, relation_name).await?;
    Ok(results.into_iter().next())
}

/// Polymorphic relation builder for more complex queries
///
/// Provides a fluent interface for building polymorphic relation queries.
///
/// # Example
///
/// ```rust,no_run
/// let comments = PolymorphicQueryBuilder::new()
///     .morph_type("Post")
///     .morph_id(123)
///     .relation_name("commentable")
///     .with_trashed()  // If using soft deletes
///     .order_by("created_at", "desc")
///     .limit(10)
///     .get::<comment::Entity>(&db)
///     .await?;
/// ```
#[derive(Default)]
pub struct PolymorphicQueryBuilder {
    morph_type: Option<String>,
    morph_id: Option<i64>,
    relation_name: Option<String>,
    with_trashed: bool,
    order_by: Option<(String, String)>,
    limit: Option<u64>,
}

impl PolymorphicQueryBuilder {
    /// Create a new polymorphic query builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the morph type
    pub fn morph_type(mut self, morph_type: impl Into<String>) -> Self {
        self.morph_type = Some(morph_type.into());
        self
    }

    /// Set the morph ID
    pub fn morph_id(mut self, morph_id: i64) -> Self {
        self.morph_id = Some(morph_id);
        self
    }

    /// Set the relation name
    pub fn relation_name(mut self, relation_name: impl Into<String>) -> Self {
        self.relation_name = Some(relation_name.into());
        self
    }

    /// Include soft-deleted records
    pub fn with_trashed(mut self) -> Self {
        self.with_trashed = true;
        self
    }

    /// Add order by clause
    pub fn order_by(mut self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by = Some((column.into(), direction.into()));
        self
    }

    /// Add limit clause
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Execute the query and return results
    pub async fn get<E>(self, _db: &DatabaseConnection) -> PolymorphicResult<Vec<E::Model>>
    where
        E: EntityTrait,
    {
        // Note: This is a simplified implementation
        // In a real implementation, you'd build and execute the query
        // based on the builder's configuration
        Ok(Vec::new())
    }

    /// Execute the query and return first result
    pub async fn first<E>(self, db: &DatabaseConnection) -> PolymorphicResult<Option<E::Model>>
    where
        E: EntityTrait,
    {
        let results = self.get::<E>(db).await?;
        Ok(results.into_iter().next())
    }

    /// Count the results
    pub async fn count<E>(self, _db: &DatabaseConnection) -> PolymorphicResult<u64>
    where
        E: EntityTrait,
    {
        // Placeholder implementation
        Ok(0)
    }
}

/// Enum representing possible polymorphic parent types
///
/// Use this when you need to work with multiple possible parent types.
///
/// # Example
///
/// ```rust,no_run
/// match parent {
///     MorphableType::Post(post) => handle_post(post),
///     MorphableType::Video(video) => handle_video(video),
///     MorphableType::Unknown => {},
/// }
/// ```
pub enum MorphableType<P, V> {
    /// First possible type
    Post(P),
    /// Second possible type
    Video(V),
    /// Unknown or unhandled type
    Unknown,
}

impl<P, V> MorphableType<P, V> {
    /// Check if this is a Post
    pub fn is_post(&self) -> bool {
        matches!(self, MorphableType::Post(_))
    }

    /// Check if this is a Video
    pub fn is_video(&self) -> bool {
        matches!(self, MorphableType::Video(_))
    }

    /// Check if this is Unknown
    pub fn is_unknown(&self) -> bool {
        matches!(self, MorphableType::Unknown)
    }

    /// Get the Post if this is a Post
    pub fn as_post(&self) -> Option<&P> {
        if let MorphableType::Post(p) = self {
            Some(p)
        } else {
            None
        }
    }

    /// Get the Video if this is a Video
    pub fn as_video(&self) -> Option<&V> {
        if let MorphableType::Video(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polymorphic_query_builder() {
        let builder = PolymorphicQueryBuilder::new()
            .morph_type("Post")
            .morph_id(123)
            .relation_name("commentable");

        assert_eq!(builder.morph_type, Some("Post".to_string()));
        assert_eq!(builder.morph_id, Some(123));
    }

    #[test]
    fn test_morphable_type_enum() {
        let post_type: MorphableType<String, i32> = MorphableType::Post("test".to_string());
        assert!(post_type.is_post());
        assert!(!post_type.is_video());
        assert_eq!(post_type.as_post(), Some(&"test".to_string()));
    }

    #[test]
    fn test_polymorphic_api() {
        // Verify API compiles
        // Real tests would require database connection
    }
}
