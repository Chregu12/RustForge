//! Migration helpers for soft deletes
///
/// Note: Due to SeaORM 0.12 API changes, users must manually create migrations.
/// Use the macro below as a template for your migrations.

use sea_orm::sea_query::{ColumnDef, Iden};

/// Helper to create a deleted_at column definition
pub fn deleted_at_column<T: Iden + 'static>(column_iden: T) -> ColumnDef {
    ColumnDef::new(column_iden)
        .timestamp_with_time_zone()
        .null()
        .to_owned()
}

#[macro_export]
macro_rules! impl_soft_delete_migration {
    ($table:expr) => {
        use sea_orm::sea_query::*;

        Table::alter()
            .table($table)
            .add_column(
                ColumnDef::new("deleted_at")
                    .timestamp_with_time_zone()
                    .null(),
            )
            .to_owned()
    };
}
