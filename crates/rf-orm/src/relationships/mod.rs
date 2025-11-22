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
pub mod loading;
pub mod morph_to_many;
pub mod through;

// Re-exports for convenience
pub use basic::{eager_load, RelationshipHelpers};

pub use through::{
    has_many_through, has_one_through, HasManyThrough, HasOneThrough, ThroughQueryBuilder,
    ThroughResult,
};

pub use morph_to_many::{
    attach_morph, detach_morph, morph_to_many, sync_morph, toggle_morph, MorphToMany,
    MorphToManyBuilder, MorphToManyResult,
};

pub use loading::{
    load_relation, load_relations, should_eager_load, CollectionExt, EagerLoad, EagerLoadConfig,
    LazyLoad, LoadResult, RelationshipData, SupportsEagerLoading,
};
