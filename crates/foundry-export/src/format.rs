//! Data formatting utilities

use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub date_format: String,
    pub datetime_format: String,
    pub currency_symbol: String,
    pub decimal_places: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            date_format: "%Y-%m-%d".to_string(),
            datetime_format: "%Y-%m-%d %H:%M:%S".to_string(),
            currency_symbol: "$".to_string(),
            decimal_places: 2,
        }
    }
}

/// Data formatter for various types
pub struct DataFormatter {
    options: FormatOptions,
}

impl DataFormatter {
    pub fn new(options: FormatOptions) -> Self {
        Self { options }
    }

    pub fn format_currency(&self, value: f64) -> String {
        format!(
            "{}{}",
            self.options.currency_symbol,
            self.format_decimal(value)
        )
    }

    pub fn format_decimal(&self, value: f64) -> String {
        format!("{:.prec$}", value, prec = self.options.decimal_places)
    }

    pub fn format_percentage(&self, value: f64) -> String {
        format!("{}%", self.format_decimal(value * 100.0))
    }

    pub fn format_date(&self, date: NaiveDate) -> String {
        date.format(&self.options.date_format).to_string()
    }

    pub fn format_datetime(&self, datetime: DateTime<Utc>) -> String {
        datetime.format(&self.options.datetime_format).to_string()
    }

    pub fn format_boolean(&self, value: bool) -> String {
        if value {
            "Yes".to_string()
        } else {
            "No".to_string()
        }
    }
}

impl Default for DataFormatter {
    fn default() -> Self {
        Self::new(FormatOptions::default())
    }
}
