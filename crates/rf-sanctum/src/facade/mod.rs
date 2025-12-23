//! Sanctum facade module providing Laravel-style static token auth API

pub mod manager;
pub mod sanctum;

pub use manager::{SanctumManager, GLOBAL_SANCTUM};
pub use sanctum::Sanctum;
