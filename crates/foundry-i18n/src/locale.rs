//! Locale detection and management

#[derive(Debug, Clone)]
pub struct Locale {
    pub code: String,
    pub name: String,
}

impl Locale {
    pub fn new(code: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
        }
    }

    pub fn en() -> Self {
        Self::new("en", "English")
    }

    pub fn de() -> Self {
        Self::new("de", "Deutsch")
    }

    pub fn fr() -> Self {
        Self::new("fr", "Français")
    }
}

pub struct LocaleDetector {
    supported_locales: Vec<String>,
    default_locale: String,
}

impl LocaleDetector {
    pub fn new(supported: Vec<String>, default: String) -> Self {
        Self {
            supported_locales: supported,
            default_locale: default,
        }
    }

    pub fn detect_from_header(&self, accept_language: &str) -> String {
        for part in accept_language.split(',') {
            let locale = part.split(';').next().unwrap_or("").trim();
            if self.supported_locales.contains(&locale.to_string()) {
                return locale.to_string();
            }
        }
        self.default_locale.clone()
    }
}
