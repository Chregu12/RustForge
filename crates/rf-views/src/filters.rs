use chrono::DateTime;
use serde_json::Value;
use std::collections::HashMap;
use tera::{Filter, Result as TeraResult};

/// Filter for generating route URLs
pub struct RouteFilter {
    routes: HashMap<String, String>,
}

impl Default for RouteFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteFilter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn register_route(&mut self, name: &str, path: &str) {
        self.routes.insert(name.to_string(), path.to_string());
    }
}

impl Filter for RouteFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let route_name = value
            .as_str()
            .ok_or_else(|| tera::Error::msg("Route name must be a string"))?;

        let mut path = self
            .routes
            .get(route_name)
            .ok_or_else(|| tera::Error::msg(format!("Route not found: {}", route_name)))?
            .clone();

        // Replace route parameters
        if let Some(params) = args.get("params") {
            if let Some(params_obj) = params.as_object() {
                for (key, value) in params_obj {
                    let placeholder = format!("{{{}}}", key);
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        _ => value.to_string(),
                    };
                    path = path.replace(&placeholder, &replacement);
                }
            }
        }

        Ok(Value::String(path))
    }
}

/// Filter for generating asset URLs
pub struct AssetFilter {
    base_url: String,
    version: Option<String>,
}

impl AssetFilter {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            version: None,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

impl Filter for AssetFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
        let path = value
            .as_str()
            .ok_or_else(|| tera::Error::msg("Asset path must be a string"))?;

        let url = if let Some(version) = &self.version {
            format!("{}/{}?v={}", self.base_url, path, version)
        } else {
            format!("{}/{}", self.base_url, path)
        };

        Ok(Value::String(url))
    }
}

/// Filter for generating absolute URLs
pub struct UrlFilter {
    base_url: String,
}

impl UrlFilter {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }
}

impl Filter for UrlFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
        let path = value
            .as_str()
            .ok_or_else(|| tera::Error::msg("URL path must be a string"))?;

        let url = if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        };

        Ok(Value::String(url))
    }
}

/// Filter for date formatting
pub struct DateFilter;

impl Filter for DateFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let date_str = value
            .as_str()
            .ok_or_else(|| tera::Error::msg("Date must be a string"))?;

        let date = DateTime::parse_from_rfc3339(date_str)
            .map_err(|e| tera::Error::msg(format!("Invalid date format: {}", e)))?;

        let formatted = date.format(format).to_string();
        Ok(Value::String(formatted))
    }
}

/// Filter for currency formatting
pub struct MoneyFilter;

impl Filter for MoneyFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let amount = value
            .as_f64()
            .ok_or_else(|| tera::Error::msg("Money value must be a number"))?;

        let currency = args
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("USD");

        let symbol = match currency {
            "USD" => "$",
            "EUR" => "€",
            "GBP" => "£",
            "JPY" => "¥",
            _ => currency,
        };

        let formatted = format!("{}{:.2}", symbol, amount);
        Ok(Value::String(formatted))
    }
}

/// Filter for truncating text
pub struct TruncateFilter;

impl Filter for TruncateFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let text = value
            .as_str()
            .ok_or_else(|| tera::Error::msg("Value must be a string"))?;

        let length = args.get("length").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        let suffix = args.get("suffix").and_then(|v| v.as_str()).unwrap_or("...");

        if text.chars().count() <= length {
            Ok(Value::String(text.to_string()))
        } else {
            let truncated = text.chars().take(length).collect::<String>();
            Ok(Value::String(format!("{}{}", truncated, suffix)))
        }
    }
}

/// Filter for pluralization
pub struct PluralizeFilter;

impl Filter for PluralizeFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let count = value
            .as_u64()
            .ok_or_else(|| tera::Error::msg("Value must be a number"))?;

        let singular = args
            .get("singular")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("Singular form required"))?;

        let default_plural = format!("{}s", singular);
        let plural = args
            .get("plural")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_plural);

        let word = if count == 1 { singular } else { plural };

        Ok(Value::String(format!("{} {}", count, word)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_filter() {
        let filter = AssetFilter::new("/assets");
        let result = filter
            .filter(&Value::String("css/app.css".to_string()), &HashMap::new())
            .unwrap();
        assert_eq!(result.as_str().unwrap(), "/assets/css/app.css");
    }

    #[test]
    fn test_asset_filter_with_version() {
        let filter = AssetFilter::new("/assets").with_version("1.2.3");
        let result = filter
            .filter(&Value::String("css/app.css".to_string()), &HashMap::new())
            .unwrap();
        assert_eq!(result.as_str().unwrap(), "/assets/css/app.css?v=1.2.3");
    }

    #[test]
    fn test_url_filter() {
        let filter = UrlFilter::new("https://example.com");
        let result = filter
            .filter(&Value::String("/path".to_string()), &HashMap::new())
            .unwrap();
        assert_eq!(result.as_str().unwrap(), "https://example.com/path");
    }

    #[test]
    fn test_money_filter() {
        let filter = MoneyFilter;
        let mut args = HashMap::new();
        args.insert("currency".to_string(), Value::String("USD".to_string()));

        let value = serde_json::json!(42.5);
        let result = filter.filter(&value, &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "$42.50");
    }

    #[test]
    fn test_truncate_filter() {
        let filter = TruncateFilter;
        let mut args = HashMap::new();
        args.insert("length".to_string(), Value::Number(10.into()));

        let result = filter
            .filter(&Value::String("This is a long text".to_string()), &args)
            .unwrap();
        assert_eq!(result.as_str().unwrap(), "This is a ...");
    }

    #[test]
    fn test_pluralize_filter() {
        let filter = PluralizeFilter;
        let mut args = HashMap::new();
        args.insert("singular".to_string(), Value::String("item".to_string()));

        let result = filter.filter(&Value::Number(1.into()), &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "1 item");

        let result = filter.filter(&Value::Number(5.into()), &args).unwrap();
        assert_eq!(result.as_str().unwrap(), "5 items");
    }
}
