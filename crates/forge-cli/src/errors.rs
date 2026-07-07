#![allow(dead_code)] // utility helpers reserved for CLI commands, not all consumed yet
//! Enhanced error handling and reporting
//!
//! This module provides beautiful, helpful error messages with suggestions,
//! error codes, and links to documentation.

use colored::*;
use std::fmt;

/// Error codes for different types of CLI errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // File operations (1000-1999)
    FileNotFound = 1001,
    FileAlreadyExists = 1002,
    FilePermissionDenied = 1003,
    DirectoryNotFound = 1004,

    // Project errors (2000-2999)
    NotInForgeProject = 2001,
    InvalidProjectStructure = 2002,
    MissingDependency = 2003,

    // Migration errors (3000-3999)
    MigrationFailed = 3001,
    MigrationSyntaxError = 3002,
    MigrationNotFound = 3003,
    DatabaseConnectionFailed = 3004,

    // Generation errors (4000-4999)
    InvalidModelName = 4001,
    InvalidControllerName = 4002,
    TemplateNotFound = 4003,
    GenerationFailed = 4004,

    // Validation errors (5000-5999)
    InvalidInput = 5001,
    ValidationFailed = 5002,

    // General errors
    Unknown = 9999,
}

impl ErrorCode {
    pub fn as_str(&self) -> &str {
        match self {
            ErrorCode::FileNotFound => "RF_FILE_001",
            ErrorCode::FileAlreadyExists => "RF_FILE_002",
            ErrorCode::FilePermissionDenied => "RF_FILE_003",
            ErrorCode::DirectoryNotFound => "RF_FILE_004",
            ErrorCode::NotInForgeProject => "RF_PROJ_001",
            ErrorCode::InvalidProjectStructure => "RF_PROJ_002",
            ErrorCode::MissingDependency => "RF_PROJ_003",
            ErrorCode::MigrationFailed => "RF_MIG_001",
            ErrorCode::MigrationSyntaxError => "RF_MIG_002",
            ErrorCode::MigrationNotFound => "RF_MIG_003",
            ErrorCode::DatabaseConnectionFailed => "RF_MIG_004",
            ErrorCode::InvalidModelName => "RF_GEN_001",
            ErrorCode::InvalidControllerName => "RF_GEN_002",
            ErrorCode::TemplateNotFound => "RF_GEN_003",
            ErrorCode::GenerationFailed => "RF_GEN_004",
            ErrorCode::InvalidInput => "RF_VAL_001",
            ErrorCode::ValidationFailed => "RF_VAL_002",
            ErrorCode::Unknown => "RF_UNKNOWN",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A CLI error with rich formatting and suggestions
#[derive(Debug)]
pub struct CliError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub suggestion: Option<String>,
    pub docs_link: Option<String>,
    pub file_location: Option<FileLocation>,
}

#[derive(Debug, Clone)]
pub struct FileLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub context: Option<Vec<String>>,
}

impl CliError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            suggestion: None,
            docs_link: None,
            file_location: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_docs(mut self, link: impl Into<String>) -> Self {
        self.docs_link = Some(link.into());
        self
    }

    pub fn with_location(mut self, location: FileLocation) -> Self {
        self.file_location = Some(location);
        self
    }

    /// Display the error with rich formatting
    pub fn display(&self) {
        println!();
        println!("{} {}", "✗".red().bold(), "Error:".red().bold());
        println!();
        println!("  {} ({})", self.message, self.code.to_string().yellow());
        println!();

        if let Some(details) = &self.details {
            println!("{}", details);
            println!();
        }

        if let Some(location) = &self.file_location {
            self.display_file_location(location);
        }

        if let Some(suggestion) = &self.suggestion {
            println!("{}", "Did you mean?".cyan().bold());
            println!("  {}", suggestion.green());
            println!();
        }

        if let Some(docs) = &self.docs_link {
            println!("{} {}", "See:".blue().bold(), docs.cyan().underline());
            println!();
        }
    }

    fn display_file_location(&self, location: &FileLocation) {
        println!(
            "  {}:{}:{}",
            location.file.cyan(),
            location.line,
            location.column
        );
        println!();

        if let Some(context) = &location.context {
            for (i, line) in context.iter().enumerate() {
                let line_num = location.line - 1 + i;
                if line_num == location.line {
                    println!("  {} │ {}", line_num.to_string().yellow().bold(), line);
                    println!(
                        "  {} │ {}{}",
                        " ".repeat(line_num.to_string().len()),
                        " ".repeat(location.column - 1),
                        "^".repeat(3).red().bold()
                    );
                } else {
                    println!("  {} │ {}", line_num.to_string().dimmed(), line.dimmed());
                }
            }
            println!();
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for CliError {}

/// Helper functions for common errors

pub fn file_not_found(path: &str) -> CliError {
    CliError::new(ErrorCode::FileNotFound, format!("File not found: {}", path))
        .with_suggestion(format!("Make sure the file exists at: {}", path))
        .with_docs("https://docs.rustforge.dev/cli/errors#file-not-found")
}

pub fn file_already_exists(path: &str) -> CliError {
    CliError::new(
        ErrorCode::FileAlreadyExists,
        format!("File already exists: {}", path),
    )
    .with_suggestion("Use a different name or remove the existing file first")
    .with_docs("https://docs.rustforge.dev/cli/errors#file-exists")
}

pub fn not_in_forge_project() -> CliError {
    CliError::new(
        ErrorCode::NotInForgeProject,
        "Not in a RustForge project directory",
    )
    .with_details("This command must be run from the root of a RustForge project.")
    .with_suggestion("Run 'forge new <name>' to create a new project, or navigate to an existing project directory")
    .with_docs("https://docs.rustforge.dev/getting-started")
}

pub fn migration_syntax_error(file: &str, line: usize, column: usize, error: &str) -> CliError {
    let location = FileLocation {
        file: file.to_string(),
        line,
        column,
        context: None,
    };

    CliError::new(
        ErrorCode::MigrationSyntaxError,
        "Migration file has syntax errors",
    )
    .with_details(error)
    .with_location(location)
    .with_docs("https://docs.rustforge.dev/migrations")
}

pub fn database_connection_failed(details: &str) -> CliError {
    CliError::new(
        ErrorCode::DatabaseConnectionFailed,
        "Failed to connect to database",
    )
    .with_details(details)
    .with_suggestion("Check your database configuration in .env file")
    .with_docs("https://docs.rustforge.dev/database/configuration")
}

pub fn invalid_model_name(name: &str) -> CliError {
    CliError::new(
        ErrorCode::InvalidModelName,
        format!("Invalid model name: {}", name),
    )
    .with_details("Model names should be PascalCase and start with an uppercase letter.")
    .with_suggestion(format!(
        "Try: {}",
        name.chars()
            .enumerate()
            .map(|(i, c)| if i == 0 {
                c.to_uppercase().to_string()
            } else {
                c.to_string()
            })
            .collect::<String>()
    ))
    .with_docs("https://docs.rustforge.dev/models#naming-conventions")
}

pub fn template_not_found(template: &str) -> CliError {
    CliError::new(
        ErrorCode::TemplateNotFound,
        format!("Template not found: {}", template),
    )
    .with_suggestion("This might be a bug. Please report it on GitHub.")
    .with_docs("https://github.com/your-org/rustforge/issues")
}

/// Print an error and exit
pub fn fatal_error(error: &CliError) -> ! {
    error.display();
    std::process::exit(1);
}

/// Print a formatted error message
pub fn print_error(message: &str) {
    println!("{} {}", "✗".red().bold(), message.red());
}

/// Print a formatted warning message
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message.yellow());
}

/// Print a formatted success message
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message.green());
}

/// Print a formatted info message
pub fn print_info(message: &str) {
    println!("{} {}", "→".blue().bold(), message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::FileNotFound.as_str(), "RF_FILE_001");
        assert_eq!(ErrorCode::MigrationFailed.as_str(), "RF_MIG_001");
        assert_eq!(ErrorCode::InvalidModelName.as_str(), "RF_GEN_001");
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::FileNotFound.to_string(), "RF_FILE_001");
    }

    #[test]
    fn test_cli_error_new() {
        let error = CliError::new(ErrorCode::FileNotFound, "Test error");
        assert_eq!(error.code, ErrorCode::FileNotFound);
        assert_eq!(error.message, "Test error");
        assert!(error.details.is_none());
        assert!(error.suggestion.is_none());
    }

    #[test]
    fn test_cli_error_with_details() {
        let error = CliError::new(ErrorCode::FileNotFound, "Test error").with_details("More info");
        assert_eq!(error.details, Some("More info".to_string()));
    }

    #[test]
    fn test_cli_error_with_suggestion() {
        let error =
            CliError::new(ErrorCode::FileNotFound, "Test error").with_suggestion("Try this");
        assert_eq!(error.suggestion, Some("Try this".to_string()));
    }

    #[test]
    fn test_cli_error_with_docs() {
        let error =
            CliError::new(ErrorCode::FileNotFound, "Test error").with_docs("https://example.com");
        assert_eq!(error.docs_link, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_file_not_found_error() {
        let error = file_not_found("test.rs");
        assert_eq!(error.code, ErrorCode::FileNotFound);
        assert!(error.message.contains("test.rs"));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_file_already_exists_error() {
        let error = file_already_exists("test.rs");
        assert_eq!(error.code, ErrorCode::FileAlreadyExists);
        assert!(error.message.contains("test.rs"));
    }

    #[test]
    fn test_not_in_forge_project_error() {
        let error = not_in_forge_project();
        assert_eq!(error.code, ErrorCode::NotInForgeProject);
        assert!(error.details.is_some());
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_invalid_model_name_error() {
        let error = invalid_model_name("user");
        assert_eq!(error.code, ErrorCode::InvalidModelName);
        assert!(error.message.contains("user"));
        assert!(error.suggestion.is_some());
    }

    #[test]
    fn test_database_connection_failed_error() {
        let error = database_connection_failed("Connection refused");
        assert_eq!(error.code, ErrorCode::DatabaseConnectionFailed);
        assert!(error.details.is_some());
    }

    #[test]
    fn test_file_location() {
        let location = FileLocation {
            file: "test.rs".to_string(),
            line: 10,
            column: 5,
            context: None,
        };
        assert_eq!(location.file, "test.rs");
        assert_eq!(location.line, 10);
        assert_eq!(location.column, 5);
    }
}
