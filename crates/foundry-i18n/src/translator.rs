//! Translation engine

use std::collections::HashMap;
use crate::Locale;

pub struct Translator {
    translations: HashMap<String, HashMap<String, String>>,
    default_locale: Locale,
}

impl Translator {
    pub fn new(default_locale: Locale) -> Self {
        Self {
            translations: HashMap::new(),
            default_locale,
        }
    }

    pub fn add_translations(&mut self, locale: String, translations: HashMap<String, String>) {
        self.translations.insert(locale, translations);
    }

    pub fn translate(&self, key: &str, locale: Option<&Locale>) -> String {
        let locale = locale.unwrap_or(&self.default_locale);

        self.translations
            .get(&locale.code)
            .and_then(|t| t.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn trans(&self, key: &str) -> String {
        self.translate(key, None)
    }

    pub fn trans_with_locale(&self, key: &str, locale: &Locale) -> String {
        self.translate(key, Some(locale))
    }

    pub fn trans_choice(&self, key: &str, count: usize, locale: Option<&Locale>) -> String {
        let translation = self.translate(key, locale);
        // Implement pluralization logic
        translation.replace("{count}", &count.to_string())
    }
}
