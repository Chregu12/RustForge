//! Soft delete trait for entities

use chrono::{DateTime, Utc};
use sea_orm::{ActiveValue, Set};

/// Trait for entities that support soft deletion
///
/// Soft delete sets a `deleted_at` timestamp instead of actually
/// removing the record from the database.
///
/// # Example
///
/// ```rust
/// use rf_orm::SoftDelete;
/// use sea_orm::{entity::prelude::*, Set};
/// use chrono::{DateTime, Utc};
///
/// #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #[sea_orm(table_name = "users")]
/// pub struct Model {
///     #[sea_orm(primary_key)]
///     pub id: i32,
///     pub name: String,
///     pub deleted_at: Option<DateTime<Utc>>,
/// }
///
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// pub enum Relation {}
///
/// impl ActiveModelBehavior for ActiveModel {}
///
/// // Implement SoftDelete
/// impl SoftDelete for ActiveModel {
///     fn soft_delete(&mut self) {
///         self.deleted_at = Set(Some(Utc::now()));
///     }
///
///     fn restore(&mut self) {
///         self.deleted_at = Set(None);
///     }
///
///     fn is_deleted(&self) -> bool {
///         matches!(&self.deleted_at, ActiveValue::Set(Some(_)) | ActiveValue::Unchanged(Some(_)))
///     }
/// }
/// ```
pub trait SoftDelete {
    /// Mark entity as soft-deleted by setting `deleted_at` timestamp
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut user = user.into_active_model();
    /// user.soft_delete();
    /// user.update(db).await?;
    /// ```
    fn soft_delete(&mut self);

    /// Restore soft-deleted entity by clearing `deleted_at`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut user = deleted_user.into_active_model();
    /// user.restore();
    /// user.update(db).await?;
    /// ```
    fn restore(&mut self);

    /// Check if entity is soft-deleted
    ///
    /// Returns `true` if `deleted_at` is set, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let user = user.into_active_model();
    /// if user.is_deleted() {
    ///     println!("User is deleted");
    /// }
    /// ```
    fn is_deleted(&self) -> bool;
}

/// Helper to set deleted_at timestamp
///
/// # Example
///
/// ```rust
/// use rf_orm::soft_delete::set_deleted_at;
/// use sea_orm::Set;
///
/// let deleted_at = set_deleted_at();
/// assert!(matches!(deleted_at, Set(Some(_))));
/// ```
pub fn set_deleted_at() -> ActiveValue<Option<DateTime<Utc>>> {
    Set(Some(Utc::now()))
}

/// Helper to clear deleted_at timestamp
///
/// # Example
///
/// ```rust
/// use rf_orm::soft_delete::clear_deleted_at;
/// use sea_orm::Set;
///
/// let deleted_at = clear_deleted_at();
/// assert!(matches!(deleted_at, Set(None)));
/// ```
pub fn clear_deleted_at() -> ActiveValue<Option<DateTime<Utc>>> {
    Set(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use sea_orm::{entity::prelude::*, ActiveValue};

    // Test entity
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "test_users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub deleted_at: Option<DateTime<Utc>>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    impl SoftDelete for ActiveModel {
        fn soft_delete(&mut self) {
            self.deleted_at = Set(Some(Utc::now()));
        }

        fn restore(&mut self) {
            self.deleted_at = Set(None);
        }

        fn is_deleted(&self) -> bool {
            matches!(
                &self.deleted_at,
                ActiveValue::Set(Some(_)) | ActiveValue::Unchanged(Some(_))
            )
        }
    }

    #[test]
    fn test_soft_delete() {
        let mut user = ActiveModel {
            id: ActiveValue::Set(1),
            name: ActiveValue::Set("Test User".to_string()),
            deleted_at: ActiveValue::Set(None),
        };

        assert!(!user.is_deleted());

        user.soft_delete();
        assert!(user.is_deleted());

        match &user.deleted_at {
            ActiveValue::Set(Some(_)) => {}
            _ => panic!("Expected deleted_at to be set"),
        }
    }

    #[test]
    fn test_restore() {
        let mut user = ActiveModel {
            id: ActiveValue::Set(1),
            name: ActiveValue::Set("Test User".to_string()),
            deleted_at: ActiveValue::Set(Some(Utc::now())),
        };

        assert!(user.is_deleted());

        user.restore();
        assert!(!user.is_deleted());

        match &user.deleted_at {
            ActiveValue::Set(None) => {}
            _ => panic!("Expected deleted_at to be None"),
        }
    }

    #[test]
    fn test_set_deleted_at_helper() {
        let deleted_at = set_deleted_at();
        match deleted_at {
            ActiveValue::Set(Some(_)) => {}
            _ => panic!("Expected Some with timestamp"),
        }
    }

    #[test]
    fn test_clear_deleted_at_helper() {
        let deleted_at = clear_deleted_at();
        match deleted_at {
            ActiveValue::Set(None) => {}
            _ => panic!("Expected None"),
        }
    }

    #[test]
    fn test_is_deleted_with_unchanged() {
        let user = ActiveModel {
            id: ActiveValue::Set(1),
            name: ActiveValue::Set("Test".to_string()),
            deleted_at: ActiveValue::Unchanged(Some(Utc::now())),
        };

        assert!(user.is_deleted());
    }
}
