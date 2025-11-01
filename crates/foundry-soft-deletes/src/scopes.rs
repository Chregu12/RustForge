//! Query scopes for soft deletes

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Select};

/// Soft delete scope for queries
pub enum SoftDeleteScope {
    /// Include soft-deleted records
    WithTrashed,
    /// Only soft-deleted records
    OnlyTrashed,
    /// Exclude soft-deleted records (default)
    WithoutTrashed,
}

/// Extension trait for queries
pub trait QueryScopeExt<E>
where
    E: EntityTrait,
{
    fn with_scope(self, scope: SoftDeleteScope) -> Select<E>;
}

impl<E> QueryScopeExt<E> for Select<E>
where
    E: EntityTrait,
{
    fn with_scope(self, scope: SoftDeleteScope) -> Select<E> {
        match scope {
            SoftDeleteScope::WithTrashed => self,
            SoftDeleteScope::OnlyTrashed => {
                self.filter(E::Column::DeletedAt.is_not_null())
            }
            SoftDeleteScope::WithoutTrashed => self.filter(E::Column::DeletedAt.is_null()),
        }
    }
}
