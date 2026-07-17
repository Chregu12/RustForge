//! Test fakes for queue, event, mail, storage, and time systems
//!
//! Provides fake implementations for testing that record all interactions
//! and allow assertions on what was called.
//!
//! # Available Fakes
//!
//! - [`EventFake`] - Records dispatched events
//! - [`QueueFake`] - Records queued jobs
//! - [`MailFake`] - Records sent emails
//! - [`StorageFake`] - In-memory file storage
//! - [`TimeFake`] - Time travel and freezing

pub mod event;
pub mod mail;
pub mod queue;
pub mod storage;
pub mod time;

pub use event::EventFake;
pub use mail::{MailFake, MailRecord};
pub use queue::QueueFake;
pub use storage::{create_fake_file, create_fake_image, FakeUploadedFile, FileRecord, StorageFake, Visibility};
pub use time::{current_time, reset_test_clock, set_test_clock, Clock, FakeClock, SystemClock, TimeFake};
