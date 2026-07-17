//! Blade Components System
//!
//! Full Laravel Blade-compatible component system with support for:
//! - Class-based components
//! - Anonymous components
//! - Named slots
//! - Component attributes
//! - Type-safe props

pub mod attributes;
pub mod class_component;
pub mod compiler;
pub mod parser;
pub mod props;
pub mod registry;
pub mod slots;

pub use attributes::AttributeBag;
pub use class_component::{BaseComponent, Component, ComponentError, ComponentResult};
pub use compiler::{
    ComponentCompileError, ComponentCompileResult, ComponentCompiler, ComponentCompilerBuilder,
};
pub use parser::{ComponentParser, ComponentTag, ParseError};
pub use props::{ComponentProps, PropDefinition, PropError, PropType};
pub use registry::{ComponentRegistry, RegistryError, RegistryResult};
pub use slots::{Slot, SlotBag};
