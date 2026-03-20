//! Deployment Smoke Tests for RustForge
//!
//! Run these tests after each deployment to verify all core functionality works:
//!
//! ```bash
//! cargo test -p deployment-tests
//! ```
//!
//! These tests cover all major crates and their public APIs without requiring
//! external services (no database, Redis, or SMTP connections needed).

// Test modules organized by crate
mod test_core;
mod test_validation;
mod test_auth;
mod test_cache;
mod test_config;
mod test_collections;
mod test_web;
mod test_routing;
mod test_pagination;
mod test_container;
mod test_eloquent;
mod test_mail;
mod test_storage;
mod test_encryption;
mod test_events;
mod test_queue;
mod test_scheduler;
mod test_health;
