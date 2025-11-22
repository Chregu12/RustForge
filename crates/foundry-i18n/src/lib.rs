//! # Foundry Internationalization (i18n)
//!
//! Multi-language support with translation files and locale detection.

pub mod loader;
pub mod locale;
pub mod pluralization;
pub mod translator;

pub use loader::{FileLoader, TranslationLoader};
pub use locale::{Locale, LocaleDetector};
pub use pluralization::PluralRules;
pub use translator::Translator;

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("Translation not found: {0}")]
    NotFound(String),

    #[error("Locale not supported: {0}")]
    UnsupportedLocale(String),

    #[error("Load error: {0}")]
    LoadError(String),
}

pub type Result<T> = std::result::Result<T, I18nError>;
