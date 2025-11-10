//! Internationalization (i18n) System for RustForge
//!
//! This crate provides multi-language support with translation management.

use handlebars::Handlebars;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::Arc,
};
use thiserror::Error;

/// i18n errors
#[derive(Debug, Error)]
pub enum I18nError {
    #[error("Translation not found: {0}")]
    TranslationNotFound(String),

    #[error("Locale not found: {0}")]
    LocaleNotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Template error: {0}")]
    TemplateError(String),
}

pub type I18nResult<T> = Result<T, I18nError>;

/// Pluralization rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluralRule {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl PluralRule {
    /// Get plural rule for English
    pub fn for_english(count: i64) -> Self {
        if count == 0 {
            PluralRule::Zero
        } else if count == 1 {
            PluralRule::One
        } else {
            PluralRule::Other
        }
    }

    /// Get plural rule for German
    pub fn for_german(count: i64) -> Self {
        if count == 1 {
            PluralRule::One
        } else {
            PluralRule::Other
        }
    }

    /// Get plural rule for French
    pub fn for_french(count: i64) -> Self {
        if count == 0 || count == 1 {
            PluralRule::One
        } else {
            PluralRule::Other
        }
    }

    /// Get plural rule key
    pub fn key(&self) -> &'static str {
        match self {
            PluralRule::Zero => "zero",
            PluralRule::One => "one",
            PluralRule::Two => "two",
            PluralRule::Few => "few",
            PluralRule::Many => "many",
            PluralRule::Other => "other",
        }
    }
}

/// Translation catalog
#[derive(Debug, Clone)]
pub struct TranslationCatalog {
    locale: String,
    translations: HashMap<String, Value>,
}

impl TranslationCatalog {
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            translations: HashMap::new(),
        }
    }

    /// Load translations from JSON
    pub fn load_json(mut self, json: &str) -> I18nResult<Self> {
        let data: HashMap<String, Value> =
            serde_json::from_str(json).map_err(|e| I18nError::ParseError(e.to_string()))?;

        self.translations = data;
        Ok(self)
    }

    /// Add a translation
    pub fn add(mut self, key: impl Into<String>, value: Value) -> Self {
        self.translations.insert(key.into(), value);
        self
    }

    /// Get a translation
    pub fn get(&self, key: &str) -> Option<&Value> {
        // Support nested keys like "messages.welcome"
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = self.translations.get(parts[0])?;

        for part in parts.iter().skip(1) {
            current = current.get(part)?;
        }

        Some(current)
    }
}

/// i18n instance
pub struct I18n {
    locale: String,
    fallback_locale: String,
    catalogs: Arc<HashMap<String, TranslationCatalog>>,
    handlebars: Handlebars<'static>,
}

impl I18n {
    /// Create a new i18n instance
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            fallback_locale: "en".to_string(),
            catalogs: Arc::new(HashMap::new()),
            handlebars: Handlebars::new(),
        }
    }

    /// Set fallback locale
    pub fn fallback(mut self, locale: impl Into<String>) -> Self {
        self.fallback_locale = locale.into();
        self
    }

    /// Add a translation catalog
    pub fn add_catalog(mut self, catalog: TranslationCatalog) -> Self {
        let mut catalogs = (*self.catalogs).clone();
        catalogs.insert(catalog.locale.clone(), catalog);
        self.catalogs = Arc::new(catalogs);
        self
    }

    /// Get the current locale
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Set the current locale
    pub fn set_locale(&mut self, locale: impl Into<String>) {
        self.locale = locale.into();
    }

    /// Translate a key
    pub fn t(&self, key: &str, data: Option<Value>) -> I18nResult<String> {
        // Try current locale first
        if let Some(catalog) = self.catalogs.get(&self.locale) {
            if let Some(translation) = catalog.get(key) {
                return self.render_translation(translation, data);
            }
        }

        // Try fallback locale
        if let Some(catalog) = self.catalogs.get(&self.fallback_locale) {
            if let Some(translation) = catalog.get(key) {
                return self.render_translation(translation, data);
            }
        }

        Err(I18nError::TranslationNotFound(key.to_string()))
    }

    /// Translate with pluralization
    pub fn t_plural(&self, key: &str, count: i64) -> I18nResult<String> {
        let plural_rule = self.get_plural_rule(count);
        let plural_key = format!("{}.{}", key, plural_rule.key());

        // Try to get plural-specific translation
        match self.t(&plural_key, Some(serde_json::json!({ "count": count }))) {
            Ok(translation) => Ok(translation),
            Err(_) => {
                // Fallback to "other" if specific rule not found
                let other_key = format!("{}.other", key);
                self.t(&other_key, Some(serde_json::json!({ "count": count })))
            }
        }
    }

    /// Format a date (simplified)
    pub fn format_date(&self, timestamp: i64, format: &str) -> String {
        // This is a simplified implementation
        // In production, use chrono with locale-specific formatting
        match format {
            "short" => format!("{}", timestamp),
            "long" => format!("Date: {}", timestamp),
            _ => format!("{}", timestamp),
        }
    }

    /// Format a number with locale-specific formatting
    pub fn format_number(&self, number: f64) -> String {
        // Simplified implementation
        // In production, use icu4x or similar for proper locale-specific formatting
        match self.locale.as_str() {
            "de" => format!("{:.2}", number).replace('.', ","),
            _ => format!("{:.2}", number),
        }
    }

    /// Format currency
    pub fn format_currency(&self, amount: f64, currency: &str) -> String {
        let formatted = self.format_number(amount);

        match (self.locale.as_str(), currency) {
            ("en", "USD") => format!("${}", formatted),
            ("de", "EUR") => format!("{} €", formatted),
            (_, _) => format!("{} {}", formatted, currency),
        }
    }

    /// Get plural rule for current locale
    fn get_plural_rule(&self, count: i64) -> PluralRule {
        match self.locale.as_str() {
            "de" => PluralRule::for_german(count),
            "fr" => PluralRule::for_french(count),
            _ => PluralRule::for_english(count),
        }
    }

    /// Render translation with interpolation
    fn render_translation(&self, translation: &Value, data: Option<Value>) -> I18nResult<String> {
        match translation {
            Value::String(s) => {
                if let Some(data) = data {
                    self.handlebars
                        .render_template(s, &data)
                        .map_err(|e| I18nError::TemplateError(e.to_string()))
                } else {
                    Ok(s.clone())
                }
            }
            _ => Ok(translation.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_i18n() -> I18n {
        let en_catalog = TranslationCatalog::new("en")
            .add("welcome", Value::String("Welcome, {{name}}!".to_string()))
            .add("goodbye", Value::String("Goodbye!".to_string()))
            .add(
                "items",
                serde_json::json!({
                    "one": "1 item",
                    "other": "{{count}} items"
                }),
            )
            .add(
                "messages",
                serde_json::json!({
                    "hello": "Hello, World!",
                    "nested": {
                        "deep": "Deep value"
                    }
                }),
            );

        let de_catalog = TranslationCatalog::new("de")
            .add("welcome", Value::String("Willkommen, {{name}}!".to_string()))
            .add("goodbye", Value::String("Auf Wiedersehen!".to_string()));

        I18n::new("en")
            .fallback("en")
            .add_catalog(en_catalog)
            .add_catalog(de_catalog)
    }

    #[test]
    fn test_simple_translation() {
        let i18n = create_test_i18n();
        let result = i18n.t("goodbye", None).unwrap();
        assert_eq!(result, "Goodbye!");
    }

    #[test]
    fn test_translation_with_interpolation() {
        let i18n = create_test_i18n();
        let result = i18n
            .t("welcome", Some(serde_json::json!({ "name": "John" })))
            .unwrap();
        assert_eq!(result, "Welcome, John!");
    }

    #[test]
    fn test_nested_translation_key() {
        let i18n = create_test_i18n();
        let result = i18n.t("messages.hello", None).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_deeply_nested_translation() {
        let i18n = create_test_i18n();
        let result = i18n.t("messages.nested.deep", None).unwrap();
        assert_eq!(result, "Deep value");
    }

    #[test]
    fn test_locale_switching() {
        let mut i18n = create_test_i18n();

        let en_result = i18n.t("goodbye", None).unwrap();
        assert_eq!(en_result, "Goodbye!");

        i18n.set_locale("de");
        let de_result = i18n.t("goodbye", None).unwrap();
        assert_eq!(de_result, "Auf Wiedersehen!");
    }

    #[test]
    fn test_fallback_locale() {
        let mut i18n = create_test_i18n();
        i18n.set_locale("de");

        // "messages.hello" only exists in English catalog
        let result = i18n.t("messages.hello", None).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_translation_not_found() {
        let i18n = create_test_i18n();
        let result = i18n.t("nonexistent.key", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_plural_one() {
        let i18n = create_test_i18n();
        let result = i18n.t_plural("items", 1).unwrap();
        assert_eq!(result, "1 item");
    }

    #[test]
    fn test_plural_other() {
        let i18n = create_test_i18n();
        let result = i18n.t_plural("items", 5).unwrap();
        assert_eq!(result, "5 items");
    }

    #[test]
    fn test_plural_zero() {
        let i18n = create_test_i18n();
        let result = i18n.t_plural("items", 0).unwrap();
        assert_eq!(result, "0 items");
    }

    #[test]
    fn test_number_formatting_en() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.format_number(1234.56), "1234.56");
    }

    #[test]
    fn test_number_formatting_de() {
        let i18n = I18n::new("de");
        assert_eq!(i18n.format_number(1234.56), "1234,56");
    }

    #[test]
    fn test_currency_formatting_usd() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.format_currency(1234.56, "USD"), "$1234.56");
    }

    #[test]
    fn test_currency_formatting_eur() {
        let i18n = I18n::new("de");
        assert_eq!(i18n.format_currency(1234.56, "EUR"), "1234,56 €");
    }

    #[test]
    fn test_plural_rules_english() {
        assert_eq!(PluralRule::for_english(0), PluralRule::Zero);
        assert_eq!(PluralRule::for_english(1), PluralRule::One);
        assert_eq!(PluralRule::for_english(2), PluralRule::Other);
        assert_eq!(PluralRule::for_english(100), PluralRule::Other);
    }

    #[test]
    fn test_plural_rules_german() {
        assert_eq!(PluralRule::for_german(1), PluralRule::One);
        assert_eq!(PluralRule::for_german(0), PluralRule::Other);
        assert_eq!(PluralRule::for_german(2), PluralRule::Other);
    }

    #[test]
    fn test_plural_rules_french() {
        assert_eq!(PluralRule::for_french(0), PluralRule::One);
        assert_eq!(PluralRule::for_french(1), PluralRule::One);
        assert_eq!(PluralRule::for_french(2), PluralRule::Other);
    }

    #[test]
    fn test_catalog_from_json() {
        let json = r#"{"greeting": "Hello", "farewell": "Goodbye"}"#;
        let catalog = TranslationCatalog::new("en").load_json(json).unwrap();

        assert_eq!(catalog.get("greeting").unwrap(), "Hello");
        assert_eq!(catalog.get("farewell").unwrap(), "Goodbye");
    }
}
