//! Test fakes for queue and event systems
//!
//! Provides fake implementations for testing that record all interactions
//! and allow assertions on what was called.

pub mod event;
pub mod queue;

pub use event::EventFake;
pub use queue::QueueFake;
