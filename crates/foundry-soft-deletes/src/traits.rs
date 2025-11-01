//! Soft delete traits

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait,
    PrimaryKeyTrait, QueryFilter, QuerySelect,
};

/// Trait for soft deletable entities
#[async_trait]
pub trait SoftDelete: EntityTrait
where
    <Self as EntityTrait>::Model: HasSoftDelete + Send + Sync,
{
    /// Soft delete a model
    async fn soft_delete<C>(
        db: &C,
        model: <Self as EntityTrait>::Model,
    ) -> crate::Result<<Self as EntityTrait>::Model>
    where
        C: sea_orm::ConnectionTrait,
    {
        let mut active_model = model.into_active_model();

        // Set deleted_at to current time
        if let Some(deleted_at_field) = active_model
            .get_mut(Self::Column::DeletedAt)
            .and_then(|v| v.as_mut())
        {
            *deleted_at_field = ActiveValue::Set(Some(Utc::now()));
        }

        Ok(active_model.update(db).await?)
    }

    /// Restore a soft-deleted model
    async fn restore<C>(
        db: &C,
        model: <Self as EntityTrait>::Model,
    ) -> crate::Result<<Self as EntityTrait>::Model>
    where
        C: sea_orm::ConnectionTrait,
    {
        let mut active_model = model.into_active_model();

        // Set deleted_at to None
        if let Some(deleted_at_field) = active_model
            .get_mut(Self::Column::DeletedAt)
            .and_then(|v| v.as_mut())
        {
            *deleted_at_field = ActiveValue::Set(None);
        }

        Ok(active_model.update(db).await?)
    }

    /// Force delete (permanently delete)
    async fn force_delete<C>(db: &C, model: <Self as EntityTrait>::Model) -> crate::Result<()>
    where
        C: sea_orm::ConnectionTrait,
    {
        model.delete(db).await?;
        Ok(())
    }

    /// Query only non-deleted records (default behavior)
    fn without_trashed() -> sea_orm::Select<Self> {
        Self::find().filter(Self::Column::DeletedAt.is_null())
    }

    /// Query including soft-deleted records
    fn with_trashed() -> sea_orm::Select<Self> {
        Self::find()
    }

    /// Query only soft-deleted records
    fn only_trashed() -> sea_orm::Select<Self> {
        Self::find().filter(Self::Column::DeletedAt.is_not_null())
    }
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
