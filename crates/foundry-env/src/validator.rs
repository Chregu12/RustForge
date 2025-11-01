//! Environment variable validation

use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation rule for environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvRule {
    /// Variable name
    pub name: String,
    /// Whether variable is required
    pub required: bool,
    /// Expected type
    pub var_type: VarType,
    /// Default value if not set
    pub default: Option<String>,
    /// Description
    pub description: Option<String>,
}

/// Type of environment variable
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VarType {
    String,
    Integer,
    Boolean,
    Url,
    Path,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Variable name
    pub name: String,
    /// Whether validation passed
    pub valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Current value
    pub value: Option<String>,
}

impl ValidationResult {
    /// Create a passing result
    pub fn pass(name: String, value: Option<String>) -> Self {
        Self {
            name,
            valid: true,
            error: None,
            value,
        }
    }

    /// Create a failing result
    pub fn fail(name: String, error: String) -> Self {
        Self {
            name,
            valid: false,
            error: Some(error),
            value: None,
        }
    }
}

/// Environment validator
pub struct EnvValidator {
    rules: Vec<EnvRule>,
}

impl EnvValidator {
    /// Create a new validator
    pub fn new(rules: Vec<EnvRule>) -> Self {
        Self { rules }
    }

    /// Validate environment variables
    pub fn validate(&self, env: &HashMap<String, String>) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for rule in &self.rules {
            let result = self.validate_rule(rule, env);
            results.push(result);
        }

        results
    }

    /// Validate a single rule
    fn validate_rule(&self, rule: &EnvRule, env: &HashMap<String, String>) -> ValidationResult {
        match env.get(&rule.name) {
            Some(value) => {
                // Check type
                if self.check_type(value, &rule.var_type) {
                    ValidationResult::pass(rule.name.clone(), Some(value.clone()))
                } else {
                    ValidationResult::fail(
                        rule.name.clone(),
                        format!("Invalid type, expected {:?}", rule.var_type),
                    )
                }
            }
            None => {
                if rule.required {
                    ValidationResult::fail(rule.name.clone(), "Required but not set".to_string())
                } else {
                    ValidationResult::pass(rule.name.clone(), rule.default.clone())
                }
            }
        }
    }

    /// Check if value matches expected type
    fn check_type(&self, value: &str, var_type: &VarType) -> bool {
        match var_type {
            VarType::String => true,
            VarType::Integer => value.parse::<i64>().is_ok(),
            VarType::Boolean => matches!(value.to_lowercase().as_str(), "true" | "false" | "1" | "0"),
            VarType::Url => value.starts_with("http://") || value.starts_with("https://"),
            VarType::Path => !value.is_empty(),
        }
    }

    /// Format validation results
    pub fn format_results(&self, results: &[ValidationResult]) -> String {
        let mut output = String::new();
        output.push_str("\n");
        output.push_str(&"Environment Validation Results\n".bold().to_string());
        output.push_str(&"═".repeat(50));
        output.push('\n');

        for result in results {
            let status = if result.valid {
                "✓".green()
            } else {
                "✗".red()
            };

            let value_str = result
                .value
                .as_ref()
                .map(|v| format!(" = {}", v))
                .unwrap_or_default();

            output.push_str(&format!("  {} {}{}\n", status, result.name, value_str));

            if let Some(error) = &result.error {
                output.push_str(&format!("      {}\n", error.red()));
            }
        }

        output.push('\n');

        let passed = results.iter().filter(|r| r.valid).count();
        let total = results.len();

        output.push_str(&format!(
            "  {} / {} checks passed\n",
            passed.to_string().green(),
            total
        ));
        output.push('\n');

        output
    }

    /// Get all required variables that are missing
    pub fn get_missing_required(&self, env: &HashMap<String, String>) -> Vec<String> {
        self.rules
            .iter()
            .filter(|r| r.required && !env.contains_key(&r.name))
            .map(|r| r.name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let pass = ValidationResult::pass("TEST".to_string(), Some("value".to_string()));
        assert!(pass.valid);

        let fail = ValidationResult::fail("TEST".to_string(), "Error".to_string());
        assert!(!fail.valid);
    }

    #[test]
    fn test_validator_new() {
        let rules = vec![EnvRule {
            name: "TEST_VAR".to_string(),
            required: true,
            var_type: VarType::String,
            default: None,
            description: None,
        }];

        let validator = EnvValidator::new(rules);
        assert_eq!(validator.rules.len(), 1);
    }

    #[test]
    fn test_validate_required() {
        let rules = vec![EnvRule {
            name: "REQUIRED_VAR".to_string(),
            required: true,
            var_type: VarType::String,
            default: None,
            description: None,
        }];

        let validator = EnvValidator::new(rules);

        let mut env = HashMap::new();
        let results = validator.validate(&env);
        assert!(!results[0].valid);

        env.insert("REQUIRED_VAR".to_string(), "value".to_string());
        let results = validator.validate(&env);
        assert!(results[0].valid);
    }

    #[test]
    fn test_check_type_integer() {
        let validator = EnvValidator::new(vec![]);
        assert!(validator.check_type("123", &VarType::Integer));
        assert!(!validator.check_type("abc", &VarType::Integer));
    }

    #[test]
    fn test_check_type_boolean() {
        let validator = EnvValidator::new(vec![]);
        assert!(validator.check_type("true", &VarType::Boolean));
        assert!(validator.check_type("false", &VarType::Boolean));
        assert!(validator.check_type("1", &VarType::Boolean));
        assert!(!validator.check_type("yes", &VarType::Boolean));
    }

    #[test]
    fn test_get_missing_required() {
        let rules = vec![
            EnvRule {
                name: "VAR1".to_string(),
                required: true,
                var_type: VarType::String,
                default: None,
                description: None,
            },
            EnvRule {
                name: "VAR2".to_string(),
                required: false,
                var_type: VarType::String,
                default: None,
                description: None,
            },
        ];

        let validator = EnvValidator::new(rules);
        let env = HashMap::new();
        let missing = validator.get_missing_required(&env);

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "VAR1");
    }
}
