//! Migration helpers for soft deletes

use sea_orm::sea_query::{ColumnDef, Table};

/// Add deleted_at column to table
pub fn add_soft_delete_column(table: &mut Table, column_name: &str) -> &mut Table {
    table.add_column(
        ColumnDef::new(column_name)
            .timestamp_with_time_zone()
            .null()
            .default(None::<String>),
    )
}

/// Create index on deleted_at column
pub fn add_soft_delete_index(
    table_name: &str,
    column_name: &str,
) -> sea_orm::sea_query::Index {
    sea_orm::sea_query::Index::create()
        .name(&format!("idx_{}_{}", table_name, column_name))
        .table(table_name.parse().unwrap())
        .col(column_name.parse().unwrap())
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
