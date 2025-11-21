//! # Advanced Migration Features
//!
//! Enhanced migration capabilities including:
//! - Foreign key constraints with cascade actions
//! - Index management (single and composite)
//! - Unique constraints
//! - Check constraints
//! - Constraint dropping operations
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::advanced_migrations::*;
//! use sea_orm::DatabaseConnection;
//!
//! # async fn example(db: &DatabaseConnection) -> Result<(), AdvancedMigrationError> {
//! let builder = AdvancedMigrationBuilder::new(db);
//!
//! // Add foreign key with cascade
//! builder.add_foreign_key(
//!     "posts",
//!     vec!["user_id"],
//!     "users",
//!     vec!["id"],
//!     Some(ForeignKeyAction::Cascade),
//!     None
//! ).await?;
//!
//! // Create composite index
//! builder.create_index(
//!     "posts",
//!     vec!["user_id", "created_at"],
//!     false
//! ).await?;
//!
//! // Add unique constraint
//! builder.add_unique_constraint(
//!     "users",
//!     vec!["email"]
//! ).await?;
//! # Ok(())
//! # }
//! ```

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use thiserror::Error;

/// Foreign key action on delete/update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignKeyAction {
    /// Delete/update cascades to child rows
    Cascade,
    /// Set foreign key to NULL
    SetNull,
    /// Reject the operation
    Restrict,
    /// Same as Restrict but deferred
    NoAction,
    /// Set to default value
    SetDefault,
}

impl ForeignKeyAction {
    /// Convert to SQL string
    pub fn to_sql(&self) -> &'static str {
        match self {
            ForeignKeyAction::Cascade => "CASCADE",
            ForeignKeyAction::SetNull => "SET NULL",
            ForeignKeyAction::Restrict => "RESTRICT",
            ForeignKeyAction::NoAction => "NO ACTION",
            ForeignKeyAction::SetDefault => "SET DEFAULT",
        }
    }
}

/// Advanced migration error types
#[derive(Debug, Error)]
pub enum AdvancedMigrationError {
    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),

    /// Invalid constraint
    #[error("Invalid constraint: {0}")]
    InvalidConstraint(String),

    /// Constraint already exists
    #[error("Constraint '{0}' already exists")]
    ConstraintExists(String),

    /// Constraint not found
    #[error("Constraint '{0}' not found")]
    ConstraintNotFound(String),

    /// Unsupported operation
    #[error("Unsupported operation for database backend: {0}")]
    UnsupportedOperation(String),
}

/// Result type for advanced migration operations
pub type AdvancedMigrationResult<T> = Result<T, AdvancedMigrationError>;

/// Advanced migration builder for complex schema operations
pub struct AdvancedMigrationBuilder<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> AdvancedMigrationBuilder<'a> {
    /// Create a new advanced migration builder
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection reference
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Get the database backend
    fn backend(&self) -> DbBackend {
        self.db.get_database_backend()
    }

    /// Add a foreign key constraint
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `columns` - Column names to create foreign key on
    /// * `foreign_table` - Referenced table name
    /// * `foreign_columns` - Referenced column names
    /// * `on_delete` - Optional ON DELETE action
    /// * `on_update` - Optional ON UPDATE action
    ///
    /// # Note
    ///
    /// SQLite does not support adding foreign keys to existing tables.
    /// This method will return an error for SQLite databases. Foreign keys
    /// must be defined when creating the table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::advanced_migrations::*;
    /// # async fn example(builder: &AdvancedMigrationBuilder<'_>) -> AdvancedMigrationResult<()> {
    /// builder.add_foreign_key(
    ///     "posts",
    ///     vec!["user_id"],
    ///     "users",
    ///     vec!["id"],
    ///     Some(ForeignKeyAction::Cascade),
    ///     Some(ForeignKeyAction::Cascade)
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_foreign_key(
        &self,
        table: &str,
        columns: Vec<&str>,
        foreign_table: &str,
        foreign_columns: Vec<&str>,
        on_delete: Option<ForeignKeyAction>,
        on_update: Option<ForeignKeyAction>,
    ) -> AdvancedMigrationResult<()> {
        if columns.is_empty() || foreign_columns.is_empty() {
            return Err(AdvancedMigrationError::InvalidConstraint(
                "Foreign key must have at least one column".to_string(),
            ));
        }

        if columns.len() != foreign_columns.len() {
            return Err(AdvancedMigrationError::InvalidConstraint(
                "Column count mismatch between foreign key and reference".to_string(),
            ));
        }

        // SQLite doesn't support adding foreign keys to existing tables
        if self.backend() == DbBackend::Sqlite {
            return Err(AdvancedMigrationError::UnsupportedOperation(
                "SQLite does not support adding foreign keys to existing tables. Define foreign keys in CREATE TABLE statement.".to_string(),
            ));
        }

        let constraint_name = format!("fk_{}_{}", table, columns.join("_"));
        let cols = columns.join(", ");
        let foreign_cols = foreign_columns.join(", ");

        let mut sql = format!(
            "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({})",
            table, constraint_name, cols, foreign_table, foreign_cols
        );

        if let Some(action) = on_delete {
            sql.push_str(&format!(" ON DELETE {}", action.to_sql()));
        }

        if let Some(action) = on_update {
            sql.push_str(&format!(" ON UPDATE {}", action.to_sql()));
        }

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Drop a foreign key constraint
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `constraint_name` - Name of the foreign key constraint to drop
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::advanced_migrations::*;
    /// # async fn example(builder: &AdvancedMigrationBuilder<'_>) -> AdvancedMigrationResult<()> {
    /// builder.drop_foreign_key("posts", "fk_posts_user_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn drop_foreign_key(
        &self,
        table: &str,
        constraint_name: &str,
    ) -> AdvancedMigrationResult<()> {
        let sql = match self.backend() {
            DbBackend::MySql => {
                format!("ALTER TABLE {} DROP FOREIGN KEY {}", table, constraint_name)
            }
            DbBackend::Postgres | DbBackend::Sqlite => {
                format!("ALTER TABLE {} DROP CONSTRAINT {}", table, constraint_name)
            }
        };

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Create an index on one or more columns
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `columns` - Column names to index
    /// * `unique` - Whether this is a unique index
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::advanced_migrations::*;
    /// # async fn example(builder: &AdvancedMigrationBuilder<'_>) -> AdvancedMigrationResult<()> {
    /// // Composite index
    /// builder.create_index("posts", vec!["user_id", "created_at"], false).await?;
    ///
    /// // Unique index
    /// builder.create_index("users", vec!["email"], true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_index(
        &self,
        table: &str,
        columns: Vec<&str>,
        unique: bool,
    ) -> AdvancedMigrationResult<()> {
        if columns.is_empty() {
            return Err(AdvancedMigrationError::InvalidConstraint(
                "Index must have at least one column".to_string(),
            ));
        }

        let index_name = format!("idx_{}_{}", table, columns.join("_"));
        let cols = columns.join(", ");
        let unique_str = if unique { "UNIQUE " } else { "" };

        let sql = format!(
            "CREATE {}INDEX {} ON {}({})",
            unique_str, index_name, table, cols
        );

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Create a named index
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `index_name` - Custom index name
    /// * `columns` - Column names to index
    /// * `unique` - Whether this is a unique index
    pub async fn create_named_index(
        &self,
        table: &str,
        index_name: &str,
        columns: Vec<&str>,
        unique: bool,
    ) -> AdvancedMigrationResult<()> {
        if columns.is_empty() {
            return Err(AdvancedMigrationError::InvalidConstraint(
                "Index must have at least one column".to_string(),
            ));
        }

        let cols = columns.join(", ");
        let unique_str = if unique { "UNIQUE " } else { "" };

        let sql = format!(
            "CREATE {}INDEX {} ON {}({})",
            unique_str, index_name, table, cols
        );

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Drop an index
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `index_name` - Name of the index to drop
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::advanced_migrations::*;
    /// # async fn example(builder: &AdvancedMigrationBuilder<'_>) -> AdvancedMigrationResult<()> {
    /// builder.drop_index("posts", "idx_posts_user_id_created_at").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn drop_index(&self, table: &str, index_name: &str) -> AdvancedMigrationResult<()> {
        let sql = match self.backend() {
            DbBackend::MySql => {
                format!("DROP INDEX {} ON {}", index_name, table)
            }
            DbBackend::Postgres | DbBackend::Sqlite => {
                format!("DROP INDEX {}", index_name)
            }
        };

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Add a unique constraint
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `columns` - Column names for the unique constraint
    ///
    /// # Note
    ///
    /// For SQLite, this creates a unique index instead of a constraint.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::advanced_migrations::*;
    /// # async fn example(builder: &AdvancedMigrationBuilder<'_>) -> AdvancedMigrationResult<()> {
    /// // Single column unique
    /// builder.add_unique_constraint("users", vec!["email"]).await?;
    ///
    /// // Composite unique
    /// builder.add_unique_constraint("user_roles", vec!["user_id", "role_id"]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_unique_constraint(
        &self,
        table: &str,
        columns: Vec<&str>,
    ) -> AdvancedMigrationResult<()> {
        if columns.is_empty() {
            return Err(AdvancedMigrationError::InvalidConstraint(
                "Unique constraint must have at least one column".to_string(),
            ));
        }

        // SQLite doesn't support ALTER TABLE ADD CONSTRAINT for unique constraints
        // Use CREATE UNIQUE INDEX instead
        if self.backend() == DbBackend::Sqlite {
            return self.create_index(table, columns, true).await;
        }

        let constraint_name = format!("uniq_{}_{}", table, columns.join("_"));
        let cols = columns.join(", ");

        let sql = format!(
            "ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({})",
            table, constraint_name, cols
        );

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Drop a unique constraint
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `constraint_name` - Name of the unique constraint to drop
    pub async fn drop_unique_constraint(
        &self,
        table: &str,
        constraint_name: &str,
    ) -> AdvancedMigrationResult<()> {
        let sql = match self.backend() {
            DbBackend::MySql => {
                format!("ALTER TABLE {} DROP INDEX {}", table, constraint_name)
            }
            DbBackend::Postgres | DbBackend::Sqlite => {
                format!("ALTER TABLE {} DROP CONSTRAINT {}", table, constraint_name)
            }
        };

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Add a check constraint (PostgreSQL and newer MySQL/SQLite)
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `constraint_name` - Name for the check constraint
    /// * `check_expression` - SQL expression for the check (e.g., "age >= 18")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::advanced_migrations::*;
    /// # async fn example(builder: &AdvancedMigrationBuilder<'_>) -> AdvancedMigrationResult<()> {
    /// builder.add_check_constraint(
    ///     "users",
    ///     "chk_age_positive",
    ///     "age >= 0"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_check_constraint(
        &self,
        table: &str,
        constraint_name: &str,
        check_expression: &str,
    ) -> AdvancedMigrationResult<()> {
        let sql = format!(
            "ALTER TABLE {} ADD CONSTRAINT {} CHECK ({})",
            table, constraint_name, check_expression
        );

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Drop a check constraint
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `constraint_name` - Name of the check constraint to drop
    pub async fn drop_check_constraint(
        &self,
        table: &str,
        constraint_name: &str,
    ) -> AdvancedMigrationResult<()> {
        let sql = format!("ALTER TABLE {} DROP CONSTRAINT {}", table, constraint_name);

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Create a composite primary key (for migration purposes)
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `columns` - Column names for the primary key
    ///
    /// Note: This is typically used when creating a table, not in ALTER TABLE
    pub async fn add_primary_key(
        &self,
        table: &str,
        columns: Vec<&str>,
    ) -> AdvancedMigrationResult<()> {
        if columns.is_empty() {
            return Err(AdvancedMigrationError::InvalidConstraint(
                "Primary key must have at least one column".to_string(),
            ));
        }

        let constraint_name = format!("pk_{}", table);
        let cols = columns.join(", ");

        let sql = format!(
            "ALTER TABLE {} ADD CONSTRAINT {} PRIMARY KEY ({})",
            table, constraint_name, cols
        );

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Drop primary key constraint
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    pub async fn drop_primary_key(&self, table: &str) -> AdvancedMigrationResult<()> {
        let sql = match self.backend() {
            DbBackend::MySql => {
                format!("ALTER TABLE {} DROP PRIMARY KEY", table)
            }
            DbBackend::Postgres => {
                format!("ALTER TABLE {} DROP CONSTRAINT {}_pkey", table, table)
            }
            DbBackend::Sqlite => {
                return Err(AdvancedMigrationError::UnsupportedOperation(
                    "SQLite does not support dropping primary keys".to_string(),
                ));
            }
        };

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Rename a table
    ///
    /// # Arguments
    ///
    /// * `old_name` - Current table name
    /// * `new_name` - New table name
    pub async fn rename_table(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> AdvancedMigrationResult<()> {
        let sql = match self.backend() {
            DbBackend::Postgres => {
                format!("ALTER TABLE {} RENAME TO {}", old_name, new_name)
            }
            DbBackend::MySql => {
                format!("RENAME TABLE {} TO {}", old_name, new_name)
            }
            DbBackend::Sqlite => {
                format!("ALTER TABLE {} RENAME TO {}", old_name, new_name)
            }
        };

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Rename a column
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `old_name` - Current column name
    /// * `new_name` - New column name
    /// * `column_type` - Column type (required for some databases)
    pub async fn rename_column(
        &self,
        table: &str,
        old_name: &str,
        new_name: &str,
        column_type: Option<&str>,
    ) -> AdvancedMigrationResult<()> {
        let sql = match self.backend() {
            DbBackend::Postgres => {
                format!(
                    "ALTER TABLE {} RENAME COLUMN {} TO {}",
                    table, old_name, new_name
                )
            }
            DbBackend::MySql => {
                let col_type = column_type.ok_or_else(|| {
                    AdvancedMigrationError::InvalidConstraint(
                        "Column type required for MySQL column rename".to_string(),
                    )
                })?;
                format!(
                    "ALTER TABLE {} CHANGE {} {} {}",
                    table, old_name, new_name, col_type
                )
            }
            DbBackend::Sqlite => {
                format!(
                    "ALTER TABLE {} RENAME COLUMN {} TO {}",
                    table, old_name, new_name
                )
            }
        };

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }

    /// Drop a column
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `column` - Column name to drop
    pub async fn drop_column(&self, table: &str, column: &str) -> AdvancedMigrationResult<()> {
        let sql = format!("ALTER TABLE {} DROP COLUMN {}", table, column);

        self.db
            .execute(Statement::from_string(self.backend(), sql))
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foreign_key_action_to_sql() {
        assert_eq!(ForeignKeyAction::Cascade.to_sql(), "CASCADE");
        assert_eq!(ForeignKeyAction::SetNull.to_sql(), "SET NULL");
        assert_eq!(ForeignKeyAction::Restrict.to_sql(), "RESTRICT");
        assert_eq!(ForeignKeyAction::NoAction.to_sql(), "NO ACTION");
        assert_eq!(ForeignKeyAction::SetDefault.to_sql(), "SET DEFAULT");
    }
}
