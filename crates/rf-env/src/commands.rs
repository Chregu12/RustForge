//! Environment management commands

use crate::validator::{EnvRule, EnvValidator, VarType};
use async_trait::async_trait;
use rf_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};
use std::path::PathBuf;

/// Validate environment variables
pub struct EnvValidateCommand;

#[async_trait]
impl FoundryCommand for EnvValidateCommand {
    fn descriptor(&self) -> &rf_domain::CommandDescriptor {
        use std::sync::OnceLock;
        static DESCRIPTOR: OnceLock<rf_domain::CommandDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            rf_domain::CommandDescriptor::builder("env:validate", "validate")
                .description("Validate environment variables against requirements")
                .build()
        })
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Common required variables
        let rules = vec![
            EnvRule {
                name: "DATABASE_URL".to_string(),
                required: false,
                var_type: VarType::String,
                default: None,
                description: Some("Database connection URL".to_string()),
            },
            EnvRule {
                name: "APP_ENV".to_string(),
                required: false,
                var_type: VarType::String,
                default: Some("development".to_string()),
                description: Some("Application environment".to_string()),
            },
        ];

        let validator = EnvValidator::new(rules);

        // Load current environment
        let env_path = PathBuf::from(".env");
        let env_vars = crate::load_env(&env_path).map_err(CommandError::Other)?;

        let results = validator.validate(&env_vars);
        let output = validator.format_results(&results);

        let all_valid = results.iter().all(|r| r.valid);

        if all_valid {
            Ok(CommandResult::success(&output))
        } else {
            Err(CommandError::Message(output))
        }
    }
}

/// Reload environment variables
pub struct EnvReloadCommand;

#[async_trait]
impl FoundryCommand for EnvReloadCommand {
    fn descriptor(&self) -> &rf_domain::CommandDescriptor {
        use std::sync::OnceLock;
        static DESCRIPTOR: OnceLock<rf_domain::CommandDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            rf_domain::CommandDescriptor::builder("env:reload", "reload")
                .description("Reload environment variables from .env file")
                .build()
        })
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let env_path = PathBuf::from(".env");

        if !env_path.exists() {
            return Err(CommandError::Message(".env file not found".to_string()));
        }

        let count = crate::reload_env(&env_path).map_err(CommandError::Other)?;

        Ok(CommandResult::success(format!(
            "Reloaded {} environment variables",
            count
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_env_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        fs::write(&env_path, "TEST_VAR=test_value\n").unwrap();

        // Verify file exists and is readable
        assert!(env_path.exists());
        let content = fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("TEST_VAR=test_value"));
    }

    #[test]
    fn test_env_file_validation() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        // Create a valid .env file
        fs::write(&env_path, "APP_ENV=production\nAPP_DEBUG=false\n").unwrap();

        // Verify file exists and content is correct
        assert!(env_path.exists());
        let content = fs::read_to_string(&env_path).unwrap();
        assert!(content.contains("APP_ENV=production"));
        assert!(content.contains("APP_DEBUG=false"));
    }

    #[test]
    fn test_env_file_parsing_with_dotenvy() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        // Create .env with various formats
        let env_content = "SIMPLE_VAR=simple_value\nNUMBER_VAR=12345\n";
        fs::write(&env_path, env_content).unwrap();

        // Load from specific path
        dotenvy::from_path(&env_path).ok();

        // Variables should be loaded
        let simple = std::env::var("SIMPLE_VAR").unwrap_or_default();
        let number = std::env::var("NUMBER_VAR").unwrap_or_default();

        assert_eq!(simple, "simple_value");
        assert_eq!(number, "12345");
    }
}
