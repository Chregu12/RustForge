//! Form validation rules and validators

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum ValidationRule {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Email,
    Url,
    Numeric,
    Integer,
    Min(f64),
    Max(f64),
    Between { min: f64, max: f64 },
    In { values: Vec<String> },
    Custom { message: String },
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationRule {
    pub fn validate(&self, field: &str, value: Option<&str>) -> Result<(), ValidationError> {
        match self {
            ValidationRule::Required => {
                if value.is_none() || value.unwrap().trim().is_empty() {
                    return Err(ValidationError {
                        field: field.to_string(),
                        message: format!("The {} field is required", field),
                    });
                }
            }
            ValidationRule::MinLength(min) => {
                if let Some(v) = value {
                    if v.len() < *min {
                        return Err(ValidationError {
                            field: field.to_string(),
                            message: format!(
                                "The {} field must be at least {} characters",
                                field, min
                            ),
                        });
                    }
                }
            }
            ValidationRule::MaxLength(max) => {
                if let Some(v) = value {
                    if v.len() > *max {
                        return Err(ValidationError {
                            field: field.to_string(),
                            message: format!(
                                "The {} field must not exceed {} characters",
                                field, max
                            ),
                        });
                    }
                }
            }
            ValidationRule::Pattern(pattern) => {
                if let Some(v) = value {
                    let re = Regex::new(pattern).map_err(|_| ValidationError {
                        field: field.to_string(),
                        message: "Invalid regex pattern".to_string(),
                    })?;
                    if !re.is_match(v) {
                        return Err(ValidationError {
                            field: field.to_string(),
                            message: format!("The {} field format is invalid", field),
                        });
                    }
                }
            }
            ValidationRule::Email => {
                if let Some(v) = value {
                    let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
                    if !email_regex.is_match(v) {
                        return Err(ValidationError {
                            field: field.to_string(),
                            message: format!("The {} field must be a valid email", field),
                        });
                    }
                }
            }
            ValidationRule::Numeric => {
                if let Some(v) = value {
                    if v.parse::<f64>().is_err() {
                        return Err(ValidationError {
                            field: field.to_string(),
                            message: format!("The {} field must be a number", field),
                        });
                    }
                }
            }
            ValidationRule::Min(min) => {
                if let Some(v) = value {
                    if let Ok(num) = v.parse::<f64>() {
                        if num < *min {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("The {} field must be at least {}", field, min),
                            });
                        }
                    }
                }
            }
            ValidationRule::Max(max) => {
                if let Some(v) = value {
                    if let Ok(num) = v.parse::<f64>() {
                        if num > *max {
                            return Err(ValidationError {
                                field: field.to_string(),
                                message: format!("The {} field must not exceed {}", field, max),
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Form validator
pub struct Validator;

impl Validator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
