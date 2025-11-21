//! Test fakes for queue and event systems
//!
//! Provides fake implementations for testing that record all interactions
//! and allow assertions on what was called.

pub mod queue;
pub mod event;

pub use queue::QueueFake;
pub use event::EventFake;
