pub mod smtp;
pub mod transport;
pub mod config;

pub use smtp::SmtpTransport;
pub use transport::{MailTransport, TransportError, TransportResponse, TransportResult};
pub use config::SmtpConfig;
