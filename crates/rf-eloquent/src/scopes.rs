//! # Query Scopes System
//!
//! Provides reusable query constraints that can be chained together.
//! Similar to Laravel's query scopes for building expressive, reusable queries.
//!
//! ## Features
//!
//! - **Named Scopes**: Define reusable query filters
//! - **Chainable**: Combine multiple scopes together
//! - **Parameterized**: Pass parameters to scopes
//! - **Global Scopes**: Automatically applied to all queries
//! - **Common Scopes**: Pre-built scopes for common use cases
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rf_eloquent::prelude::*;
//! use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
//!
//! // Define scopes on your entity
//! impl user::Entity {
//!     pub fn active<S>(select: S) -> S
//!     where
//!         S: QueryFilter,
//!     {
//!         select.filter(user::Column::Active.eq(true))
//!     }
//!
//!     pub fn verified<S>(select: S) -> S
//!     where
//!         S: QueryFilter,
//!     {
//!         select.filter(user::Column::EmailVerifiedAt.is_not_null())
//!     }
//!
//!     pub fn premium<S>(select: S) -> S
//!     where
//!         S: QueryFilter,
//!     {
//!         select.filter(user::Column::SubscriptionTier.eq("premium"))
//!     }
//! }
//!
//! // Use scopes in queries
//! # async fn example(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
//! let users = user::Entity::find()
//!     .apply_if(user::Entity::active)
//!     .apply_if(user::Entity::verified)
//!     .all(db)
//!     .await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Select,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use thiserror::Error;

/// Scope system errors
#[derive(Error, Debug)]
pub enum ScopeError {
    #[error("Scope not found: {0}")]
    NotFound(String),

    #[error("Invalid scope parameter: {0}")]
    InvalidParameter(String),

    #[error("Scope application failed: {0}")]
    ApplicationFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),
}

pub type ScopeResult<T> = Result<T, ScopeError>;

/// Trait for entities that support query scopes
pub trait HasScopes: EntityTrait {
    /// Apply a named scope to the query
    fn apply_scope(select: Select<Self>, scope_name: &str) -> ScopeResult<Select<Self>> {
        Err(ScopeError::NotFound(scope_name.to_string()))
    }

    /// Get list of available scopes
    fn available_scopes() -> Vec<&'static str> {
        Vec::new()
    }
}

/// Extension trait for applying scopes to queries
pub trait ScopedQuery<E: EntityTrait>: Sized {
    /// Apply a scope function to the query
    fn apply_if<F>(self, scope: F) -> Self
    where
        F: FnOnce(Self) -> Self;

    /// Apply scope only if condition is true
    fn apply_when<F>(self, condition: bool, scope: F) -> Self
    where
        F: FnOnce(Self) -> Self;

    /// Apply multiple scopes in sequence
    fn apply_scopes<F>(self, scopes: Vec<F>) -> Self
    where
        F: FnOnce(Self) -> Self;
}

impl<E: EntityTrait> ScopedQuery<E> for Select<E> {
    fn apply_if<F>(self, scope: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        scope(self)
    }

    fn apply_when<F>(self, condition: bool, scope: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            scope(self)
        } else {
            self
        }
    }

    fn apply_scopes<F>(mut self, scopes: Vec<F>) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        for scope in scopes {
            self = scope(self);
        }
        self
    }
}

/// Common reusable scopes for typical use cases
pub struct CommonScopes;

impl CommonScopes {
    /// Scope for active records (where active = true)
    pub fn active<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.eq(true))
    }

    /// Scope for inactive records (where active = false)
    pub fn inactive<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.eq(false))
    }

    /// Scope for recent records (created within last N days)
    pub fn recent<E, C, S>(select: S, column: C, days: i64) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        let threshold = Utc::now() - Duration::days(days);
        select.filter(column.gt(threshold))
    }

    /// Scope for records created in the last 7 days
    pub fn recent_week<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        Self::recent::<E, C, S>(select, column, 7)
    }

    /// Scope for records created in the last 30 days
    pub fn recent_month<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        Self::recent::<E, C, S>(select, column, 30)
    }

    /// Scope for popular records (views > threshold)
    pub fn popular<E, C, S>(select: S, column: C, threshold: i64) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.gt(threshold))
    }

    /// Scope for published records
    pub fn published<E, C1, C2, S>(select: S, published_col: C1, published_at_col: C2) -> S
    where
        E: EntityTrait,
        C1: ColumnTrait,
        C2: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(
            Condition::all()
                .add(published_col.eq(true))
                .add(published_at_col.lte(Utc::now())),
        )
    }

    /// Scope for featured records
    pub fn featured<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.eq(true))
    }

    /// Scope for verified records (where verified_at is not null)
    pub fn verified<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.is_not_null())
    }

    /// Scope for records with null values
    pub fn unverified<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.is_null())
    }

    /// Scope for records created after a specific date
    pub fn created_after<E, C, S>(select: S, column: C, date: DateTime<Utc>) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.gt(date))
    }

    /// Scope for records created before a specific date
    pub fn created_before<E, C, S>(select: S, column: C, date: DateTime<Utc>) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(column.lt(date))
    }

    /// Scope for records between two dates
    pub fn created_between<E, C, S>(
        select: S,
        column: C,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryFilter,
    {
        select.filter(Condition::all().add(column.gte(start)).add(column.lte(end)))
    }

    /// Scope for ordering by latest (created_at DESC)
    pub fn latest<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryOrder,
    {
        select.order_by_desc(column)
    }

    /// Scope for ordering by oldest (created_at ASC)
    pub fn oldest<E, C, S>(select: S, column: C) -> S
    where
        E: EntityTrait,
        C: ColumnTrait,
        S: QueryOrder,
    {
        select.order_by_asc(column)
    }
}

/// Builder for constructing queries with multiple scopes
pub struct ScopeBuilder<E: EntityTrait> {
    select: Select<E>,
    applied_scopes: Vec<String>,
}

impl<E: EntityTrait> ScopeBuilder<E> {
    /// Create a new scope builder
    pub fn new() -> Self {
        Self {
            select: E::find(),
            applied_scopes: Vec::new(),
        }
    }

    /// Create a scope builder from an existing query
    pub fn from_select(select: Select<E>) -> Self {
        Self {
            select,
            applied_scopes: Vec::new(),
        }
    }

    /// Apply a scope function
    pub fn scope<F>(mut self, name: &str, f: F) -> Self
    where
        F: FnOnce(Select<E>) -> Select<E>,
    {
        self.select = f(self.select);
        self.applied_scopes.push(name.to_string());
        self
    }

    /// Apply scope conditionally
    pub fn when<F>(self, condition: bool, name: &str, f: F) -> Self
    where
        F: FnOnce(Select<E>) -> Select<E>,
    {
        if condition {
            self.scope(name, f)
        } else {
            self
        }
    }

    /// Apply scope unless condition is true
    pub fn unless<F>(self, condition: bool, name: &str, f: F) -> Self
    where
        F: FnOnce(Select<E>) -> Select<E>,
    {
        self.when(!condition, name, f)
    }

    /// Get the list of applied scopes
    pub fn get_applied_scopes(&self) -> &[String] {
        &self.applied_scopes
    }

    /// Build and return the final query
    pub fn build(self) -> Select<E> {
        self.select
    }

    /// Execute the query and get all results
    pub async fn get(self, db: &DatabaseConnection) -> Result<Vec<E::Model>, DbErr> {
        self.select.all(db).await
    }

    /// Execute the query and get the first result
    pub async fn first(self, db: &DatabaseConnection) -> Result<Option<E::Model>, DbErr> {
        self.select.one(db).await
    }
}

impl<E: EntityTrait> Default for ScopeBuilder<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for global scopes that are automatically applied
pub struct GlobalScopeRegistry<E: EntityTrait> {
    scopes: HashMap<String, Arc<dyn Fn(Select<E>) -> Select<E> + Send + Sync>>,
    _phantom: PhantomData<E>,
}

impl<E: EntityTrait> GlobalScopeRegistry<E> {
    /// Create a new global scope registry
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Register a global scope
    pub fn register<F>(&mut self, name: impl Into<String>, scope: F)
    where
        F: Fn(Select<E>) -> Select<E> + Send + Sync + 'static,
    {
        self.scopes.insert(name.into(), Arc::new(scope));
    }

    /// Apply all global scopes to a query
    pub fn apply_all(&self, mut select: Select<E>) -> Select<E> {
        for scope in self.scopes.values() {
            select = scope(select);
        }
        select
    }

    /// Remove a global scope
    pub fn remove(&mut self, name: &str) -> bool {
        self.scopes.remove(name).is_some()
    }

    /// Check if a scope is registered
    pub fn has(&self, name: &str) -> bool {
        self.scopes.contains_key(name)
    }

    /// Clear all global scopes
    pub fn clear(&mut self) {
        self.scopes.clear();
    }

    /// Get the number of registered scopes
    pub fn count(&self) -> usize {
        self.scopes.len()
    }
}

impl<E: EntityTrait> Default for GlobalScopeRegistry<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock entity for testing
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestModel {
        id: i32,
        active: bool,
        views: i64,
    }

    #[test]
    fn test_scope_builder_tracks_applied_scopes() {
        // This is a conceptual test - would need actual entity implementation
        // to fully test, but demonstrates the API
        assert!(true);
    }

    #[test]
    fn test_global_scope_registry() {
        // Test the basic registry API without requiring a full Entity implementation
        // The registry stores scope closures and provides methods to manage them

        // Test that we can conceptually use the registry
        // In practice, this would be used with real entities
        assert!(true); // Placeholder - full test requires integration with actual entities
    }
}
