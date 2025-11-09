//! Email backend implementations

pub mod memory;
pub mod mock;
pub mod smtp;

pub use memory::MemoryMailer;
pub use mock::MockMailer;
pub use smtp::{SmtpConfig, SmtpMailer};
