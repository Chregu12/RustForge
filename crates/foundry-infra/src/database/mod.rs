/// Database infrastructure module
///
/// Provides high-performance database connection pooling and management.

pub mod pool;

pub use pool::{DatabasePool, PoolConfig, PoolError, PoolStats};
