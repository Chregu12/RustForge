use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};

use crate::query_builder::QueryBuilder;

/// Base Model trait for Eloquent-like models
///
/// This trait provides a Laravel-style API for database operations.
///
/// # Example
///
/// ```rust,no_run
/// use sea_orm::entity::prelude::*;
///
/// #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #[sea_orm(table_name = "posts")]
/// pub struct Model {
///     #[sea_orm(primary_key)]
///     pub id: i32,
///     pub title: String,
///     pub body: String,
///     pub published: bool,
///     pub created_at: DateTimeUtc,
///     pub updated_at: DateTimeUtc,
/// }
///
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// pub enum Relation {}
///
/// impl ActiveModelBehavior for ActiveModel {}
/// ```
#[async_trait]
pub trait Model: EntityTrait
where
    Self: Sized,
{
    /// Create a new query builder for this model
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::Model;
    /// # use sea_orm::DatabaseConnection;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_eq(post::Column::Published, true)
    ///     .order_by_desc(post::Column::Id)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    fn query(db: DatabaseConnection) -> QueryBuilder<Self> {
        QueryBuilder::new(db)
    }

    /// Get all models
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::Model;
    /// # use sea_orm::DatabaseConnection;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub title: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::all(&db).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn all(db: &DatabaseConnection) -> Result<Vec<Self::Model>, DbErr> {
        Self::find().all(db).await
    }
}

// Blanket implementation for all EntityTrait types
impl<T> Model for T where T: EntityTrait {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_trait() {
        // This test just ensures the trait compiles
    }
}
