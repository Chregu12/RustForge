//! # rf-cache-facade
//!
//! Laravel-style `Cache` facade for the RustForge framework.
//!
//! This crate used to carry its **own** duplicate `Cache`/`CacheManager`
//! implementation backed by a *separate* process-global `GLOBAL_CACHE`. That
//! meant `use rf::Cache` (which resolves to `rf_cache::CacheFacade`) and the
//! `cache!` helper macro (which expands to `rf_cache_facade::Cache::…`) wrote to
//! **two different** in-memory caches — a split-brain bug.
//!
//! It now simply **re-exports the single real implementation from
//! [`rf_cache`]**, so there is exactly one source of truth and one global cache
//! shared by the prelude facade and the `cache!` macro.
//!
//! # Recommended Usage
//!
//! Prefer the consolidated `rf` crate (`use rf::Cache;`). When depending on this
//! crate directly:
//!
//! ```rust
//! use rf_cache_facade::Cache;
//! ```

// One source of truth: the real Cache facade + global manager live in `rf-cache`.
pub use rf_cache::cache_manager::{CacheManager, GLOBAL_CACHE};
pub use rf_cache::facade::{Cache, IntoTtl};

// Re-export commonly used types from rf-cache (kept for API stability).
pub use rf_cache::{CacheError, CacheResult, MemoryCache, TaggedCache};
