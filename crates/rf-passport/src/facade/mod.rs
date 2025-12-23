//! Passport facade module providing Laravel-style static OAuth2 API

pub mod config;
pub mod manager;
pub mod passport;

pub use config::{GrantControl, PkceControl, TokenLifetimes};
pub use manager::{PassportManager, GLOBAL_PASSPORT};
pub use passport::Passport;
