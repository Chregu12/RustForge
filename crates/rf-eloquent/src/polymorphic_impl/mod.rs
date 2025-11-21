//! # Polymorphic Relationships Module
//!
//! This module provides Laravel-style polymorphic relationships for RustForge.
//! Polymorphic relationships allow a model to belong to multiple other model types
//! on a single association.
//!
//! ## Supported Polymorphic Relationships
//!
//! - `MorphTo`: Belongs to multiple model types (inverse of MorphOne/MorphMany)
//! - `MorphOne`: One-to-one polymorphic relationship
//! - `MorphMany`: One-to-many polymorphic relationship
//! - `MorphToMany`: Many-to-many polymorphic relationship (with pivot)
//! - `MorphedByMany`: Inverse of MorphToMany

pub mod morph_many;
pub mod morph_one;
pub mod morph_to;
pub mod morph_to_many;
pub mod morphed_by_many;
pub mod polymorphic;
pub mod type_registry;

pub use morph_many::MorphMany;
pub use morph_one::MorphOne;
pub use morph_to::MorphTo;
pub use morph_to_many::MorphToMany;
pub use morphed_by_many::MorphedByMany;
pub use polymorphic::{Polymorphic, PolymorphicRelation};
pub use type_registry::{TypeRegistry, TypeResolver, GLOBAL_TYPE_REGISTRY};
