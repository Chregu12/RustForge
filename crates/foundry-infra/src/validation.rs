use async_trait::async_trait;
use foundry_plugins::{
    CommandError, ValidationPort, ValidationReport, ValidationRules, ValidationViolation,
};
use serde_json::Value;

#[derive(Debug, Default)]
pub struct SimpleValidationService;

#[async_trait]
impl ValidationPort for SimpleValidationService {
    async fn validate(
        &self,
        payload: Value,
        rules: ValidationRules,
    ) -> Result<ValidationReport, CommandError> {
        let mut errors = Vec::new();

        if let Some(required) = rules
            .rules
            .get("required")
            .and_then(|value| value.as_array())
        {
            for field in required.iter().filter_map(|value| value.as_str()) {
                if is_missing(&payload, field) {
                    errors.push(ValidationViolation {
                        field: field.to_string(),
                        message: "Field is required".to_string(),
                        code: Some("required".to_string()),
                    });
                }
            }
        }

        if let Some(fields) = rules
            .rules
            .get("fields")
            .and_then(|value| value.as_object())
        {
            for (field, config) in fields {
                if let Some(map) = config.as_object() {
                    apply_field_rules(&mut errors, &payload, field, map);
                }
            }
        }

        Ok(ValidationReport::with_errors(errors))
    }
}

fn apply_field_rules(
    errors: &mut Vec<ValidationViolation>,
    payload: &Value,
    field: &str,
    map: &serde_json::Map<String, Value>,
) {
    if let Some(min) = map.get("min_length").and_then(|value| value.as_u64()) {
        if !string_length_at_least(payload, field, min as usize) {
            errors.push(violation(
                field,
                &format!("Must be at least {min} characters"),
                "min_length",
            ));
        }
    }

    if let Some(max) = map.get("max_length").and_then(|value| value.as_u64()) {
        if !string_length_at_most(payload, field, max as usize) {
            errors.push(violation(
                field,
                &format!("Must be at most {max} characters"),
                "max_length",
            ));
        }
    }

    if let Some(range) = map.get("length_between").and_then(|value| value.as_array()) {
        if let Some((min, max)) = parse_range(range) {
            if !string_length_between(payload, field, min as usize, max as usize) {
                errors.push(violation(
                    field,
                    &format!("Length must be between {min} and {max}"),
                    "length_between",
                ));
            }
        }
    }

    if let Some(pattern) = map.get("regex").and_then(|value| value.as_str()) {
        if !matches_regex(payload, field, pattern) {
            errors.push(violation(field, "Format is invalid", "regex"));
        }
    }

    if map
        .get("email")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        if !is_email(payload, field) {
            errors.push(violation(field, "Must be a valid email address", "email"));
        }
    }

    if map
        .get("numeric")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        if !is_numeric(payload, field) {
            errors.push(violation(field, "Must be numeric", "numeric"));
        }
    }

    if let Some(range) = map.get("between").and_then(|value| value.as_array()) {
        if let Some((min, max)) = parse_range(range) {
            if !is_between(payload, field, min, max) {
                errors.push(violation(
                    field,
                    &format!("Must be between {min} and {max}"),
                    "between",
                ));
            }
        }
    }

    if let Some(min) = map.get("min").and_then(|value| value.as_f64()) {
        if !numeric_min(payload, field, min) {
            errors.push(violation(field, &format!("Must be at least {min}"), "min"));
        }
    }

    if let Some(max) = map.get("max").and_then(|value| value.as_f64()) {
        if !numeric_max(payload, field, max) {
            errors.push(violation(field, &format!("Must be at most {max}"), "max"));
        }
    }

    if let Some(enum_values) = map.get("in").and_then(|value| value.as_array()) {
        if !is_in_list(payload, field, enum_values) {
            errors.push(violation(field, "Value not allowed", "in"));
        }
    }

    if let Some(custom) = map.get("custom_predicate").and_then(|value| value.as_str()) {
        if !apply_custom(custom, payload, field, map) {
            let message = map
                .get("custom_message")
                .and_then(|value| value.as_str())
                .unwrap_or("Custom rule validation failed");
            errors.push(violation(field, message, custom));
        }
    }
}

fn is_missing(payload: &Value, field: &str) -> bool {
    match lookup(payload, field) {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(value)) => value.trim().is_empty(),
        Some(Value::Array(values)) => values.is_empty(),
        _ => false,
    }
}

fn string_length_at_least(payload: &Value, field: &str, min: usize) -> bool {
    lookup(payload, field)
        .and_then(|value| value.as_str())
        .map(|value| value.chars().count() >= min)
        .unwrap_or(false)
}

fn string_length_at_most(payload: &Value, field: &str, max: usize) -> bool {
    lookup(payload, field)
        .and_then(|value| value.as_str())
        .map(|value| value.chars().count() <= max)
        .unwrap_or(true)
}

fn string_length_between(payload: &Value, field: &str, min: usize, max: usize) -> bool {
    let length = lookup(payload, field)
        .and_then(|value| value.as_str())
        .map(|value| value.chars().count());
    match length {
        Some(length) => length >= min && length <= max,
        None => false,
    }
}

fn is_email(payload: &Value, field: &str) -> bool {
    lookup(payload, field)
        .and_then(|value| value.as_str())
        .map(|value| {
            let value = value.trim();
            let at = value.find('@');
            let dot = value.rfind('.');
            at.is_some() && dot > at
        })
        .unwrap_or(false)
}

fn is_numeric(payload: &Value, field: &str) -> bool {
    lookup(payload, field)
        .map(|value| match value {
            Value::Number(number) => number.is_f64() || number.is_i64() || number.is_u64(),
            Value::String(text) => text.parse::<f64>().is_ok(),
            _ => false,
        })
        .unwrap_or(false)
}

fn is_between(payload: &Value, field: &str, min: f64, max: f64) -> bool {
    lookup(payload, field)
        .and_then(|value| match value {
            Value::Number(number) => number.as_f64(),
            Value::String(text) => text.parse::<f64>().ok(),
            _ => None,
        })
        .map(|value| value >= min && value <= max)
        .unwrap_or(false)
}

fn lookup<'a>(payload: &'a Value, field: &str) -> Option<&'a Value> {
    let mut current = payload;
    for part in field.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            Value::Array(items) => {
                let index = part.parse::<usize>().ok()?;
                current = items.get(index)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn parse_range(range: &[Value]) -> Option<(f64, f64)> {
    if range.len() != 2 {
        return None;
    }
    let min = range[0].as_f64().unwrap_or(f64::MIN);
    let max = range[1].as_f64().unwrap_or(f64::MAX);
    Some((min, max))
}

fn matches_regex(payload: &Value, field: &str, pattern: &str) -> bool {
    if let Ok(regex) = regex::Regex::new(pattern) {
        return lookup(payload, field)
            .and_then(|value| value.as_str())
            .map(|value| regex.is_match(value))
            .unwrap_or(false);
    }
    true
}

fn is_in_list(payload: &Value, field: &str, allowed: &[Value]) -> bool {
    lookup(payload, field).map_or(false, |value| match value {
        Value::String(text) => allowed
            .iter()
            .filter_map(|value| value.as_str())
            .any(|candidate| candidate == text),
        Value::Number(num) => num.as_f64().map_or(false, |target| {
            allowed
                .iter()
                .filter_map(|value| value.as_f64())
                .any(|candidate| (candidate - target).abs() < f64::EPSILON)
        }),
        Value::Bool(flag) => allowed
            .iter()
            .filter_map(|value| value.as_bool())
            .any(|candidate| candidate == *flag),
        _ => false,
    })
}

fn numeric_min(payload: &Value, field: &str, min: f64) -> bool {
    lookup(payload, field)
        .and_then(|value| numeric_value(value))
        .map(|value| value >= min)
        .unwrap_or(false)
}

fn numeric_max(payload: &Value, field: &str, max: f64) -> bool {
    lookup(payload, field)
        .and_then(|value| numeric_value(value))
        .map(|value| value <= max)
        .unwrap_or(false)
}

fn numeric_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
}

fn violation(field: &str, message: &str, code: &str) -> ValidationViolation {
    ValidationViolation {
        field: field.to_string(),
        message: message.to_string(),
        code: Some(code.to_string()),
    }
}

fn apply_custom(
    rule: &str,
    payload: &Value,
    field: &str,
    map: &serde_json::Map<String, Value>,
) -> bool {
    match rule.trim() {
        "non_empty_array" => lookup(payload, field)
            .and_then(|value| value.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false),
        "unique_array" => lookup(payload, field)
            .and_then(|value| value.as_array())
            .map(|items| {
                let mut seen = std::collections::HashSet::new();
                items.iter().all(|item| seen.insert(item.clone()))
            })
            .unwrap_or(false),
        "accepted" => lookup(payload, field)
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        "matches_field" => map
            .get("other")
            .and_then(|value| value.as_str())
            .and_then(|other| lookup(payload, other))
            .map(|other| lookup(payload, field) == Some(other))
            .unwrap_or(false),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn validates_required_and_email_rules() {
        let service = SimpleValidationService::default();
        let rules = ValidationRules {
            rules: serde_json::json!({
                "required": ["email"],
                "fields": {
                    "email": {"email": true}
                }
            }),
        };

        let payload = serde_json::json!({"email": "invalid"});
        let report = service.validate(payload, rules).await.unwrap();
        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|err| err.code.as_deref() == Some("email")));
    }

    #[tokio::test]
    async fn validates_numeric_range_and_regex() {
        let service = SimpleValidationService::default();
        let rules = ValidationRules {
            rules: serde_json::json!({
                "fields": {
                    "age": {"numeric": true, "between": [18, 65]},
                    "slug": {"regex": "^[a-z0-9_-]+$"}
                }
            }),
        };

        let payload = serde_json::json!({"age": 70, "slug": "Bad Slug"});
        let report = service.validate(payload, rules).await.unwrap();
        assert_eq!(report.errors.len(), 2);
        assert!(report
            .errors
            .iter()
            .any(|err| err.code.as_deref() == Some("between")));
        assert!(report
            .errors
            .iter()
            .any(|err| err.code.as_deref() == Some("regex")));
    }

    #[tokio::test]
    async fn supports_custom_predicates() {
        let service = SimpleValidationService::default();
        let rules = ValidationRules {
            rules: serde_json::json!({
                "fields": {
                    "tags": {"custom_predicate": "non_empty_array"},
                    "confirm": {"custom_predicate": "matches_field", "other": "password"}
                }
            }),
        };

        let payload = serde_json::json!({
            "tags": [],
            "password": "secret",
            "confirm": "different"
        });

        let report = service.validate(payload, rules).await.unwrap();
        assert_eq!(report.errors.len(), 2);
        assert!(report
            .errors
            .iter()
            .any(|err| err.code.as_deref() == Some("non_empty_array")));
        assert!(report
            .errors
            .iter()
            .any(|err| err.code.as_deref() == Some("matches_field")));
    }
}
