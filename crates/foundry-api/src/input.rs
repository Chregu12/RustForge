/// Advanced input handling for command arguments and options
///
/// This module provides utilities for parsing and validating command input,
/// supporting option arrays, defaults, and validation rules.
///
/// # Example
///
/// ```rust,no_run
/// use foundry_api::input::{InputParser, InputValidator};
///
/// let parser = InputParser::from_args(&["--tags", "admin", "--tags", "user", "name"]);
///
/// // Get single value
/// if let Some(name) = parser.argument(0) {
///     println!("Name: {}", name);
/// }
///
/// // Get array values
/// let tags = parser.option_array("tags");
/// println!("Tags: {:?}", tags);
/// ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Parsed input arguments and options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputParser {
    arguments: Vec<String>,
    options: HashMap<String, Vec<String>>,
    flags: Vec<String>,
}

impl InputParser {
    /// Create a new input parser from command arguments
    ///
    /// Distinguishes between:
    /// - **Arguments**: Non-flag values at the beginning
    /// - **Options**: `--name=value` or `--name value`
    /// - **Flags**: `--flag` or `-f` with no value
    /// - **Option Arrays**: Multiple `--name value` entries
    pub fn from_args(args: &[String]) -> Self {
        let mut arguments = Vec::new();
        let mut options: HashMap<String, Vec<String>> = HashMap::new();
        let mut flags = Vec::new();

        let mut i = 0;
        let mut seen_option = false;

        while i < args.len() {
            let arg = &args[i];

            if arg.starts_with("--") {
                seen_option = true;
                let rest = &arg[2..];

                if let Some(eq_pos) = rest.find('=') {
                    // --name=value
                    let name = rest[..eq_pos].to_string();
                    let value = rest[eq_pos + 1..].to_string();
                    options.entry(name).or_insert_with(Vec::new).push(value);
                } else {
                    // --name or --name value
                    let name = rest.to_string();

                    // Check if next arg is a value (doesn't start with -)
                    if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        let value = args[i + 1].clone();
                        options.entry(name).or_insert_with(Vec::new).push(value);
                        i += 1;
                    } else {
                        // It's a flag
                        flags.push(name);
                    }
                }
            } else if arg.starts_with('-') && arg.len() > 1 && arg != "-" {
                seen_option = true;
                let rest = &arg[1..];

                // Handle short flags: -f, -abc (multiple flags), -f value
                for ch in rest.chars() {
                    let flag = ch.to_string();

                    // Check if next arg could be a value for this flag
                    if ch == rest.chars().last().unwrap() && i + 1 < args.len()
                        && !args[i + 1].starts_with('-')
                    {
                        let value = args[i + 1].clone();
                        options.entry(flag).or_insert_with(Vec::new).push(value);
                        i += 1;
                    } else {
                        flags.push(flag);
                    }
                }
            } else if !seen_option {
                // Positional argument (before any options)
                arguments.push(arg.clone());
            }

            i += 1;
        }

        Self {
            arguments,
            options,
            flags,
        }
    }

    /// Get a positional argument by index
    pub fn argument(&self, index: usize) -> Option<String> {
        self.arguments.get(index).cloned()
    }

    /// Get all positional arguments
    pub fn arguments(&self) -> Vec<String> {
        self.arguments.clone()
    }

    /// Get the first positional argument
    pub fn first_argument(&self) -> Option<String> {
        self.arguments.first().cloned()
    }

    /// Get a single option value
    pub fn option(&self, name: &str) -> Option<String> {
        self.options.get(name).and_then(|mut values| values.first().cloned())
    }

    /// Get all values for an option (supports arrays)
    pub fn option_array(&self, name: &str) -> Vec<String> {
        self.options
            .get(name)
            .map(|values| values.clone())
            .unwrap_or_default()
    }

    /// Check if a flag is present
    pub fn has_flag(&self, name: &str) -> bool {
        self.flags.contains(&name.to_string())
    }

    /// Get option value or default
    pub fn option_with_default(&self, name: &str, default: &str) -> String {
        self.option(name).unwrap_or_else(|| default.to_string())
    }

    /// Get the number of positional arguments
    pub fn argument_count(&self) -> usize {
        self.arguments.len()
    }

    /// Get all available option names
    pub fn option_names(&self) -> Vec<String> {
        self.options.keys().cloned().collect()
    }

    /// Get all flags
    pub fn flags(&self) -> Vec<String> {
        self.flags.clone()
    }
}

/// Input validation rules
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationRule {
    pub field: String,
    pub rules: Vec<Rule>,
}

/// Individual validation rule
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Rule {
    /// Value must be present (non-empty)
    Required,
    /// String length must be at least N
    MinLength(usize),
    /// String length must be at most N
    MaxLength(usize),
    /// Value must match regex pattern
    Pattern(String),
    /// Value must be one of the allowed values
    OneOf(Vec<String>),
    /// Custom validation function
    Custom(String),
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rule::Required => write!(f, "This field is required"),
            Rule::MinLength(n) => write!(f, "Minimum length is {}", n),
            Rule::MaxLength(n) => write!(f, "Maximum length is {}", n),
            Rule::Pattern(_) => write!(f, "Invalid format"),
            Rule::OneOf(values) => write!(f, "Must be one of: {}", values.join(", ")),
            Rule::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

/// Validation violation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationViolation {
    pub field: String,
    pub rule: String,
    pub message: String,
}

/// Input validator
pub struct InputValidator {
    rules: Vec<ValidationRule>,
}

impl InputValidator {
    /// Create a new input validator
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a validation rule
    pub fn rule(mut self, field: impl Into<String>, rules: Vec<Rule>) -> Self {
        self.rules.push(ValidationRule {
            field: field.into(),
            rules,
        });
        self
    }

    /// Add a required field rule
    pub fn required(self, field: impl Into<String>) -> Self {
        self.rule(field, vec![Rule::Required])
    }

    /// Add a string length rule
    pub fn string_length(
        self,
        field: impl Into<String>,
        min: usize,
        max: usize,
    ) -> Self {
        self.rule(
            field,
            vec![Rule::MinLength(min), Rule::MaxLength(max)],
        )
    }

    /// Add a pattern rule
    pub fn pattern(self, field: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.rule(field, vec![Rule::Pattern(pattern.into())])
    }

    /// Validate input
    pub fn validate(&self, parser: &InputParser) -> Result<(), Vec<ValidationViolation>> {
        let mut violations = Vec::new();

        for rule_set in &self.rules {
            let value = parser.option(&rule_set.field);

            for rule in &rule_set.rules {
                if let Some(violation) = self.check_rule(&rule_set.field, &value, rule) {
                    violations.push(violation);
                }
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    fn check_rule(
        &self,
        field: &str,
        value: &Option<String>,
        rule: &Rule,
    ) -> Option<ValidationViolation> {
        match rule {
            Rule::Required => {
                if value.is_none() || value.as_ref().unwrap().is_empty() {
                    Some(ValidationViolation {
                        field: field.to_string(),
                        rule: "required".to_string(),
                        message: "This field is required".to_string(),
                    })
                } else {
                    None
                }
            }
            Rule::MinLength(min) => {
                if let Some(v) = value {
                    if v.len() < *min {
                        return Some(ValidationViolation {
                            field: field.to_string(),
                            rule: "min_length".to_string(),
                            message: format!("Minimum length is {}", min),
                        });
                    }
                }
                None
            }
            Rule::MaxLength(max) => {
                if let Some(v) = value {
                    if v.len() > *max {
                        return Some(ValidationViolation {
                            field: field.to_string(),
                            rule: "max_length".to_string(),
                            message: format!("Maximum length is {}", max),
                        });
                    }
                }
                None
            }
            Rule::OneOf(allowed) => {
                if let Some(v) = value {
                    if !allowed.contains(v) {
                        return Some(ValidationViolation {
                            field: field.to_string(),
                            rule: "one_of".to_string(),
                            message: format!("Must be one of: {}", allowed.join(", ")),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_positional_args() {
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let parser = InputParser::from_args(&args);
        assert_eq!(parser.argument(0), Some("arg1".to_string()));
        assert_eq!(parser.argument(1), Some("arg2".to_string()));
        assert_eq!(parser.argument_count(), 2);
    }

    #[test]
    fn test_parse_single_option() {
        let args = vec!["--name=John".to_string()];
        let parser = InputParser::from_args(&args);
        assert_eq!(parser.option("name"), Some("John".to_string()));
    }

    #[test]
    fn test_parse_option_with_separate_value() {
        let args = vec!["--name".to_string(), "John".to_string()];
        let parser = InputParser::from_args(&args);
        assert_eq!(parser.option("name"), Some("John".to_string()));
    }

    #[test]
    fn test_parse_option_array() {
        let args = vec![
            "--tag".to_string(),
            "admin".to_string(),
            "--tag".to_string(),
            "user".to_string(),
        ];
        let parser = InputParser::from_args(&args);
        let tags = parser.option_array("tag");
        assert_eq!(tags, vec!["admin".to_string(), "user".to_string()]);
    }

    #[test]
    fn test_parse_flags() {
        let args = vec!["--verbose".to_string(), "--force".to_string()];
        let parser = InputParser::from_args(&args);
        assert!(parser.has_flag("verbose"));
        assert!(parser.has_flag("force"));
    }

    #[test]
    fn test_short_flags() {
        let args = vec!["-v".to_string(), "-f".to_string()];
        let parser = InputParser::from_args(&args);
        assert!(parser.has_flag("v"));
        assert!(parser.has_flag("f"));
    }

    #[test]
    fn test_mixed_arguments_and_options() {
        let args = vec![
            "create".to_string(),
            "--name=John".to_string(),
            "--age=30".to_string(),
        ];
        let parser = InputParser::from_args(&args);
        assert_eq!(parser.first_argument(), Some("create".to_string()));
        assert_eq!(parser.option("name"), Some("John".to_string()));
        assert_eq!(parser.option("age"), Some("30".to_string()));
    }

    #[test]
    fn test_validation_required() {
        let validator = InputValidator::new().required("name");
        let args = vec![];
        let parser = InputParser::from_args(&args);
        let result = validator.validate(&parser);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_success() {
        let validator = InputValidator::new().required("name");
        let args = vec!["--name".to_string(), "John".to_string()];
        let parser = InputParser::from_args(&args);
        let result = validator.validate(&parser);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_length() {
        let validator = InputValidator::new().string_length("name", 2, 10);

        // Too short
        let args = vec!["--name".to_string(), "a".to_string()];
        let parser = InputParser::from_args(&args);
        assert!(validator.validate(&parser).is_err());

        // Valid
        let args = vec!["--name".to_string(), "john".to_string()];
        let parser = InputParser::from_args(&args);
        assert!(validator.validate(&parser).is_ok());

        // Too long
        let args = vec!["--name".to_string(), "verylongname".to_string()];
        let parser = InputParser::from_args(&args);
        assert!(validator.validate(&parser).is_err());
    }
}
