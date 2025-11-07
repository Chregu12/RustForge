//! Soft delete traits

use async_trait::async_trait;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, EntityTrait, ModelTrait,
};

/// Trait for soft deletable entities
///
/// Note: Implementors must ensure their entity has a `deleted_at` column
/// and implement the query methods based on their specific Column enum.
#[async_trait]
pub trait SoftDelete: EntityTrait
where
    <Self as EntityTrait>::Model: HasSoftDelete + Send + Sync,
{
    /// The active model type for this entity
    type ActiveModel: ActiveModelTrait<Entity = Self> + ActiveModelBehavior + Send + Sync + From<<Self as EntityTrait>::Model>;
    /// Soft delete a model (must be implemented by user)
    async fn soft_delete<C>(
        db: &C,
        model: <Self as EntityTrait>::Model,
    ) -> crate::Result<<Self as EntityTrait>::Model>
    where
        C: sea_orm::ConnectionTrait;

    /// Restore a soft-deleted model (must be implemented by user)
    async fn restore<C>(
        db: &C,
        model: <Self as EntityTrait>::Model,
    ) -> crate::Result<<Self as EntityTrait>::Model>
    where
        C: sea_orm::ConnectionTrait;

    /// Force delete (permanently delete)
    async fn force_delete<C>(db: &C, model: <Self as EntityTrait>::Model) -> crate::Result<()>
    where
        C: sea_orm::ConnectionTrait,
    {
        let active_model: Self::ActiveModel = model.into();
        active_model.delete(db).await?;
        Ok(())
    }

    /// Query only non-deleted records (must be implemented by user)
    fn without_trashed() -> sea_orm::Select<Self>;

    /// Query including soft-deleted records
    fn with_trashed() -> sea_orm::Select<Self> {
        Self::find()
    }

    /// Query only soft-deleted records (must be implemented by user)
    fn only_trashed() -> sea_orm::Select<Self>;
}

/// Extension trait for models
#[async_trait]
pub trait SoftDeleteExt: ModelTrait + HasSoftDelete {
    /// Check if model is soft deleted
    fn is_trashed(&self) -> bool {
        self.deleted_at().is_some()
    }

    /// Check if model is not deleted
    fn is_not_trashed(&self) -> bool {
        !self.is_trashed()
    }
}

// Blanket implementation
impl<T> SoftDeleteExt for T where T: ModelTrait + HasSoftDelete {}

use crate::HasSoftDelete;

#[cfg(test)]
mod tests {
    use super::*;

    struct TestModel {
        id: i32,
        deleted_at: Option<DateTime<Utc>>,
    }

    impl HasSoftDelete for TestModel {
        fn deleted_at(&self) -> Option<DateTime<Utc>> {
            self.deleted_at
        }

        fn set_deleted_at(&mut self, value: Option<DateTime<Utc>>) {
            self.deleted_at = value;
        }
    }

    #[test]
    fn test_is_trashed() {
        let model = TestModel {
            id: 1,
            deleted_at: Some(Utc::now()),
        };
        assert!(model.is_trashed());
    }

    #[test]
    fn test_is_not_trashed() {
        let model = TestModel {
            id: 1,
            deleted_at: None,
        };
        assert!(model.is_not_trashed());
    }
}
