//! Internationalization (i18n) System for RustForge
//!
//! This crate provides multi-language support with translation management,
//! including JSON catalogs, Handlebars interpolation, pluralization, locale
//! fallback, and (behind the `axum` Cargo feature) an axum-0.8
//! `Accept-Language` extractor for per-request locale negotiation.
//!
//! # Axum integration (opt-in)
//!
//! Enable the `axum` feature to get the [`AcceptLanguage`] extractor:
//!
//! ```toml
//! [dependencies]
//! rf-i18n = { path = "...", features = ["axum"] }
//! ```
//!
//! ```ignore
//! use std::sync::Arc;
//! use axum::{routing::get, Router, Extension};
//! use rf_i18n::{I18n, AcceptLanguage};
//!
//! async fn hello(AcceptLanguage(locale): AcceptLanguage, Extension(i18n): Extension<Arc<I18n>>) -> String {
//!     let local = i18n.for_locale(&locale);
//!     local.t("greeting", None).unwrap_or_default()
//! }
//!
//! let i18n: Arc<I18n> = Arc::new(/* … */);
//! let app = Router::new()
//!     .route("/hello", get(hello))
//!     .layer(Extension(i18n));
//! ```

use handlebars::Handlebars;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
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
    /// Get plural rule for English-family languages (en, es, it, pt, …)
    pub fn for_english(count: i64) -> Self {
        if count == 0 {
            PluralRule::Zero
        } else if count == 1 {
            PluralRule::One
        } else {
            PluralRule::Other
        }
    }

    /// Get plural rule for German-family languages (de, nl, …)
    pub fn for_german(count: i64) -> Self {
        if count == 1 {
            PluralRule::One
        } else {
            PluralRule::Other
        }
    }

    /// Get plural rule for French-family languages (fr, pt-BR, …)
    pub fn for_french(count: i64) -> Self {
        if count == 0 || count == 1 {
            PluralRule::One
        } else {
            PluralRule::Other
        }
    }

    /// Get plural rule for Slavic languages (ru, uk, be, …) per CLDR.
    ///
    /// Forms: one / few / many. There is no "other" for integers in these
    /// locales; "many" covers the residual category.
    pub fn for_slavic(count: i64) -> Self {
        let n = count.unsigned_abs();
        if n % 10 == 1 && n % 100 != 11 {
            PluralRule::One
        } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
            PluralRule::Few
        } else {
            PluralRule::Many
        }
    }

    /// Get plural rule for Arabic (ar) per CLDR.
    ///
    /// Forms: zero / one / two / few / many / other.
    pub fn for_arabic(count: i64) -> Self {
        let n = count.unsigned_abs();
        if n == 0 {
            PluralRule::Zero
        } else if n == 1 {
            PluralRule::One
        } else if n == 2 {
            PluralRule::Two
        } else if (3..=10).contains(&(n % 100)) {
            PluralRule::Few
        } else if (11..=99).contains(&(n % 100)) {
            PluralRule::Many
        } else {
            PluralRule::Other
        }
    }

    /// Get plural rule key suitable for use as a catalog sub-key
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

    /// Get a translation (supports dot-separated nested keys like `"messages.welcome"`)
    pub fn get(&self, key: &str) -> Option<&Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = self.translations.get(parts[0])?;

        for part in parts.iter().skip(1) {
            current = current.get(part)?;
        }

        Some(current)
    }
}

/// i18n instance
///
/// Cheaply [`Clone`]-able — the translation catalogs live behind an [`Arc`] so
/// cloning copies the [`Arc`] pointer, not the catalog data. This makes it
/// ergonomic to share a single configured instance via `Arc<I18n>` and obtain
/// a lightweight per-request view with [`I18n::for_locale`].
#[derive(Clone)]
pub struct I18n {
    locale: String,
    fallback_locale: String,
    catalogs: Arc<HashMap<String, TranslationCatalog>>,
    handlebars: Handlebars<'static>,
}

impl I18n {
    /// Create a new i18n instance with the given default locale
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            fallback_locale: "en".to_string(),
            catalogs: Arc::new(HashMap::new()),
            handlebars: Handlebars::new(),
        }
    }

    /// Set fallback locale (used when a key is absent in the current locale)
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

    /// Set the current locale in place
    pub fn set_locale(&mut self, locale: impl Into<String>) {
        self.locale = locale.into();
    }

    /// Return a cloned view of this `I18n` with a different active locale.
    ///
    /// The catalog data is shared (behind `Arc`) so this clone is cheap.
    /// Typical use: obtain a per-request `I18n` from a shared `Arc<I18n>`
    /// based on the `Accept-Language` header.
    ///
    /// ```ignore
    /// let shared: Arc<I18n> = /* … */;
    /// let local = shared.for_locale("de");
    /// local.t("greeting", None)?;
    /// ```
    pub fn for_locale(&self, locale: impl Into<String>) -> I18n {
        let mut i = self.clone();
        i.locale = locale.into();
        i
    }

    /// Translate a key, optionally interpolating `data` into the template.
    ///
    /// Handlebars placeholders that are present in the translation but absent
    /// in `data` (or when `data` is `None`) resolve to an empty string rather
    /// than leaking the raw `{{…}}` syntax into the output.
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

    /// Translate with pluralization based on `count`.
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

    /// Format a date (simplified; for production use chrono with locale formatting)
    pub fn format_date(&self, timestamp: i64, format: &str) -> String {
        match format {
            "short" => format!("{}", timestamp),
            "long" => format!("Date: {}", timestamp),
            _ => format!("{}", timestamp),
        }
    }

    /// Format a number with locale-specific formatting (simplified)
    pub fn format_number(&self, number: f64) -> String {
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

    /// Get plural rule for the current locale.
    ///
    /// Locales not explicitly listed fall back to English rules and emit a
    /// `tracing::warn` so callers can add the missing rule or supply a catalog
    /// that sidesteps the issue.
    fn get_plural_rule(&self, count: i64) -> PluralRule {
        match self.locale.as_str() {
            // German family: one / other
            "de" | "nl" | "af" | "sq" | "az" | "hy" | "ka" | "lb" | "mk" | "sw" => {
                PluralRule::for_german(count)
            }
            // French family: one (0 or 1) / other
            "fr" | "pt-br" | "pt-BR" | "am" | "ff" | "gu" | "hi" | "ln" | "mg" | "mr"
            | "ti" | "wa" => PluralRule::for_french(count),
            // Slavic family: one / few / many
            "ru" | "uk" | "be" | "bs" | "hr" | "sr" | "sh" => PluralRule::for_slavic(count),
            // Arabic: zero / one / two / few / many / other
            "ar" => PluralRule::for_arabic(count),
            // English family: zero / one / other
            "en" | "es" | "it" | "pt" | "da" | "fi" | "nb" | "sv" | "el" | "he" | "hu"
            | "id" | "ja" | "ko" | "ms" | "th" | "tr" | "vi" | "zh" => {
                PluralRule::for_english(count)
            }
            other => {
                tracing::warn!(
                    locale = other,
                    "rf-i18n: no plural rules for locale '{}'; \
                     falling back to English rules (one/other). \
                     Add a PluralRule::for_<locale> impl or use a two-form catalog.",
                    other
                );
                PluralRule::for_english(count)
            }
        }
    }

    /// Render a translation value with Handlebars interpolation.
    ///
    /// Always passes `data` (or an empty object when `None`) through the
    /// Handlebars engine so that unmatched `{{…}}` placeholders resolve to
    /// empty strings rather than leaking raw template syntax into responses.
    fn render_translation(&self, translation: &Value, data: Option<Value>) -> I18nResult<String> {
        match translation {
            Value::String(s) => {
                let ctx = data.unwrap_or_else(|| serde_json::json!({}));
                self.handlebars
                    .render_template(s, &ctx)
                    .map_err(|e| I18nError::TemplateError(e.to_string()))
            }
            _ => Ok(translation.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// Optional axum integration
// ---------------------------------------------------------------------------

#[cfg(feature = "axum")]
pub use axum_integration::AcceptLanguage;

#[cfg(feature = "axum")]
mod axum_integration {
    use axum::{
        extract::FromRequestParts,
        http::{header, request::Parts},
    };

    /// Parse an `Accept-Language` header value into a best-match BCP-47 primary
    /// language tag (e.g. `"de-DE,de;q=0.9,en;q=0.8"` → `"de"`).
    ///
    /// The string is lowercased and only the primary subtag before the first `-`
    /// is returned (`"zh-Hant"` → `"zh"`). If parsing fails or the header is
    /// empty, returns `"en"`.
    pub(super) fn parse_accept_language(header: &str) -> String {
        let mut locales: Vec<(f32, &str)> = header
            .split(',')
            .filter_map(|entry| {
                let entry = entry.trim();
                if entry.is_empty() {
                    return None;
                }
                if let Some(semi) = entry.find(';') {
                    let lang = entry[..semi].trim();
                    let q = entry[semi + 1..]
                        .split(';')
                        .find_map(|p| {
                            p.trim()
                                .strip_prefix("q=")
                                .and_then(|v| v.parse::<f32>().ok())
                        })
                        .unwrap_or(1.0);
                    if lang.is_empty() {
                        None
                    } else {
                        Some((q, lang))
                    }
                } else {
                    Some((1.0, entry))
                }
            })
            .collect();

        // Stable sort: highest q first; preserve order for equal q values.
        locales.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        locales
            .first()
            .map(|(_, lang)| {
                lang.split('-')
                    .next()
                    .unwrap_or("en")
                    .to_ascii_lowercase()
            })
            .unwrap_or_else(|| "en".to_string())
    }

    /// Axum extractor that resolves the best-match locale for a request.
    ///
    /// Resolution order (first match wins):
    /// 1. `?locale=<tag>` query parameter
    /// 2. `Accept-Language` header (highest-weight tag)
    /// 3. Falls back to `"en"`
    ///
    /// Only the primary subtag is returned: `"de-DE"` → `"de"`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use axum::{routing::get, Router, Extension};
    /// use rf_i18n::{I18n, AcceptLanguage};
    ///
    /// async fn greet(
    ///     AcceptLanguage(locale): AcceptLanguage,
    ///     Extension(i18n): Extension<Arc<I18n>>,
    /// ) -> String {
    ///     i18n.for_locale(&locale).t("greeting", None).unwrap_or_default()
    /// }
    /// ```
    #[derive(Debug, Clone)]
    pub struct AcceptLanguage(pub String);

    impl<S> FromRequestParts<S> for AcceptLanguage
    where
        S: Send + Sync,
    {
        type Rejection = std::convert::Infallible;

        async fn from_request_parts(
            parts: &mut Parts,
            _state: &S,
        ) -> Result<Self, Self::Rejection> {
            // 1. ?locale= query param takes precedence (easy per-request override).
            if let Some(query) = parts.uri.query() {
                for pair in query.split('&') {
                    if let Some(val) = pair.strip_prefix("locale=") {
                        let lang = val
                            .split('-')
                            .next()
                            .unwrap_or("en")
                            .to_ascii_lowercase();
                        if !lang.is_empty() {
                            return Ok(AcceptLanguage(lang));
                        }
                    }
                }
            }

            // 2. Accept-Language header.
            if let Some(value) = parts.headers.get(header::ACCEPT_LANGUAGE) {
                if let Ok(s) = value.to_str() {
                    return Ok(AcceptLanguage(parse_accept_language(s)));
                }
            }

            // 3. Default.
            Ok(AcceptLanguage("en".to_string()))
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
            .add(
                "welcome",
                Value::String("Willkommen, {{name}}!".to_string()),
            )
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

    // --- New regression tests for the four fixes ---

    /// Fix 1: I18n must implement Clone so Arc<I18n> patterns work.
    #[test]
    fn test_i18n_clone() {
        let i18n = create_test_i18n();
        let cloned = i18n.clone();
        assert_eq!(cloned.locale(), "en");
        assert_eq!(cloned.t("goodbye", None).unwrap(), "Goodbye!");
    }

    /// Fix 1b: for_locale() returns a cheap clone with the desired locale.
    #[test]
    fn test_for_locale() {
        let i18n = create_test_i18n();
        let de = i18n.for_locale("de");
        assert_eq!(de.locale(), "de");
        assert_eq!(de.t("goodbye", None).unwrap(), "Auf Wiedersehen!");
        // Original is unchanged.
        assert_eq!(i18n.locale(), "en");
    }

    /// Fix 2: t(key, None) with a placeholder must NOT leak {{…}} into output.
    #[test]
    fn test_no_template_leak_with_none_data() {
        let i18n = create_test_i18n();
        // "welcome" = "Welcome, {{name}}!" — data is None
        let result = i18n.t("welcome", None).unwrap();
        assert!(
            !result.contains("{{"),
            "raw Handlebars syntax leaked into output: {result}"
        );
        // Handlebars renders the missing variable as an empty string.
        assert_eq!(result, "Welcome, !");
    }

    /// Fix 3: Slavic plural rules.
    #[test]
    fn test_plural_rules_slavic() {
        assert_eq!(PluralRule::for_slavic(1), PluralRule::One);
        assert_eq!(PluralRule::for_slavic(11), PluralRule::Many); // n%10==1 but n%100==11
        assert_eq!(PluralRule::for_slavic(21), PluralRule::One);
        assert_eq!(PluralRule::for_slavic(2), PluralRule::Few);
        assert_eq!(PluralRule::for_slavic(12), PluralRule::Many); // n%10==2 but n%100==12
        assert_eq!(PluralRule::for_slavic(22), PluralRule::Few);
        assert_eq!(PluralRule::for_slavic(5), PluralRule::Many);
        assert_eq!(PluralRule::for_slavic(0), PluralRule::Many);
    }

    /// Fix 3: Arabic plural rules.
    #[test]
    fn test_plural_rules_arabic() {
        assert_eq!(PluralRule::for_arabic(0), PluralRule::Zero);
        assert_eq!(PluralRule::for_arabic(1), PluralRule::One);
        assert_eq!(PluralRule::for_arabic(2), PluralRule::Two);
        assert_eq!(PluralRule::for_arabic(5), PluralRule::Few);   // 5 % 100 = 5 ∈ 3..10
        assert_eq!(PluralRule::for_arabic(11), PluralRule::Many); // 11 % 100 = 11 ∈ 11..99
        assert_eq!(PluralRule::for_arabic(100), PluralRule::Other);
    }

    /// Fix 3: get_plural_rule on a Russian locale (I18n-level wiring).
    #[test]
    fn test_i18n_slavic_plural_rule_wiring() {
        let mut i18n = create_test_i18n();
        i18n.set_locale("ru");
        // The "items" catalog only has "one"/"other" keys, but the plural rule
        // for "ru"/count=21 is One → the "one" catalog key should resolve.
        let result = i18n.t_plural("items", 21).unwrap();
        assert_eq!(result, "1 item");
    }

    /// Fix 3: unhandled locale falls back without panicking (warns via tracing).
    #[test]
    fn test_unhandled_locale_plural_no_panic() {
        let mut i18n = create_test_i18n();
        i18n.set_locale("xx"); // unknown locale
        // Should not panic; falls back to English rules.
        let result = i18n.t_plural("items", 5).unwrap();
        assert_eq!(result, "5 items");
    }

    /// Fix 4: parse_accept_language helper (requires the `axum` feature).
    #[cfg(feature = "axum")]
    #[test]
    fn test_parse_accept_language() {
        use super::axum_integration::parse_accept_language;
        assert_eq!(parse_accept_language("de-DE,de;q=0.9,en;q=0.8"), "de");
        assert_eq!(parse_accept_language("fr"), "fr");
        assert_eq!(parse_accept_language("en-US,en;q=0.9,de;q=0.8"), "en");
        // Highest q wins even if listed second.
        assert_eq!(parse_accept_language("de;q=0.8,fr;q=0.9"), "fr");
        // Empty header → default.
        assert_eq!(parse_accept_language(""), "en");
    }
}
