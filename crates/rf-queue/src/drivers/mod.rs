//! Queue driver implementations
//!
//! This module contains various queue backend drivers.

#[cfg(feature = "database")]
pub mod database;

#[cfg(feature = "sqs")]
pub mod sqs;

pub mod failover;

// Re-exports
#[cfg(feature = "database")]
pub use database::DatabaseQueue;

#[cfg(feature = "sqs")]
pub use sqs::SqsQueue;

pub use failover::FailoverQueue;
