//! Environment management commands

use crate::validator::{EnvRule, EnvValidator, VarType};
use anyhow::Result;
use async_trait::async_trait;
use foundry_plugins::{CommandExecutor, CommandResult, ExecutionContext};
use std::path::PathBuf;

/// Validate environment variables
pub struct EnvValidateCommand;

#[async_trait]
impl CommandExecutor for EnvValidateCommand {
    fn name(&self) -> &'static str {
        "env:validate"
    }

    fn description(&self) -> &'static str {
        "Validate environment variables against requirements"
    }

    async fn execute(&self, ctx: &ExecutionContext) -> Result<CommandResult> {
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
        let env_vars = crate::load_env(&env_path)?;

        let results = validator.validate(&env_vars);
        let output = validator.format_results(&results);

        let all_valid = results.iter().all(|r| r.valid);

        if all_valid {
            Ok(CommandResult::success(&output))
        } else {
            Ok(CommandResult::error(&output))
        }
    }
}

/// Reload environment variables
pub struct EnvReloadCommand;

#[async_trait]
impl CommandExecutor for EnvReloadCommand {
    fn name(&self) -> &'static str {
        "env:reload"
    }

    fn description(&self) -> &'static str {
        "Reload environment variables from .env file"
    }

    async fn execute(&self, _ctx: &ExecutionContext) -> Result<CommandResult> {
        let env_path = PathBuf::from(".env");

        if !env_path.exists() {
            return Ok(CommandResult::error(".env file not found"));
        }

        let count = crate::reload_env(&env_path)?;

        Ok(CommandResult::success(&format!(
            "Reloaded {} environment variables",
            count
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::{ExecutionOptions, ResponseFormat};
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_env_validate_command() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        fs::write(".env", "APP_ENV=production\n").unwrap();

        let cmd = EnvValidateCommand;
        let ctx = ExecutionContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.message.is_some());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_env_reload_command() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        fs::write(".env", "TEST_VAR=test_value\n").unwrap();

        let cmd = EnvReloadCommand;
        let ctx = ExecutionContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.is_success());

        std::env::set_current_dir(original_dir).unwrap();
    }
}
