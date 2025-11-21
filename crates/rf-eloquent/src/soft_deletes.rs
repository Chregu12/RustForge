//! # Soft Deletes for rf-eloquent
//!
//! Provides Laravel-style soft delete functionality for Eloquent models.
//! Soft deletes mark records as deleted without actually removing them from the database.
//!
//! ## Usage
//!
//! ```rust
//! use rf_eloquent::soft_deletes::*;
//! use chrono::{DateTime, Utc};
//! use sea_orm::*;
//!
//! #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #[sea_orm(table_name = "users")]
//! pub struct Model {
//!     #[sea_orm(primary_key)]
//!     pub id: i64,
//!     pub name: String,
//!     pub email: String,
//!     pub deleted_at: Option<DateTime<Utc>>,
//! }
//!
//! #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
//! pub enum Relation {}
//!
//! impl ActiveModelBehavior for ActiveModel {}
//!
//! // Implement SoftDeletes for the ActiveModel
//! impl SoftDeletes for ActiveModel {
//!     fn soft_delete(&mut self) {
//!         self.deleted_at = Set(Some(Utc::now()));
//!     }
//!
//!     fn restore(&mut self) {
//!         self.deleted_at = Set(None);
//!     }
//!
//!     fn is_trashed(&self) -> bool {
//!         matches!(&self.deleted_at, ActiveValue::Set(Some(_)) | ActiveValue::Unchanged(Some(_)))
//!     }
//!
//!     fn deleted_at(&self) -> Option<DateTime<Utc>> {
//!         match &self.deleted_at {
//!             ActiveValue::Set(dt) | ActiveValue::Unchanged(dt) => *dt,
//!             _ => None,
//!         }
//!     }
//! }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr,
    EntityTrait, ModelTrait, QueryFilter, QueryTrait, Set,
};

/// Trait for models that support soft deletion
///
/// Models implementing this trait have a `deleted_at` timestamp column that
/// marks when a record was soft-deleted, or NULL if not deleted.
#[async_trait]
pub trait SoftDeletes: Sized {
    /// Mark this model as soft-deleted by setting the deleted_at timestamp
    fn soft_delete(&mut self);

    /// Restore a soft-deleted model by clearing the deleted_at timestamp
    fn restore(&mut self);

    /// Check if this model is currently soft-deleted (trashed)
    fn is_trashed(&self) -> bool;

    /// Get the deleted_at timestamp, if any
    fn deleted_at(&self) -> Option<DateTime<Utc>>;
}

/// Helper trait for entities that support soft deletes at the query level
///
/// This trait provides methods to query soft-deleted records.
#[async_trait]
pub trait SoftDeleteEntity: EntityTrait {
    /// The column that stores the deleted_at timestamp
    fn deleted_at_column() -> <Self as EntityTrait>::Column;

    /// Get only non-deleted records (default behavior)
    fn without_trashed() -> sea_orm::Select<Self> {
        Self::find().filter(Self::deleted_at_column().is_null())
    }

    /// Get all records including soft-deleted ones
    fn with_trashed() -> sea_orm::Select<Self> {
        Self::find()
    }

    /// Get only soft-deleted records
    fn only_trashed() -> sea_orm::Select<Self> {
        Self::find().filter(Self::deleted_at_column().is_not_null())
    }
}

/// Helper function to create a deleted_at timestamp value
///
/// Returns a Set value with the current UTC timestamp.
///
/// # Example
///
/// ```rust
/// use rf_eloquent::soft_deletes::set_deleted_at;
/// use sea_orm::Set;
///
/// let deleted_at = set_deleted_at();
/// assert!(matches!(deleted_at, Set(Some(_))));
/// ```
pub fn set_deleted_at() -> ActiveValue<Option<DateTime<Utc>>> {
    Set(Some(Utc::now()))
}

/// Helper function to clear the deleted_at timestamp
///
/// Returns a Set value with None, indicating the record is not deleted.
///
/// # Example
///
/// ```rust
/// use rf_eloquent::soft_deletes::clear_deleted_at;
/// use sea_orm::Set;
///
/// let deleted_at = clear_deleted_at();
/// assert!(matches!(deleted_at, Set(None)));
/// ```
pub fn clear_deleted_at() -> ActiveValue<Option<DateTime<Utc>>> {
    Set(None)
}

/// Trait for forcing permanent deletion on soft-deletable models
///
/// This allows you to permanently delete a record even if it supports soft deletes.
#[async_trait]
pub trait ForceDelete {
    /// Permanently delete this record from the database
    ///
    /// Unlike soft_delete(), this actually removes the record.
    async fn force_delete(self, db: &DatabaseConnection) -> Result<(), DbErr>;
}

#[async_trait]
impl<T> ForceDelete for T
where
    T: ActiveModelTrait + ActiveModelBehavior + Send,
{
    async fn force_delete(self, db: &DatabaseConnection) -> Result<(), DbErr> {
        self.delete(db).await.map(|_| ())
    }
}

/// Query scope builder for soft delete operations
///
/// Provides a fluent interface for building queries with soft delete awareness.
///
/// # Example
///
/// ```rust,ignore
/// let users = SoftDeleteScope::new()
///     .with_trashed()
///     .where_column("role", "admin")
///     .all(&db)
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct SoftDeleteScope<E: EntityTrait> {
    pub(crate) query: sea_orm::Select<E>,
    pub include_trashed: bool,
    pub only_trashed: bool,
}

impl<E: EntityTrait> SoftDeleteScope<E> {
    /// Create a new soft delete scope
    pub fn new() -> Self {
        Self {
            query: E::find(),
            include_trashed: false,
            only_trashed: false,
        }
    }

    /// Include soft-deleted records in the query
    pub fn with_trashed(mut self) -> Self {
        self.include_trashed = true;
        self.only_trashed = false;
        self
    }

    /// Query only soft-deleted records
    pub fn only_trashed(mut self) -> Self {
        self.only_trashed = true;
        self.include_trashed = false;
        self
    }

    /// Get the underlying query builder
    ///
    /// Applies the soft delete filter based on the scope settings.
    pub fn query<C>(self, deleted_at_column: C) -> sea_orm::Select<E>
    where
        C: ColumnTrait,
    {
        if self.only_trashed {
            self.query.filter(deleted_at_column.is_not_null())
        } else if self.include_trashed {
            self.query
        } else {
            self.query.filter(deleted_at_column.is_null())
        }
    }
}

impl<E: EntityTrait> Default for SoftDeleteScope<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to implement SoftDeletes for an ActiveModel
///
/// This generates the standard soft delete implementation.
///
/// # Example
///
/// ```rust,ignore
/// impl_soft_deletes!(user::ActiveModel, deleted_at);
/// ```
#[macro_export]
macro_rules! impl_soft_deletes {
    ($model:ty, $field:ident) => {
        impl $crate::soft_deletes::SoftDeletes for $model {
            fn soft_delete(&mut self) {
                self.$field = $crate::soft_deletes::set_deleted_at();
            }

            fn restore(&mut self) {
                self.$field = $crate::soft_deletes::clear_deleted_at();
            }

            fn is_trashed(&self) -> bool {
                matches!(
                    &self.$field,
                    sea_orm::ActiveValue::Set(Some(_))
                        | sea_orm::ActiveValue::Unchanged(Some(_))
                )
            }

            fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
                match &self.$field {
                    sea_orm::ActiveValue::Set(dt) | sea_orm::ActiveValue::Unchanged(dt) => *dt,
                    _ => None,
                }
            }
        }
    };
}

/// Macro to implement SoftDeleteEntity for an Entity
///
/// This enables query-level soft delete methods.
///
/// # Example
///
/// ```rust,ignore
/// impl_soft_delete_entity!(user::Entity, user::Column::DeletedAt);
/// ```
#[macro_export]
macro_rules! impl_soft_delete_entity {
    ($entity:ty, $column:expr) => {
        #[async_trait::async_trait]
        impl $crate::soft_deletes::SoftDeleteEntity for $entity {
            fn deleted_at_column() -> <Self as sea_orm::EntityTrait>::Column {
                $column
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{entity::prelude::*, ActiveValue, Set};

    // Test entity for soft deletes
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "test_users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub name: String,
        pub email: String,
        pub deleted_at: Option<DateTime<Utc>>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    impl SoftDeletes for ActiveModel {
        fn soft_delete(&mut self) {
            self.deleted_at = Set(Some(Utc::now()));
        }

        fn restore(&mut self) {
            self.deleted_at = Set(None);
        }

        fn is_trashed(&self) -> bool {
            matches!(
                &self.deleted_at,
                ActiveValue::Set(Some(_)) | ActiveValue::Unchanged(Some(_))
            )
        }

        fn deleted_at(&self) -> Option<DateTime<Utc>> {
            match &self.deleted_at {
                ActiveValue::Set(dt) | ActiveValue::Unchanged(dt) => *dt,
                _ => None,
            }
        }
    }

    #[test]
    fn test_soft_delete() {
        let mut user = ActiveModel {
            id: Set(1),
            name: Set("John Doe".to_string()),
            email: Set("john@example.com".to_string()),
            deleted_at: Set(None),
        };

        assert!(!user.is_trashed());
        assert!(user.deleted_at().is_none());

        user.soft_delete();

        assert!(user.is_trashed());
        assert!(user.deleted_at().is_some());
    }

    #[test]
    fn test_restore() {
        let mut user = ActiveModel {
            id: Set(1),
            name: Set("John Doe".to_string()),
            email: Set("john@example.com".to_string()),
            deleted_at: Set(Some(Utc::now())),
        };

        assert!(user.is_trashed());

        user.restore();

        assert!(!user.is_trashed());
        assert!(user.deleted_at().is_none());
    }

    #[test]
    fn test_is_trashed_with_unchanged() {
        let user = ActiveModel {
            id: Set(1),
            name: Set("John".to_string()),
            email: Set("john@example.com".to_string()),
            deleted_at: ActiveValue::Unchanged(Some(Utc::now())),
        };

        assert!(user.is_trashed());
    }

    #[test]
    fn test_set_deleted_at_helper() {
        let deleted_at = set_deleted_at();
        match deleted_at {
            ActiveValue::Set(Some(_)) => {}
            _ => panic!("Expected Set(Some(_))"),
        }
    }

    #[test]
    fn test_clear_deleted_at_helper() {
        let deleted_at = clear_deleted_at();
        match deleted_at {
            ActiveValue::Set(None) => {}
            _ => panic!("Expected Set(None)"),
        }
    }

    #[test]
    fn test_soft_delete_scope_default() {
        let scope = SoftDeleteScope::<Entity>::new();
        assert!(!scope.include_trashed);
        assert!(!scope.only_trashed);
    }

    #[test]
    fn test_soft_delete_scope_with_trashed() {
        let scope = SoftDeleteScope::<Entity>::new().with_trashed();
        assert!(scope.include_trashed);
        assert!(!scope.only_trashed);
    }

    #[test]
    fn test_soft_delete_scope_only_trashed() {
        let scope = SoftDeleteScope::<Entity>::new().only_trashed();
        assert!(!scope.include_trashed);
        assert!(scope.only_trashed);
    }

    #[test]
    fn test_multiple_soft_delete_restore_cycles() {
        let mut user = ActiveModel {
            id: Set(1),
            name: Set("Test".to_string()),
            email: Set("test@example.com".to_string()),
            deleted_at: Set(None),
        };

        // Cycle 1
        user.soft_delete();
        assert!(user.is_trashed());
        user.restore();
        assert!(!user.is_trashed());

        // Cycle 2
        user.soft_delete();
        assert!(user.is_trashed());
        user.restore();
        assert!(!user.is_trashed());

        // Cycle 3
        user.soft_delete();
        assert!(user.is_trashed());
    }
}
