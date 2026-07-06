//! # rf-mail-facade
//!
//! Laravel-style `Mail` facade for the RustForge framework.
//!
//! This crate used to carry its **own** duplicate `Mail`/`Mailer` implementation
//! backed by a *separate* global `MemoryMailer` that only delivered via
//! `block_in_place` (and therefore required an ambient Tokio runtime). The real
//! `rf::Mail` (which resolves to `rf_mail::MailFacade`) meanwhile grew a proper
//! synchronous file/SMTP transport. The `send_mail!` helper macro expands to
//! `rf_mail_facade::Mail::…`, so it targeted the stale mock instead of the real
//! transport.
//!
//! It now simply **re-exports the single real implementation from
//! [`rf_mail`]**, so there is exactly one source of truth: `Mail::to(..).send(..)`
//! writes real `.eml` files (or delivers over SMTP when configured), with or
//! without a Tokio runtime.
//!
//! # Recommended Usage
//!
//! Prefer the consolidated `rf` crate (`use rf::Mail;`). When depending on this
//! crate directly:
//!
//! ```rust
//! use rf_mail_facade::Mail;
//! ```

// One source of truth: the real Mail facade lives in `rf-mail`.
pub use rf_mail::facade::{Mail, Mailer, GLOBAL_MAILER};

// Re-export commonly used types from rf-mail (kept for API stability).
pub use rf_mail::{MailResult, Mailable, MemoryMailer};
