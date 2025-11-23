//! Built-in service providers for common services

mod application;
mod auth;
mod cache;
mod database;
mod mail;

pub use application::ApplicationServiceProvider;
pub use auth::AuthServiceProvider;
pub use cache::CacheServiceProvider;
pub use database::DatabaseServiceProvider;
pub use mail::MailServiceProvider;
