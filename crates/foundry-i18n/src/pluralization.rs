//! Pluralization rules

pub struct PluralRules;

impl PluralRules {
    pub fn select(locale: &str, count: usize) -> &'static str {
        match locale {
            "en" => Self::english(count),
            "de" => Self::german(count),
            _ => Self::english(count),
        }
    }

    fn english(count: usize) -> &'static str {
        if count == 1 {
            "one"
        } else {
            "other"
        }
    }

    fn german(count: usize) -> &'static str {
        if count == 1 {
            "one"
        } else {
            "other"
        }
    }
}
