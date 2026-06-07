//! # Query Scopes
//!
//! Laravel Eloquent-style query scopes for reusable query logic.
//!
//! ## Overview
//!
//! Query scopes allow you to define reusable query constraints that can be
//! chained together for elegant query building, similar to Laravel Eloquent.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::prelude::*;
//! use rf_orm::scopes::*;
//! use std::collections::HashMap;
//! # mod post {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "posts")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool, pub created_at: String }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # use post::Entity as Post;
//!
//! // Define scopes for your entity
//! impl HasScopes for post::Entity {
//!     fn scopes() -> HashMap<&'static str, ScopeFn<Self>> {
//!         let mut map = HashMap::new();
//!
//!         map.insert("published", Box::new(|query: sea_orm::Select<Self>| {
//!             query.filter(post::Column::Published.eq(true))
//!         }) as ScopeFn<Self>);
//!
//!         map.insert("recent", Box::new(|query: sea_orm::Select<Self>| {
//!             query.order_by_desc(post::Column::CreatedAt).limit(10)
//!         }) as ScopeFn<Self>);
//!
//!         map
//!     }
//! }
//!
//! # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
//! // Use scopes in queries
//! let posts = Post::query(db)
//!     .apply_scope("published")
//!     .apply_scope("recent")
//!     .get()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use sea_orm::{EntityTrait, Select};
use std::collections::HashMap;

/// Function type for scope definitions
///
/// A scope function takes a Select query and returns a modified Select query
pub type ScopeFn<E> = Box<dyn Fn(Select<E>) -> Select<E> + Send + Sync>;

/// Trait for entities that support query scopes
///
/// Implement this trait to define named scopes for your entity.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::scopes::*;
/// use sea_orm::{ColumnTrait, QueryFilter};
/// use std::collections::HashMap;
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub active: bool, pub premium: bool }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # use user::Entity as User;
///
/// impl HasScopes for User {
///     fn scopes() -> HashMap<&'static str, ScopeFn<Self>> {
///         let mut map = HashMap::new();
///
///         map.insert("active", Box::new(|query: sea_orm::Select<Self>| {
///             query.filter(user::Column::Active.eq(true))
///         }) as ScopeFn<Self>);
///
///         map.insert("premium", Box::new(|query: sea_orm::Select<Self>| {
///             query.filter(user::Column::Premium.eq(true))
///         }) as ScopeFn<Self>);
///
///         map
///     }
/// }
/// ```
pub trait HasScopes: EntityTrait {
    /// Return a map of scope names to scope functions
    fn scopes() -> HashMap<&'static str, ScopeFn<Self>>;

    /// Get a specific scope by name
    fn scope(name: &str) -> Option<ScopeFn<Self>> {
        Self::scopes().remove(name)
    }
}

/// Macro for easily defining scopes
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::define_scopes;
/// use sea_orm::{ColumnTrait, QueryFilter, QueryOrder, QuerySelect};
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool, pub created_at: String, pub views: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
///
/// define_scopes!(post::Entity, {
///     "published" => |query| query.filter(post::Column::Published.eq(true)),
///     "recent" => |query| query.order_by_desc(post::Column::CreatedAt).limit(10),
///     "popular" => |query| query.filter(post::Column::Views.gt(1000)),
/// });
/// ```
#[macro_export]
macro_rules! define_scopes {
    ($entity:ty, { $($name:literal => |$query:ident| $body:expr),* $(,)? }) => {
        impl $crate::scopes::HasScopes for $entity {
            fn scopes() -> std::collections::HashMap<&'static str, $crate::scopes::ScopeFn<Self>> {
                let mut map = std::collections::HashMap::new();
                $(
                    map.insert(
                        $name,
                        Box::new(|$query: sea_orm::Select<Self>| $body)
                            as $crate::scopes::ScopeFn<Self>
                    );
                )*
                map
            }
        }
    };
}

/// Extension methods for QueryBuilder to support scopes
pub trait ScopeExt<E: EntityTrait> {
    /// Apply a named scope to the query
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let users = User::query(db)
    ///     .apply_scope("active")
    ///     .apply_scope("premium")
    ///     .get()
    ///     .await?;
    /// ```
    fn apply_scope(self, name: &str) -> Self;

    /// Apply multiple named scopes to the query
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let users = User::query(db)
    ///     .apply_scopes(&["active", "verified"])
    ///     .get()
    ///     .await?;
    /// ```
    fn apply_scopes(self, names: &[&str]) -> Self;
}

impl<E> ScopeExt<E> for crate::QueryBuilder<E>
where
    E: EntityTrait + HasScopes,
{
    fn apply_scope(self, name: &str) -> Self {
        if let Some(scope_fn) = E::scopes().get(name) {
            let (select, db) = self.into_select();
            let new_select = scope_fn(select);
            Self::from_select(new_select, db)
        } else {
            self
        }
    }

    fn apply_scopes(self, names: &[&str]) -> Self {
        let mut query = self;
        for name in names {
            query = query.apply_scope(name);
        }
        query
    }
}

/// Registry for dynamically registered scopes
///
/// Allows registering scopes at runtime instead of compile-time.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::scopes::ScopeRegistry;
///
/// let mut registry = ScopeRegistry::<post::Entity>::new();
///
/// registry.register("published", |query| {
///     query.filter(post::Column::Published.eq(true))
/// });
///
/// registry.register("recent", |query| {
///     query.order_by_desc(post::Column::CreatedAt)
/// });
///
/// // Apply registered scope
/// let query = registry.apply(Post::query(db), "published");
/// ```
pub struct ScopeRegistry<E: EntityTrait> {
    scopes: HashMap<String, ScopeFn<E>>,
}

impl<E: EntityTrait> ScopeRegistry<E> {
    /// Create a new scope registry
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
        }
    }

    /// Register a new scope
    pub fn register<F>(&mut self, name: impl Into<String>, scope: F)
    where
        F: Fn(Select<E>) -> Select<E> + Send + Sync + 'static,
    {
        self.scopes.insert(name.into(), Box::new(scope));
    }

    /// Apply a registered scope to a query builder
    pub fn apply(&self, query: crate::QueryBuilder<E>, name: &str) -> crate::QueryBuilder<E> {
        if let Some(scope_fn) = self.scopes.get(name) {
            let (select, db) = query.into_select();
            let new_select = scope_fn(select);
            crate::QueryBuilder::from_select(new_select, db)
        } else {
            query
        }
    }

    /// Apply multiple registered scopes
    pub fn apply_many(
        &self,
        mut query: crate::QueryBuilder<E>,
        names: &[&str],
    ) -> crate::QueryBuilder<E> {
        for name in names {
            query = self.apply(query, name);
        }
        query
    }

    /// Check if a scope is registered
    pub fn has(&self, name: &str) -> bool {
        self.scopes.contains_key(name)
    }

    /// Remove a registered scope
    pub fn unregister(&mut self, name: &str) -> bool {
        self.scopes.remove(name).is_some()
    }

    /// Get all registered scope names
    pub fn names(&self) -> Vec<String> {
        self.scopes.keys().cloned().collect()
    }

    /// Clear all registered scopes
    pub fn clear(&mut self) {
        self.scopes.clear();
    }
}

impl<E: EntityTrait> Default for ScopeRegistry<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_registry_creation() {
        // Just verify the API compiles
        // Real tests would need a database connection
    }

    #[test]
    fn test_scope_registry_registration() {
        // Verify registration API
    }

    #[test]
    fn test_has_scopes_trait() {
        // Verify trait definition compiles
    }
}
