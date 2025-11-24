//! Internationalization (i18n) helpers.
//!
//! This module provides Laravel-style translation helpers.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Translation key-value storage
type Translations = HashMap<String, HashMap<String, String>>;

/// Global translator
pub struct Translator {
    /// Translations by locale
    translations: RwLock<Translations>,
    /// Default locale
    default_locale: RwLock<String>,
    /// Fallback locale
    fallback_locale: RwLock<String>,
}

impl Translator {
    /// Create a new translator
    pub fn new() -> Self {
        Self {
            translations: RwLock::new(HashMap::new()),
            default_locale: RwLock::new("en".to_string()),
            fallback_locale: RwLock::new("en".to_string()),
        }
    }

    /// Set the default locale
    pub fn set_locale(&self, locale: impl Into<String>) {
        *self.default_locale.write() = locale.into();
    }

    /// Get the current locale
    pub fn get_locale(&self) -> String {
        self.default_locale.read().clone()
    }

    /// Set the fallback locale
    pub fn set_fallback_locale(&self, locale: impl Into<String>) {
        *self.fallback_locale.write() = locale.into();
    }

    /// Add translations for a locale
    pub fn add_translations(&self, locale: impl Into<String>, translations: HashMap<String, String>) {
        let locale = locale.into();
        let mut all_translations = self.translations.write();

        all_translations
            .entry(locale)
            .or_insert_with(HashMap::new)
            .extend(translations);
    }

    /// Add a single translation
    pub fn add(&self, locale: impl Into<String>, key: impl Into<String>, value: impl Into<String>) {
        let locale = locale.into();
        let mut all_translations = self.translations.write();

        all_translations
            .entry(locale)
            .or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
    }

    /// Get a translation for a key
    pub fn get(&self, key: &str) -> String {
        self.get_with_locale(key, &self.get_locale())
    }

    /// Get a translation for a key with a specific locale
    pub fn get_with_locale(&self, key: &str, locale: &str) -> String {
        let translations = self.translations.read();

        // Try the requested locale
        if let Some(locale_translations) = translations.get(locale) {
            if let Some(value) = locale_translations.get(key) {
                return value.clone();
            }
        }

        // Try the fallback locale
        let fallback = self.fallback_locale.read().clone();
        if let Some(fallback_translations) = translations.get(&fallback) {
            if let Some(value) = fallback_translations.get(key) {
                return value.clone();
            }
        }

        // Return the key itself if no translation found
        key.to_string()
    }

    /// Get a translation with replacements
    ///
    /// Replacements should be in the format `:key` in the translation string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::translation::global_translator;
    /// use std::collections::HashMap;
    ///
    /// let translator = global_translator();
    /// translator.add("en", "welcome", "Welcome, :name!");
    ///
    /// let mut replacements = HashMap::new();
    /// replacements.insert("name".to_string(), "John".to_string());
    ///
    /// let result = translator.get_with_replacements("welcome", &replacements);
    /// assert_eq!(result, "Welcome, John!");
    /// ```
    pub fn get_with_replacements(&self, key: &str, replacements: &HashMap<String, String>) -> String {
        let mut translation = self.get(key);

        for (key, value) in replacements {
            let placeholder = format!(":{}", key);
            translation = translation.replace(&placeholder, value);
        }

        translation
    }

    /// Check if a translation exists
    pub fn has(&self, key: &str) -> bool {
        let translations = self.translations.read();
        let locale = self.get_locale();

        if let Some(locale_translations) = translations.get(&locale) {
            return locale_translations.contains_key(key);
        }

        false
    }

    /// Get all translations for a locale
    pub fn all(&self, locale: &str) -> HashMap<String, String> {
        self.translations
            .read()
            .get(locale)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear all translations (for testing)
    pub fn clear(&self) {
        self.translations.write().clear();
    }

    /// Get the number of loaded locales
    pub fn locale_count(&self) -> usize {
        self.translations.read().len()
    }
}

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

/// Global translator instance
static TRANSLATOR: Lazy<Translator> = Lazy::new(Translator::new);

/// Get the global translator
pub fn global_translator() -> &'static Translator {
    &TRANSLATOR
}

/// Translate a key using the default locale.
///
/// This is the main translation function, mimicking Laravel's `__()` helper.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::__;
///
/// let greeting = __("greeting.hello");
/// ```
pub fn __(key: &str) -> String {
    global_translator().get(key)
}

/// Translate a key with replacements.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::translation::trans;
/// use std::collections::HashMap;
///
/// let mut replacements = HashMap::new();
/// replacements.insert("name".to_string(), "Alice".to_string());
///
/// let result = trans("greeting.hello_name", &replacements);
/// ```
pub fn trans(key: &str, replacements: &HashMap<String, String>) -> String {
    global_translator().get_with_replacements(key, replacements)
}

/// Translate a key with a specific locale.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::translation::trans_locale;
///
/// let greeting = trans_locale("greeting.hello", "de");
/// ```
pub fn trans_locale(key: &str, locale: &str) -> String {
    global_translator().get_with_locale(key, locale)
}

/// Get or set the application locale.
///
/// If a locale is provided, it sets the locale. Otherwise, it returns the current locale.
pub fn app_locale(locale: Option<&str>) -> String {
    if let Some(locale) = locale {
        global_translator().set_locale(locale);
    }
    global_translator().get_locale()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translator_new() {
        let translator = Translator::new();
        assert_eq!(translator.get_locale(), "en");
    }

    #[test]
    fn test_set_and_get_locale() {
        let translator = Translator::new();
        translator.set_locale("de");
        assert_eq!(translator.get_locale(), "de");
    }

    #[test]
    fn test_add_translation() {
        let translator = Translator::new();
        translator.add("en", "hello", "Hello");
        translator.add("de", "hello", "Hallo");

        translator.set_locale("en");
        assert_eq!(translator.get("hello"), "Hello");

        translator.set_locale("de");
        assert_eq!(translator.get("hello"), "Hallo");
    }

    #[test]
    fn test_add_translations_batch() {
        let translator = Translator::new();
        let mut translations = HashMap::new();
        translations.insert("hello".to_string(), "Hello".to_string());
        translations.insert("goodbye".to_string(), "Goodbye".to_string());

        translator.add_translations("en", translations);

        assert_eq!(translator.get("hello"), "Hello");
        assert_eq!(translator.get("goodbye"), "Goodbye");
    }

    #[test]
    fn test_get_missing_translation() {
        let translator = Translator::new();
        assert_eq!(translator.get("missing.key"), "missing.key");
    }

    #[test]
    fn test_fallback_locale() {
        let translator = Translator::new();
        translator.set_fallback_locale("en");
        translator.add("en", "hello", "Hello");

        translator.set_locale("de");
        // Should fall back to English
        assert_eq!(translator.get("hello"), "Hello");
    }

    #[test]
    fn test_get_with_replacements() {
        let translator = Translator::new();
        translator.add("en", "welcome", "Welcome, :name!");

        let mut replacements = HashMap::new();
        replacements.insert("name".to_string(), "John".to_string());

        let result = translator.get_with_replacements("welcome", &replacements);
        assert_eq!(result, "Welcome, John!");
    }

    #[test]
    fn test_multiple_replacements() {
        let translator = Translator::new();
        translator.add("en", "message", "Hello :name, you have :count messages");

        let mut replacements = HashMap::new();
        replacements.insert("name".to_string(), "Alice".to_string());
        replacements.insert("count".to_string(), "5".to_string());

        let result = translator.get_with_replacements("message", &replacements);
        assert_eq!(result, "Hello Alice, you have 5 messages");
    }

    #[test]
    fn test_has_translation() {
        let translator = Translator::new();
        translator.add("en", "hello", "Hello");

        assert!(translator.has("hello"));
        assert!(!translator.has("missing"));
    }

    #[test]
    fn test_all_translations() {
        let translator = Translator::new();
        translator.add("en", "hello", "Hello");
        translator.add("en", "goodbye", "Goodbye");

        let all = translator.all("en");
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("hello"), Some(&"Hello".to_string()));
    }

    #[test]
    fn test_double_underscore_helper() {
        global_translator().clear();
        global_translator().add("en", "test", "Test Value");

        assert_eq!(__("test"), "Test Value");
        assert_eq!(__("missing"), "missing");
    }

    #[test]
    fn test_trans_helper() {
        global_translator().clear();
        global_translator().add("en", "greeting", "Hello, :name!");

        let mut replacements = HashMap::new();
        replacements.insert("name".to_string(), "Bob".to_string());

        assert_eq!(trans("greeting", &replacements), "Hello, Bob!");
    }

    #[test]
    fn test_trans_locale_helper() {
        global_translator().clear();
        global_translator().add("en", "hello", "Hello");
        global_translator().add("de", "hello", "Hallo");

        assert_eq!(trans_locale("hello", "en"), "Hello");
        assert_eq!(trans_locale("hello", "de"), "Hallo");
    }

    #[test]
    fn test_app_locale_helper() {
        global_translator().clear();

        assert_eq!(app_locale(Some("de")), "de");
        assert_eq!(app_locale(None), "de");
    }

    #[test]
    fn test_locale_count() {
        let translator = Translator::new();
        translator.add("en", "test", "Test");
        translator.add("de", "test", "Test");
        translator.add("fr", "test", "Test");

        assert_eq!(translator.locale_count(), 3);
    }
}
