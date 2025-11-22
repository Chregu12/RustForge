use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, ModelTrait, Related};

/// Simplified Relationship helpers for Laravel-like syntax
///
/// Note: SeaORM already has a powerful relationship system.
/// These helpers provide Laravel-style convenience methods.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::RelationshipHelpers;
/// use sea_orm::entity::prelude::*;
///
/// // Define relationships in SeaORM's way (already done)
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// pub enum Relation {
///     #[sea_orm(belongs_to = "user::Entity", from = "Column::UserId", to = "user::Column::Id")]
///     User,
/// }
///
/// impl Related<user::Entity> for Entity {
///     fn to() -> RelationDef {
///         Relation::User.def()
///     }
/// }
///
/// // Use Laravel-style helpers
/// let user = post.load_belongs_to::<user::Entity>(&db).await?;
/// ```

/// Helper trait for loading relationships
#[async_trait]
pub trait RelationshipHelpers: ModelTrait + Sized {
    /// Load a BelongsTo relationship
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let post = Post::find_by_id(1).one(&db).await?.unwrap();
    /// let author = post.load_belongs_to::<User>(&db).await?;
    /// ```
    async fn load_belongs_to<R>(&self, db: &DatabaseConnection) -> Result<Option<R::Model>, DbErr>
    where
        R: EntityTrait,
        Self::Entity: Related<R>,
    {
        self.find_related(R::default()).one(db).await
    }

    /// Load a HasMany relationship
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let user = User::find_by_id(1).one(&db).await?.unwrap();
    /// let posts = user.load_has_many::<Post>(&db).await?;
    /// ```
    async fn load_has_many<R>(&self, db: &DatabaseConnection) -> Result<Vec<R::Model>, DbErr>
    where
        R: EntityTrait,
        Self::Entity: Related<R>,
    {
        self.find_related(R::default()).all(db).await
    }
}

// Blanket implementation for all ModelTrait types
impl<T> RelationshipHelpers for T where T: ModelTrait {}

/// Eager loading helper
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::eager_load;
///
/// let posts = Post::find().all(&db).await?;
/// let with_authors = eager_load::<Post, User>(posts, &db).await?;
/// ```
pub async fn eager_load<E, R>(
    models: Vec<E::Model>,
    db: &DatabaseConnection,
) -> Result<Vec<(E::Model, Vec<R::Model>)>, DbErr>
where
    E: EntityTrait,
    R: EntityTrait,
    E: Related<R>,
    E::Model: ModelTrait,
{
    let mut result = Vec::new();

    for model in models {
        let related = model.find_related(R::default()).all(db).await?;
        result.push((model, related));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationships() {
        // Trait definitions compile
    }
}
