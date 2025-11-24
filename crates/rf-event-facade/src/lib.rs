//! # rf-event-facade
//!
//! Laravel-style Event facade for the RustForge framework.
//!
//! ## Features
//!
//! - **Static Event API**: Use `Event::dispatch()`, `Event::listen()`, etc.
//! - **Global Event Dispatcher**: Thread-safe global event management
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_event_facade::Event;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct UserCreated {
//!     user_id: u64,
//!     email: String,
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Dispatch an event
//! Event::dispatch("user.created", UserCreated {
//!     user_id: 1,
//!     email: "user@example.com".to_string(),
//! }).await?;
//!
//! // Listen for events
//! Event::listen("user.created", |event: UserCreated| {
//!     println!("User created: {}", event.email);
//! }).await;
//!
//! // Check if event has listeners
//! if Event::has_listeners("user.created").await {
//!     println!("Event has listeners");
//! }
//!
//! // Forget all listeners for an event
//! Event::forget("user.created").await;
//! # Ok(())
//! # }
//! ```

pub mod facade;
pub mod manager;

pub use facade::Event;
pub use manager::{EventManager, GLOBAL_EVENT};
