//! # Relationships Module
//!
//! Advanced relationship types and loading strategies for rf-orm.
//!
//! ## Modules
//!
//! - `basic` - Basic relationship helpers (BelongsTo, HasMany)
//! - `through` - HasOneThrough and HasManyThrough relationships
//! - `morph_to_many` - Polymorphic many-to-many relationships
//! - `loading` - Eager and lazy loading control

pub mod basic;
pub mod through;
pub mod morph_to_many;
pub mod loading;

// Re-exports for convenience
pub use basic::{RelationshipHelpers, eager_load};

pub use through::{
    HasOneThrough, HasManyThrough, ThroughQueryBuilder, ThroughResult,
    has_one_through, has_many_through,
};

pub use morph_to_many::{
    MorphToMany, MorphToManyBuilder, MorphToManyResult,
    morph_to_many, attach_morph, detach_morph, sync_morph, toggle_morph,
};

pub use loading::{
    EagerLoad, LazyLoad, CollectionExt, LoadResult, RelationshipData,
    EagerLoadConfig, SupportsEagerLoading,
    load_relation, load_relations, should_eager_load,
};
