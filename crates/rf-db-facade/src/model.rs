//! Laravel-style Model trait for Eloquent-like database operations.
//!
//! This module provides the `Model` trait that enables Laravel-style syntax:
//!
//! ```rust,no_run
//! use rf_db_facade::{Model, model};
//! use serde_json::json;
//!
//! // Define a model using the macro
//! model!(User, "users");
//!
//! async fn example() {
//!     // Now use Laravel-style syntax!
//!     let active_users = User::r#where("active", true).get().await.unwrap();
//!     let user = User::find(1).await.unwrap();
//!     let new_user = User::create(json!({
//!         "name": "John",
//!         "email": "john@example.com"
//!     })).await.unwrap();
//!
//!     // Chain queries like Laravel!
//!     let admins = User::r#where("role", "admin")
//!         .r#where("active", true)
//!         .order_by("name", "asc")
//!         .limit(10)
//!         .get().await.unwrap();
//! }
//! ```

// The `Model` trait is a deliberate Laravel-style public API using `async fn` in
// traits. We don't require `Send` auto-trait bounds on the returned futures, so
// suppress the `async_fn_in_trait` lint rather than desugar to `impl Future`.
#![allow(async_fn_in_trait)]

use crate::query_builder::QueryBuilder;
use serde_json::Value;

/// The Model trait for Laravel-style Eloquent operations.
///
/// Implement this trait on a struct to enable Laravel-style database operations.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_db_facade::Model;
///
/// struct User;
///
/// impl Model for User {
///     const TABLE: &'static str = "users";
/// }
///
/// async fn example() {
///     // Now you can use Laravel-style syntax!
///     let users = User::r#where("active", true).get().await.unwrap();
///     let user = User::find(1).await.unwrap();
/// }
/// ```
///
/// # ⚠️ In-memory stub
///
/// These query methods are backed by an **in-memory store** (see
/// [`QueryBuilder`]), not a real database connection — they are for prototyping
/// and tests, and do not interpret SQL or `WHERE` clauses. For real persistence,
/// configure a database connection and use the typed, SeaORM-backed model API.
pub trait Model: Sized {
    /// The database table name for this model
    const TABLE: &'static str;

    /// Start a query with a where clause - Laravel style!
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Just like Laravel!
    /// User::r#where("active", true).get().await?;
    /// User::r#where("email", "john@example.com").first().await?;
    /// ```
    fn r#where<V: Into<Value>>(column: impl Into<String>, value: V) -> QueryBuilder {
        QueryBuilder::new(Self::TABLE).r#where(column, value)
    }

    /// Alias for `r#where` - cleaner syntax without r# prefix
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Clean and readable!
    /// User::filter("active", true).get().await?;
    /// User::filter("role", "admin").filter("active", true).get().await?;
    /// ```
    fn filter<V: Into<Value>>(column: impl Into<String>, value: V) -> QueryBuilder {
        QueryBuilder::new(Self::TABLE).filter(column, value)
    }

    /// Start a query (returns all records if no conditions added)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let query = User::query()
    ///     .r#where("active", true)
    ///     .order_by("name", "asc");
    /// ```
    fn query() -> QueryBuilder {
        QueryBuilder::new(Self::TABLE)
    }

    /// Get all records
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let all_users = User::all().await?;
    /// ```
    async fn all() -> Result<Vec<Value>, String> {
        QueryBuilder::new(Self::TABLE).get().await
    }

    /// Find a record by ID
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let user = User::find(1).await?;
    /// let user = User::find("uuid-string").await?;
    /// ```
    async fn find<V: Into<Value>>(id: V) -> Result<Option<Value>, String> {
        QueryBuilder::new(Self::TABLE)
            .r#where("id", id)
            .first()
            .await
    }

    /// Find a record by ID or fail
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let user = User::find_or_fail(1).await?; // Returns error if not found
    /// ```
    async fn find_or_fail<V: Into<Value>>(id: V) -> Result<Value, String> {
        Self::find(id)
            .await?
            .ok_or_else(|| format!("{} not found", Self::TABLE))
    }

    /// Create a new record
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Just like Laravel!
    /// let user = User::create(serde_json::json!({
    ///     "name": "John",
    ///     "email": "john@example.com"
    /// })).await?;
    ///
    /// println!("Created user with ID: {}", user["id"]);
    /// ```
    async fn create(data: Value) -> Result<Value, String> {
        QueryBuilder::new(Self::TABLE).create(data).await
    }

    /// Update a record by ID
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// User::update_by_id(1, serde_json::json!({
    ///     "name": "John Doe"
    /// })).await?;
    /// ```
    async fn update_by_id<V: Into<Value>>(id: V, data: Value) -> Result<u64, String> {
        QueryBuilder::new(Self::TABLE)
            .r#where("id", id)
            .update(data)
            .await
    }

    /// Delete a record by ID
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// User::destroy(1).await?;
    /// ```
    async fn destroy<V: Into<Value>>(id: V) -> Result<u64, String> {
        QueryBuilder::new(Self::TABLE)
            .r#where("id", id)
            .delete()
            .await
    }

    /// Count all records
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let total = User::count().await?;
    /// ```
    async fn count() -> Result<usize, String> {
        QueryBuilder::new(Self::TABLE).count().await
    }

    /// First or create - find first matching record or create it
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let user = User::first_or_create(
    ///     serde_json::json!({"email": "john@example.com"}),  // search
    ///     serde_json::json!({"name": "John", "email": "john@example.com"})  // create
    /// ).await?;
    /// ```
    async fn first_or_create(
        search: Value,
        create_data: Value,
    ) -> Result<Value, String> {
        // Build search query from search params
        let mut builder = QueryBuilder::new(Self::TABLE);

        if let Some(obj) = search.as_object() {
            for (key, value) in obj {
                builder = builder.r#where(key.clone(), value.clone());
            }
        }

        if let Some(found) = builder.first().await? {
            return Ok(found);
        }

        // Not found, create it
        QueryBuilder::new(Self::TABLE).create(create_data).await
    }

    /// Update or create - update matching record or create new
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let user = User::update_or_create(
    ///     serde_json::json!({"email": "john@example.com"}),  // search
    ///     serde_json::json!({"name": "John Updated"})  // update/create data
    /// ).await?;
    /// ```
    async fn update_or_create(
        search: Value,
        update_data: Value,
    ) -> Result<Value, String> {
        // Build search query from search params
        let mut builder = QueryBuilder::new(Self::TABLE);

        if let Some(obj) = search.as_object() {
            for (key, value) in obj {
                builder = builder.r#where(key.clone(), value.clone());
            }
        }

        if let Some(found) = builder.first().await? {
            // Found, update it
            if let Some(id) = found.get("id") {
                QueryBuilder::new(Self::TABLE)
                    .r#where("id", id.clone())
                    .update(update_data)
                    .await?;
                // Return updated record
                return QueryBuilder::new(Self::TABLE)
                    .r#where("id", id.clone())
                    .first()
                    .await?
                    .ok_or_else(|| "Update failed".to_string());
            }
        }

        // Not found, create with merged data
        let mut merged = search.clone();
        if let (Some(m), Some(u)) = (merged.as_object_mut(), update_data.as_object()) {
            for (key, value) in u {
                m.insert(key.clone(), value.clone());
            }
        }
        QueryBuilder::new(Self::TABLE).create(merged).await
    }

    // =========================================================================
    // Laravel-style camelCase aliases
    // =========================================================================

    /// Laravel-style alias for `first_or_create`
    #[allow(non_snake_case)]
    async fn firstOrCreate(search: Value, create_data: Value) -> Result<Value, String> {
        Self::first_or_create(search, create_data).await
    }

    /// Laravel-style alias for `update_or_create`
    #[allow(non_snake_case)]
    async fn updateOrCreate(search: Value, update_data: Value) -> Result<Value, String> {
        Self::update_or_create(search, update_data).await
    }

    /// Laravel-style alias for `find_or_fail`
    #[allow(non_snake_case)]
    async fn findOrFail<V: Into<Value>>(id: V) -> Result<Value, String> {
        Self::find_or_fail(id).await
    }

    /// Laravel-style alias for `update_by_id`
    #[allow(non_snake_case)]
    async fn updateById<V: Into<Value>>(id: V, data: Value) -> Result<u64, String> {
        Self::update_by_id(id, data).await
    }
}

/// Macro for quickly defining a model
///
/// # Examples
///
/// ```rust,no_run
/// use rf_db_facade::{model, Model};
///
/// // Simple model
/// model!(User, "users");
///
/// async fn example() {
///     // Now use it:
///     let users = User::r#where("active", true).get().await.unwrap();
/// }
/// ```
#[macro_export]
macro_rules! model {
    ($name:ident, $table:expr) => {
        pub struct $name;

        impl $crate::Model for $name {
            const TABLE: &'static str = $table;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Define test models
    model!(User, "users");
    model!(Post, "posts");

    #[test]
    fn test_model_table_name() {
        assert_eq!(User::TABLE, "users");
        assert_eq!(Post::TABLE, "posts");
    }

    #[tokio::test]
    async fn test_model_where() {
        let query = User::r#where("active", true);
        let results = query.get().await;
        assert!(results.is_ok());
    }

    #[tokio::test]
    async fn test_model_query() {
        let query = User::query()
            .r#where("active", true)
            .order_by("name", "asc")
            .limit(10);

        let results = query.get().await;
        assert!(results.is_ok());
    }

    #[tokio::test]
    async fn test_model_all() {
        let results = User::all().await;
        assert!(results.is_ok());
    }

    #[tokio::test]
    async fn test_model_find() {
        let result = User::find(1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_model_create() {
        let result = User::create(serde_json::json!({
            "name": "Test User",
            "email": "test@example.com"
        })).await;

        assert!(result.is_ok());
        let user = result.unwrap();
        assert!(user.get("id").is_some());
    }

    #[tokio::test]
    async fn test_model_destroy() {
        let result = User::destroy(1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_model_chained_query() {
        let results = User::r#where("role", "admin")
            .r#where("active", true)
            .where_not_null("email")
            .order_by_desc("created_at")
            .limit(5)
            .get()
            .await;

        assert!(results.is_ok());
    }

    #[tokio::test]
    async fn test_model_count() {
        let count = User::count().await;
        assert!(count.is_ok());
    }
}
