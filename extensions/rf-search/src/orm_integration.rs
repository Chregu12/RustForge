//! # Search-ORM Integration
//!
//! Bridges the [`Searchable`] trait with SeaORM entities so that ORM models can
//! be indexed and searched without boilerplate.
//!
//! ## Overview
//!
//! Implement [`SearchableOrmModel`] on your ORM model to get:
//!
//! - `index_model(driver, model)` — index a single ORM model.
//! - `index_models(driver, models)` — batch-index a collection of ORM models.
//! - `remove_from_index(driver, id)` — delete a document from the index.
//! - `reindex_all(driver, models)` — clear and rebuild the index in one call.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_search::orm_integration::{SearchableOrmModel, OrmSearchHelper};
//! use rf_search::{Searchable, SearchDriver};
//! use serde::{Deserialize, Serialize};
//! use async_trait::async_trait;
//!
//! // A plain ORM model struct
//! #[derive(Clone, Serialize, Deserialize)]
//! pub struct Post {
//!     pub id: i64,
//!     pub title: String,
//!     pub body: String,
//! }
//!
//! // Implement the lightweight bridge trait
//! impl SearchableOrmModel for Post {
//!     type SearchDocument = Post;
//!
//!     fn search_id(&self) -> String {
//!         self.id.to_string()
//!     }
//!
//!     fn index_name() -> &'static str {
//!         "posts"
//!     }
//!
//!     fn searchable_fields() -> Vec<&'static str> {
//!         vec!["title", "body"]
//!     }
//!
//!     fn to_search_document(&self) -> Self::SearchDocument {
//!         self.clone()
//!     }
//! }
//!
//! # async fn example(driver: &impl SearchDriver, posts: Vec<Post>) -> rf_search::Result<()> {
//! // Index all posts
//! OrmSearchHelper::index_models(driver, &posts).await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    driver::{Result, SearchDriver},
    searchable::{SearchOptions, SearchResult, Searchable},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ─── Bridge trait ─────────────────────────────────────────────────────────────

/// Bridge trait that adds search capabilities to ORM model types.
///
/// Unlike [`Searchable`], which is the core search trait, `SearchableOrmModel`
/// is designed to be implemented directly on plain struct model types (such as
/// SeaORM `Model` structs) without requiring the associated-type gymnastics of
/// the lower-level trait.
pub trait SearchableOrmModel: Send + Sync {
    /// The serializable document type written to the search index.
    ///
    /// Usually the same as `Self` for simple cases.
    type SearchDocument: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static;

    /// Unique document ID in the search index.
    fn search_id(&self) -> String;

    /// Name of the search index (e.g., `"posts"`).
    fn index_name() -> &'static str;

    /// Columns included in full-text search.
    fn searchable_fields() -> Vec<&'static str>;

    /// Columns available for filtering (optional override).
    fn filterable_fields() -> Vec<&'static str> {
        Vec::new()
    }

    /// Columns available for sorting (optional override).
    fn sortable_fields() -> Vec<&'static str> {
        Vec::new()
    }

    /// Convert `self` into a search document.
    fn to_search_document(&self) -> Self::SearchDocument;
}

// ─── Adapter that connects SearchableOrmModel to the Searchable trait ─────────

/// Internal adapter implementing [`Searchable`] for a reference to `M`.
///
/// This allows existing [`SearchDriver`] implementations (which speak `Searchable`)
/// to work transparently with `SearchableOrmModel`.
struct OrmModelAdapter<'a, M: SearchableOrmModel>(&'a M);

impl<'a, M: SearchableOrmModel> Searchable for OrmModelAdapter<'a, M> {
    type Model = M::SearchDocument;

    fn searchable_fields() -> Vec<&'static str> {
        M::searchable_fields()
    }

    fn to_searchable(&self) -> Self::Model {
        self.0.to_search_document()
    }

    fn search_id(&self) -> String {
        self.0.search_id()
    }

    fn index_name() -> &'static str {
        M::index_name()
    }

    fn filterable_fields() -> Vec<&'static str> {
        M::filterable_fields()
    }

    fn sortable_fields() -> Vec<&'static str> {
        M::sortable_fields()
    }
}

// ─── Helper functions ─────────────────────────────────────────────────────────

/// Static helper providing ergonomic index / search / delete operations for
/// [`SearchableOrmModel`] types.
pub struct OrmSearchHelper;

impl OrmSearchHelper {
    /// Index a single ORM model.
    pub async fn index_model<D, M>(driver: &D, model: &M) -> Result<()>
    where
        D: SearchDriver,
        M: SearchableOrmModel,
    {
        driver.index(&OrmModelAdapter(model)).await
    }

    /// Batch-index a slice of ORM models.
    pub async fn index_models<D, M>(driver: &D, models: &[M]) -> Result<()>
    where
        D: SearchDriver,
        M: SearchableOrmModel,
    {
        let adapters: Vec<OrmModelAdapter<'_, M>> =
            models.iter().map(OrmModelAdapter).collect();

        // Index each model individually (the Searchable trait uses a single
        // concrete type for batch operations, so we iterate here).
        for adapter in &adapters {
            driver.index(adapter).await?;
        }
        Ok(())
    }

    /// Remove a single document from the index by its ID.
    pub async fn remove_from_index<D, M>(driver: &D, id: &str) -> Result<()>
    where
        D: SearchDriver,
        M: SearchableOrmModel,
    {
        driver.delete::<OrmModelAdapter<M>>(id).await
    }

    /// Search the index associated with `M` and return typed results.
    pub async fn search<D, M>(
        driver: &D,
        query: &str,
        options: Option<SearchOptions>,
    ) -> Result<SearchResult<M::SearchDocument>>
    where
        D: SearchDriver,
        M: SearchableOrmModel,
    {
        driver.search::<OrmModelAdapter<M>>(query, options).await
    }

    /// Clear then re-index a complete collection of models.
    ///
    /// Useful for initial imports or rebuilding after schema changes.
    pub async fn reindex_all<D, M>(driver: &D, models: &[M]) -> Result<()>
    where
        D: SearchDriver,
        M: SearchableOrmModel,
    {
        // Clear the index using the adapter type.
        driver.clear_index::<OrmModelAdapter<M>>().await?;
        // Then index every model.
        for model in models {
            driver.index(&OrmModelAdapter(model)).await?;
        }
        Ok(())
    }
}

// ─── Convenience extension trait ──────────────────────────────────────────────

/// Extension trait that adds search methods directly on types that implement
/// [`SearchableOrmModel`].
///
/// ```rust,no_run
/// # use rf_search::orm_integration::{SearchableOrmModel, ModelSearchExt};
/// # use rf_search::SearchDriver;
/// # #[derive(Clone, serde::Serialize, serde::Deserialize)]
/// # struct Post { id: i64, title: String }
/// # impl SearchableOrmModel for Post {
/// #     type SearchDocument = Post;
/// #     fn search_id(&self) -> String { self.id.to_string() }
/// #     fn index_name() -> &'static str { "posts" }
/// #     fn searchable_fields() -> Vec<&'static str> { vec!["title"] }
/// #     fn to_search_document(&self) -> Self::SearchDocument { self.clone() }
/// # }
/// # async fn example(driver: &impl SearchDriver, post: Post) -> rf_search::Result<()> {
/// post.index_self(driver).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait ModelSearchExt: SearchableOrmModel + Sized {
    /// Index this model.
    async fn index_self<D: SearchDriver + Sync>(&self, driver: &D) -> Result<()>
    where
        Self: Sync,
    {
        OrmSearchHelper::index_model(driver, self).await
    }

    /// Remove this model from the index.
    async fn remove_self<D: SearchDriver + Sync>(&self, driver: &D) -> Result<()>
    where
        Self: Sync,
    {
        let id = self.search_id();
        OrmSearchHelper::remove_from_index::<D, Self>(driver, &id).await
    }
}

/// Blanket implementation: every `SearchableOrmModel + Sized` automatically gets
/// `ModelSearchExt`.
impl<T: SearchableOrmModel + Sized> ModelSearchExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct MockPost {
        id: i64,
        title: String,
    }

    impl SearchableOrmModel for MockPost {
        type SearchDocument = MockPost;

        fn search_id(&self) -> String {
            self.id.to_string()
        }

        fn index_name() -> &'static str {
            "posts"
        }

        fn searchable_fields() -> Vec<&'static str> {
            vec!["title"]
        }

        fn to_search_document(&self) -> Self::SearchDocument {
            self.clone()
        }
    }

    #[test]
    fn test_adapter_search_id() {
        let post = MockPost { id: 42, title: "Hello".to_string() };
        let adapter = OrmModelAdapter(&post);
        assert_eq!(adapter.search_id(), "42");
    }

    #[test]
    fn test_adapter_index_name() {
        assert_eq!(<OrmModelAdapter<MockPost> as Searchable>::index_name(), "posts");
    }

    #[test]
    fn test_adapter_to_searchable() {
        let post = MockPost { id: 1, title: "Rust".to_string() };
        let adapter = OrmModelAdapter(&post);
        let doc = adapter.to_searchable();
        assert_eq!(doc.title, "Rust");
    }
}
