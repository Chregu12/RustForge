use crate::error::{PromptError, PromptResult};
use dialoguer::{Confirm, Input, MultiSelect, Password, Select};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::Helper;

/// Options for configuring prompts
#[derive(Debug, Clone)]
pub struct PromptOptions {
    /// Whether the prompt allows empty input
    pub allow_empty: bool,
    /// Default value if user provides no input
    pub default: Option<String>,
    /// Validation function
    pub validator: Option<fn(&str) -> bool>,
}

impl Default for PromptOptions {
    fn default() -> Self {
        Self {
            allow_empty: false,
            default: None,
            validator: None,
        }
    }
}

/// Represents a selectable option in choice/multi-select prompts
#[derive(Debug, Clone)]
pub struct SelectOption {
    pub label: String,
    pub description: Option<String>,
    pub value: String,
}

impl SelectOption {
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
            description: Some(description.into()),
        }
    }

    pub fn simple(label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            value: label.clone(),
            label,
            description: None,
        }
    }

    pub fn with_value(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            description: None,
        }
    }
}

impl std::fmt::Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(desc) = &self.description {
            write!(f, "{} - {}", self.label, desc)
        } else {
            write!(f, "{}", self.label)
        }
    }
}

/// Ask for text input with a prompt message
///
/// # Example
/// ```no_run
/// use foundry_interactive::ask;
///
/// let name = ask("What is your name?").unwrap();
/// println!("Hello, {}!", name);
/// ```
pub fn ask(prompt: &str) -> PromptResult<String> {
    Input::<String>::new()
        .with_prompt(prompt)
        .interact_text()
        .map_err(|_| PromptError::Cancelled)
}

/// Ask for text input with a default value
///
/// # Example
/// ```no_run
/// use foundry_interactive::ask_with_default;
///
/// let port = ask_with_default("Port", "8080").unwrap();
/// ```
pub fn ask_with_default(prompt: &str, default: &str) -> PromptResult<String> {
    Input::<String>::new()
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()
        .map_err(|_| PromptError::Cancelled)
}

/// Present a single-choice selection prompt
///
/// # Example
/// ```no_run
/// use foundry_interactive::{choice, SelectOption};
///
/// let options = vec![
///     SelectOption::new("SQLite", "Lightweight database"),
///     SelectOption::new("PostgreSQL", "Production-ready database"),
/// ];
///
/// let selected = choice("Choose database", &options, 0).unwrap();
/// println!("Selected: {}", selected);
/// ```
pub fn choice(prompt: &str, options: &[SelectOption], default: usize) -> PromptResult<String> {
    if options.is_empty() {
        return Err(PromptError::InvalidInput("No options provided".to_string()));
    }

    let default = if default >= options.len() {
        0
    } else {
        default
    };

    let items: Vec<String> = options.iter().map(|opt| opt.to_string()).collect();

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(default)
        .interact()
        .map_err(|_| PromptError::Cancelled)?;

    Ok(options[selection].value.clone())
}

/// Ask for confirmation (yes/no)
///
/// # Example
/// ```no_run
/// use foundry_interactive::confirm;
///
/// if confirm("Overwrite existing file?", false).unwrap() {
///     println!("File will be overwritten");
/// }
/// ```
pub fn confirm(prompt: &str, default: bool) -> PromptResult<bool> {
    Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(|_| PromptError::Cancelled)
}

/// Ask for password input (hidden)
///
/// # Example
/// ```no_run
/// use foundry_interactive::password;
///
/// let pwd = password("Enter password").unwrap();
/// ```
pub fn password(prompt: &str) -> PromptResult<String> {
    Password::new()
        .with_prompt(prompt)
        .interact()
        .map_err(|_| PromptError::Cancelled)
}

/// Present a multi-select prompt (multiple choices)
///
/// # Example
/// ```no_run
/// use foundry_interactive::{multi_select, SelectOption};
///
/// let options = vec![
///     SelectOption::simple("Feature A"),
///     SelectOption::simple("Feature B"),
///     SelectOption::simple("Feature C"),
/// ];
///
/// let selected = multi_select("Select features", &options, &[0]).unwrap();
/// ```
pub fn multi_select(
    prompt: &str,
    options: &[SelectOption],
    defaults: &[usize],
) -> PromptResult<Vec<String>> {
    if options.is_empty() {
        return Err(PromptError::InvalidInput("No options provided".to_string()));
    }

    let items: Vec<String> = options.iter().map(|opt| opt.to_string()).collect();

    let defaults_bool: Vec<bool> = (0..options.len())
        .map(|i| defaults.contains(&i))
        .collect();

    let selections = MultiSelect::new()
        .with_prompt(prompt)
        .items(&items)
        .defaults(&defaults_bool)
        .interact()
        .map_err(|_| PromptError::Cancelled)?;

    Ok(selections
        .into_iter()
        .map(|idx| options[idx].value.clone())
        .collect())
}

/// Helper struct for autocomplete functionality
struct AutocompleteHelper {
    suggestions: Vec<String>,
}

impl Helper for AutocompleteHelper {}

impl Completer for AutocompleteHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);

        let current_word = &line[start..pos];

        let matches: Vec<Pair> = self
            .suggestions
            .iter()
            .filter(|s| s.starts_with(current_word))
            .map(|s| Pair {
                display: s.clone(),
                replacement: s.clone(),
            })
            .collect();

        Ok((start, matches))
    }
}

impl Hinter for AutocompleteHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if pos < line.len() {
            return None;
        }

        self.suggestions
            .iter()
            .find(|s| s.starts_with(line))
            .map(|s| s[line.len()..].to_string())
    }
}

impl Highlighter for AutocompleteHelper {}
impl Validator for AutocompleteHelper {}

/// Ask for text input with autocomplete suggestions
///
/// # Example
/// ```no_run
/// use foundry_interactive::autocomplete;
///
/// let suggestions = vec!["User".to_string(), "UserProfile".to_string(), "Post".to_string()];
/// let model = autocomplete("Model name", &suggestions, None).unwrap();
/// ```
pub fn autocomplete(
    prompt: &str,
    suggestions: &[String],
    default: Option<&str>,
) -> PromptResult<String> {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let helper = AutocompleteHelper {
        suggestions: suggestions.to_vec(),
    };

    let mut rl = Editor::with_config(config).map_err(|e| {
        PromptError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string(),
        ))
    })?;

    rl.set_helper(Some(helper));

    let prompt_text = if let Some(def) = default {
        format!("{} [{}]: ", prompt, def)
    } else {
        format!("{}: ", prompt)
    };

    match rl.readline(&prompt_text) {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if let Some(def) = default {
                    Ok(def.to_string())
                } else {
                    Ok(line)
                }
            } else {
                Ok(trimmed.to_string())
            }
        }
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
            Err(PromptError::Cancelled)
        }
        Err(err) => Err(PromptError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_option_display() {
        let opt1 = SelectOption::simple("Test");
        assert_eq!(format!("{}", opt1), "Test");

        let opt2 = SelectOption::new("Test", "Description");
        assert_eq!(format!("{}", opt2), "Test - Description");
    }

    #[test]
    fn test_select_option_with_value() {
        let opt = SelectOption::with_value("Display", "value");
        assert_eq!(opt.label, "Display");
        assert_eq!(opt.value, "value");
    }
}
