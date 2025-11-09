//! Common mailable types

pub mod password_reset;
pub mod welcome;

pub use password_reset::PasswordResetEmail;
pub use welcome::WelcomeEmail;
