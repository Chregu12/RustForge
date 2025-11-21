//! Database error types

use thiserror::Error;
use sea_orm::DbErr;

/// Database operation errors
#[derive(Debug, Error)]
pub enum DbError {
    /// Database connection error
    #[error("Failed to connect to database: {source}")]
    ConnectionFailed {
        #[source]
        source: DbErr,
    },

    /// Query execution error
    #[error("Query failed: {query}: {source}")]
    QueryFailed {
        query: String,
        #[source]
        source: DbErr,
    },

    /// Entity not found
    #[error("Entity '{entity}' with ID '{id}' not found")]
    NotFound { entity: String, id: String },

    /// Unique constraint violation
    #[error("Unique constraint violation on field '{field}' with value '{value}'")]
    UniqueViolation { field: String, value: String },

    /// Foreign key constraint violation
    #[error("Foreign key constraint violation on table '{table}' key '{key}'")]
    ForeignKeyViolation { table: String, key: String },

    /// Transaction error
    #[error("Transaction failed: {source}")]
    TransactionFailed {
        #[source]
        source: DbErr,
    },

    /// Migration error
    #[cfg(feature = "migrate")]
    #[error("Migration '{migration}' failed: {source}")]
    MigrationFailed {
        migration: String,
        #[source]
        source: DbErr,
    },

    /// Connection pool error
    #[error("Connection pool error: {0}")]
    PoolError(String),

    /// Invalid configuration
    #[error("Invalid database configuration: {0}")]
    InvalidConfig(String),

    /// Internal database error
    #[error("Internal database error: {0}")]
    Internal(#[from] DbErr),
}

/// Result type for database operations
pub type DbResult<T> = Result<T, DbError>;

impl From<DbError> for rf_core::AppError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound { entity, id } => rf_core::AppError::NotFound {
                resource: format!("{} (id: {})", entity, id),
            },
            DbError::UniqueViolation { field, value } => rf_core::AppError::Conflict {
                message: format!("Duplicate {} : {}", field, value),
            },
            DbError::ForeignKeyViolation { table, key } => rf_core::AppError::BadRequest {
                message: format!("Invalid reference to {}:{}", table, key),
            },
            DbError::ConnectionFailed { .. }
            | DbError::PoolError(_)
            | DbError::TransactionFailed { .. } => rf_core::AppError::ServiceUnavailable {
                service: "database".to_string(),
            },
            DbError::InvalidConfig(msg) => rf_core::AppError::BadRequest { message: msg },
            _ => rf_core::AppError::Internal(anyhow::Error::from(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DbError::NotFound {
            entity: "User".to_string(),
            id: "123".to_string(),
        };
        assert!(err.to_string().contains("User"));
        assert!(err.to_string().contains("123"));
    }

    #[test]
    fn test_unique_violation() {
        let err = DbError::UniqueViolation {
            field: "email".to_string(),
            value: "test@example.com".to_string(),
        };
        assert!(err.to_string().contains("email"));
        assert!(err.to_string().contains("test@example.com"));
    }

    #[test]
    fn test_conversion_to_app_error() {
        let db_err = DbError::NotFound {
            entity: "User".to_string(),
            id: "1".to_string(),
        };

        let app_err: rf_core::AppError = db_err.into();

        match app_err {
            rf_core::AppError::NotFound { resource } => {
                assert!(resource.contains("User"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
