//! Built-in health check implementations

mod checks_impl;

pub use checks_impl::{AlwaysHealthyCheck, DiskCheck, MemoryCheck};

#[cfg(feature = "database")]
pub use checks_impl::DatabaseCheck;

#[cfg(feature = "redis-check")]
pub use checks_impl::RedisCheck;
