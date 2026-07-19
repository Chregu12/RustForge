//! # rf-event-facade
//!
//! Laravel-style Event facade for the RustForge framework.
//!
//! ## Features
//!
//! - **Static Event API**: Use `Event::dispatch()`, `Event::listen()`, etc. - no `.await` needed!
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
//! fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Dispatch an event
//!     Event::dispatch("user.created", UserCreated {
//!         user_id: 1,
//!         email: "user@example.com".to_string(),
//!     })?;
//!
//!     // Listen for events
//!     Event::listen("user.created", |event| {
//!         println!("User created: {:?}", event);
//!     });
//!
//!     // Check if event has listeners
//!     if Event::has_listeners("user.created") {
//!         println!("Event has listeners");
//!     }
//!
//!     // Forget all listeners for an event
//!     Event::forget("user.created");
//!     Ok(())
//! }
//! ```

pub mod facade;
pub mod manager;
pub mod typed;

pub use facade::Event;
pub use manager::{EventManager, GLOBAL_EVENT};

// Typed, synchronous, in-process event surface (the vision's `event(payload)`).
// Dispatches by the payload's concrete type rather than by a string name.
pub use typed::{event, event_later, forget_all as forget_all_typed, listen};
