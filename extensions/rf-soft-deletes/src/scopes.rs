//! Query scopes for soft deletes

use sea_orm::{EntityTrait, Select};

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
///
/// Note: Implementors must implement this trait based on their entity's specific Column enum.
/// The default implementation cannot know the name of the deleted_at column at compile time.
pub trait QueryScopeExt<E>
where
    E: EntityTrait,
{
    fn with_scope(self, scope: SoftDeleteScope) -> Select<E>;
}

// Users must implement this trait for their specific entities
// Example:
// ```
// impl QueryScopeExt<MyEntity> for Select<MyEntity> {
//     fn with_scope(self, scope: SoftDeleteScope) -> Select<MyEntity> {
//         match scope {
//             SoftDeleteScope::WithTrashed => self,
//             SoftDeleteScope::OnlyTrashed => self.filter(Column::DeletedAt.is_not_null()),
//             SoftDeleteScope::WithoutTrashed => self.filter(Column::DeletedAt.is_null()),
//         }
//     }
// }
// ```
