//! Common mailable types

pub mod invoice;
pub mod order_shipped;
pub mod password_reset;
pub mod welcome;

pub use invoice::InvoiceMail;
pub use order_shipped::OrderShippedMail;
pub use password_reset::PasswordResetEmail;
pub use welcome::WelcomeEmail;
