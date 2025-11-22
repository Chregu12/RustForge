//! Session Management
//!
//! Provides session management with multiple backend drivers:
//! - Cookie-based sessions
//! - Database-backed sessions
//! - Redis-backed sessions
//! - Flash data support
//! - Session regeneration for security

pub mod driver;
pub mod middleware;
pub mod store;

pub use driver::{CookieSessionDriver, DatabaseSessionDriver, RedisSessionDriver, SessionDriver};
pub use middleware::{SessionConfig, SessionMiddleware};
pub use store::{Session, SessionStore};
