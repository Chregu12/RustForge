use async_graphql::{Error as GraphQLError, ErrorExtensions};
use sea_orm::DbErr;
use std::fmt;

#[derive(Debug)]
pub enum GraphQLErrorCode {
    NotFound,
    ValidationError,
    DatabaseError,
    InternalError,
}

impl fmt::Display for GraphQLErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLErrorCode::NotFound => write!(f, "NOT_FOUND"),
            GraphQLErrorCode::ValidationError => write!(f, "VALIDATION_ERROR"),
            GraphQLErrorCode::DatabaseError => write!(f, "DATABASE_ERROR"),
            GraphQLErrorCode::InternalError => write!(f, "INTERNAL_ERROR"),
        }
    }
}

pub fn not_found(message: impl Into<String>) -> GraphQLError {
    GraphQLError::new(message).extend_with(|_, e| {
        e.set("code", GraphQLErrorCode::NotFound.to_string());
    })
}

pub fn validation_error(message: impl Into<String>) -> GraphQLError {
    GraphQLError::new(message).extend_with(|_, e| {
        e.set("code", GraphQLErrorCode::ValidationError.to_string());
    })
}

pub fn database_error(err: DbErr) -> GraphQLError {
    GraphQLError::new(format!("Database error: {}", err)).extend_with(|_, e| {
        e.set("code", GraphQLErrorCode::DatabaseError.to_string());
    })
}

pub fn internal_error(message: impl Into<String>) -> GraphQLError {
    GraphQLError::new(message).extend_with(|_, e| {
        e.set("code", GraphQLErrorCode::InternalError.to_string());
    })
}
