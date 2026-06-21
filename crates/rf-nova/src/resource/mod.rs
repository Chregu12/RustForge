pub mod crud;
pub mod field;
#[allow(clippy::module_inception)] // intentional: re-exported submodule name
pub mod resource;

pub use crud::*;
pub use field::*;
pub use resource::*;
