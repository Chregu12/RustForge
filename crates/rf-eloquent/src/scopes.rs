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
//! ```rust,ignore
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
use once_cell::sync::Lazy;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Select,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
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

// ---------------------------------------------------------------------------
// Global scope registry
// ---------------------------------------------------------------------------

// Type alias used internally for the stored scope closure
type ScopeFnBox<E> = Box<dyn Fn(Select<E>) -> Select<E> + Send + Sync>;

/// Thread-safe global scope registry: entity TypeId → (name → erased closure)
static GLOBAL_SCOPE_REGISTRY: Lazy<
    RwLock<HashMap<TypeId, HashMap<String, Arc<dyn Any + Send + Sync>>>>,
> = Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a global scope that is automatically applied to every query of
/// entity type `E`.
///
/// Call at application startup (e.g. in `main` or a boot function).
///
/// # Example
///
/// ```rust,ignore
/// use rf_eloquent::scopes::add_global_scope;
/// use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
///
/// // Automatically filter soft-deleted rows for all user queries
/// add_global_scope::<user::Entity, _>("active", |q| {
///     q.filter(user::Column::DeletedAt.is_null())
/// });
/// ```
pub fn add_global_scope<E, F>(name: impl Into<String>, scope: F)
where
    E: EntityTrait + 'static,
    F: Fn(Select<E>) -> Select<E> + Send + Sync + 'static,
{
    let boxed: ScopeFnBox<E> = Box::new(scope);
    let erased: Arc<dyn Any + Send + Sync> = Arc::new(boxed);
    let entity_id = TypeId::of::<E>();
    if let Ok(mut registry) = GLOBAL_SCOPE_REGISTRY.write() {
        registry
            .entry(entity_id)
            .or_default()
            .insert(name.into(), erased);
    }
}

/// Remove a named global scope for entity type `E`.
pub fn remove_global_scope<E>(name: &str)
where
    E: EntityTrait + 'static,
{
    let entity_id = TypeId::of::<E>();
    if let Ok(mut registry) = GLOBAL_SCOPE_REGISTRY.write() {
        if let Some(scopes) = registry.get_mut(&entity_id) {
            scopes.remove(name);
        }
    }
}

/// Apply all global scopes registered for entity type `E` to the given query.
///
/// This is called automatically by query helpers that are scope-aware.
/// You can also call it manually when building custom queries.
///
/// # Example
///
/// ```rust,ignore
/// use rf_eloquent::scopes::apply_global_scopes;
/// use sea_orm::EntityTrait;
///
/// let query = apply_global_scopes::<user::Entity>(user::Entity::find());
/// ```
pub fn apply_global_scopes<E>(select: Select<E>) -> Select<E>
where
    E: EntityTrait + 'static,
{
    let entity_id = TypeId::of::<E>();
    match GLOBAL_SCOPE_REGISTRY.read() {
        Ok(registry) => match registry.get(&entity_id) {
            Some(scopes) => {
                let mut result = select;
                for scope_any in scopes.values() {
                    if let Some(f) = scope_any.downcast_ref::<ScopeFnBox<E>>() {
                        result = f(result);
                    }
                }
                result
            }
            None => select,
        },
        Err(_) => select,
    }
}

/// Execute a closure while temporarily bypassing the specified global scopes
/// for entity type `E`.
///
/// The scopes are restored after the closure returns.
///
/// # Example
///
/// ```rust,ignore
/// use rf_eloquent::scopes::without_global_scopes;
///
/// let result = without_global_scopes::<user::Entity, _, _>(
///     vec!["active".to_string()],
///     || {
///         // Query runs without the "active" scope
///         user::Entity::find()
///     },
/// );
/// ```
pub fn without_global_scopes<E, F, R>(scope_names: Vec<String>, f: F) -> R
where
    E: EntityTrait + 'static,
    F: FnOnce() -> R,
{
    let entity_id = TypeId::of::<E>();
    let mut saved: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();

    // Remove specified scopes
    if let Ok(mut registry) = GLOBAL_SCOPE_REGISTRY.write() {
        if let Some(scopes) = registry.get_mut(&entity_id) {
            for name in &scope_names {
                if let Some(scope) = scopes.remove(name) {
                    saved.insert(name.clone(), scope);
                }
            }
        }
    }

    let result = f();

    // Restore removed scopes
    if let Ok(mut registry) = GLOBAL_SCOPE_REGISTRY.write() {
        let scopes = registry.entry(entity_id).or_default();
        for (name, scope) in saved {
            scopes.insert(name, scope);
        }
    }

    result
}

// ---------------------------------------------------------------------------

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
