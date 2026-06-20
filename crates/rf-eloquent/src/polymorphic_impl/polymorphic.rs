//! # Core Polymorphic Relationship Trait
//!
//! Defines the fundamental trait that all polymorphic models must implement.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use thiserror::Error;

/// Errors that can occur in polymorphic relationships
#[derive(Error, Debug)]
pub enum PolymorphicError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("Type not registered: {0}")]
    TypeNotRegistered(String),

    #[error("Invalid morph type: {0}")]
    InvalidMorphType(String),

    #[error("Missing morph columns: {0}")]
    MissingMorphColumns(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Morphable model not found")]
    MorphableNotFound,

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

pub type PolymorphicResult<T> = Result<T, PolymorphicError>;

/// Trait for models that can participate in polymorphic relationships
pub trait Polymorphic: Send + Sync {
    /// Get the morph relation name (e.g., "commentable", "imageable")
    fn morph_name(&self) -> &str;

    /// Get the type identifier for this model (e.g., "Post", "Video")
    fn morph_type(&self) -> String;

    /// Get the primary key value for this model
    fn morph_id(&self) -> i64;
}

/// Trait for polymorphic relationship configuration
#[async_trait]
pub trait PolymorphicRelation: Send + Sync {
    /// The relation name (e.g., "commentable")
    fn relation_name(&self) -> &str;

    /// Get the morph type column name (e.g., "commentable_type")
    fn morph_type_column(&self) -> String {
        format!("{}_type", self.relation_name())
    }

    /// Get the morph id column name (e.g., "commentable_id")
    fn morph_id_column(&self) -> String {
        format!("{}_id", self.relation_name())
    }

    /// Validate that the morph columns exist and have valid values
    async fn validate_morph_columns(
        &self,
        db: &DatabaseConnection,
        table_name: &str,
    ) -> PolymorphicResult<()>;
}

/// Helper to extract morph type and ID from a dynamic model
pub trait MorphableModel {
    /// Get the morph type value from the model
    fn get_morph_type(&self, relation_name: &str) -> Option<String>;

    /// Get the morph ID value from the model
    fn get_morph_id(&self, relation_name: &str) -> Option<i64>;
}

/// Helper to set morph type and ID on a model
pub trait MorphableMutator {
    /// Set the morph type value on the model
    fn set_morph_type(&mut self, relation_name: &str, morph_type: String);

    /// Set the morph ID value on the model
    fn set_morph_id(&mut self, relation_name: &str, morph_id: i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polymorphic_error_display() {
        let err = PolymorphicError::TypeNotRegistered("Post".to_string());
        assert_eq!(err.to_string(), "Type not registered: Post");
    }

    #[test]
    fn test_polymorphic_error_type_mismatch() {
        let err = PolymorphicError::TypeMismatch {
            expected: "Post".to_string(),
            actual: "Video".to_string(),
        };
        assert_eq!(err.to_string(), "Type mismatch: expected Post, got Video");
    }

    #[test]
    fn test_morph_column_names() {
        struct TestRelation;
        #[async_trait]
        impl PolymorphicRelation for TestRelation {
            fn relation_name(&self) -> &str {
                "commentable"
            }

            async fn validate_morph_columns(
                &self,
                _db: &DatabaseConnection,
                _table_name: &str,
            ) -> PolymorphicResult<()> {
                Ok(())
            }
        }

        let rel = TestRelation;
        assert_eq!(rel.morph_type_column(), "commentable_type");
        assert_eq!(rel.morph_id_column(), "commentable_id");
    }
}
