//! # Foundry Internationalization (i18n)
//!
//! Multi-language support with translation files and locale detection.

pub mod translator;
pub mod loader;
pub mod locale;
pub mod pluralization;

pub use translator::Translator;
pub use loader::{TranslationLoader, FileLoader};
pub use locale::{Locale, LocaleDetector};
pub use pluralization::PluralRules;

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
