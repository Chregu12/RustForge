use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;
use std::process::Command;
use tracing::{info, warn};

pub struct TestCommand {
    descriptor: CommandDescriptor,
}

impl Default for TestCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl TestCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.test", "test")
            .summary("Führt alle Tests aus (Wrapper um cargo test)")
            .description(
                "Führt alle Tests im Projekt aus. \
                Unterstützt Filter, Verbose-Output und optional Code-Coverage.",
            )
            .category(CommandKind::Utility)
            .build();

        Self { descriptor }
    }

    /// Parst Command-Line Args und gibt (filter, verbose, coverage) zurück
    fn parse_args(args: &[String]) -> (Option<String>, bool, bool) {
        let mut filter: Option<String> = None;
        let mut verbose = false;
        let mut coverage = false;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            if arg.starts_with("--filter=") {
                filter = arg.strip_prefix("--filter=").map(|s| s.to_string());
            } else if arg == "--filter" {
                if let Some(value) = iter.next() {
                    filter = Some(value.clone());
                }
            } else if arg == "--verbose" || arg == "-v" {
                verbose = true;
            } else if arg == "--coverage" {
                coverage = true;
            } else if !arg.starts_with('-') {
                // Positional argument is treated as filter
                filter = Some(arg.clone());
            }
        }

        (filter, verbose, coverage)
    }

    /// Führt cargo test aus und sammelt Output
    fn run_cargo_test(
        filter: Option<&str>,
        verbose: bool,
    ) -> Result<TestRunOutput, CommandError> {
        let mut cmd = Command::new("cargo");
        cmd.arg("test");

        if verbose {
            cmd.arg("--verbose");
        }

        if let Some(pattern) = filter {
            cmd.arg(pattern);
        }

        // Capture output
        cmd.arg("--");
        if verbose {
            cmd.arg("--nocapture");
        }

        info!(
            filter = ?filter,
            verbose = verbose,
            "Executing cargo test"
        );

        let output = cmd
            .output()
            .map_err(|e| CommandError::Message(format!("Failed to execute cargo test: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse test statistics from output
        let stats = Self::parse_test_output(&stdout, &stderr);

        Ok(TestRunOutput {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            stats,
        })
    }

    /// Parst Test-Output und extrahiert Statistiken
    fn parse_test_output(stdout: &str, stderr: &str) -> TestStatistics {
        let combined = format!("{}\n{}", stdout, stderr);

        // Look for patterns like:
        // "test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
        // "test result: FAILED. 10 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out"

        let mut passed = 0;
        let mut failed = 0;
        let mut ignored = 0;
        let mut filtered = 0;

        for line in combined.lines() {
            if line.contains("test result:") {
                // Try to extract numbers
                if let Some(p) = Self::extract_number(line, "passed") {
                    passed = p;
                }
                if let Some(f) = Self::extract_number(line, "failed") {
                    failed = f;
                }
                if let Some(i) = Self::extract_number(line, "ignored") {
                    ignored = i;
                }
                if let Some(filt) = Self::extract_number(line, "filtered out") {
                    filtered = filt;
                }
            }
        }

        TestStatistics {
            passed,
            failed,
            ignored,
            filtered,
            total: passed + failed + ignored,
        }
    }

    /// Extrahiert eine Zahl aus einem String-Muster wie "15 passed"
    fn extract_number(text: &str, keyword: &str) -> Option<usize> {
        text.find(keyword).and_then(|pos| {
            // Look backwards for the number
            let before = &text[..pos];
            before
                .split_whitespace()
                .rev()
                .find_map(|word| word.parse::<usize>().ok())
        })
    }

    /// Formatiert das Ergebnis für Human-Output
    fn format_human_output(output: &TestRunOutput, coverage: bool) -> String {
        let mut result = String::new();

        if output.success {
            result.push_str("✓ Tests erfolgreich\n\n");
        } else {
            result.push_str("✗ Tests fehlgeschlagen\n\n");
        }

        // Test Statistics
        result.push_str(&format!(
            "Statistik:\n  Passed:   {}\n  Failed:   {}\n  Ignored:  {}\n  Filtered: {}\n  Total:    {}\n\n",
            output.stats.passed,
            output.stats.failed,
            output.stats.ignored,
            output.stats.filtered,
            output.stats.total
        ));

        if coverage {
            result.push_str("Info: Code-Coverage-Unterstützung ist noch nicht implementiert.\n");
            result.push_str("Verwenden Sie tools wie 'cargo-tarpaulin' oder 'cargo-llvm-cov'.\n\n");
        }

        // Show stderr if there are failures
        if !output.success && !output.stderr.is_empty() {
            result.push_str("Fehler-Output:\n");
            result.push_str(&output.stderr);
        }

        result
    }
}

#[derive(Debug)]
struct TestRunOutput {
    success: bool,
    exit_code: i32,
    stdout: String,
    stderr: String,
    stats: TestStatistics,
}

#[derive(Debug, Clone)]
struct TestStatistics {
    passed: usize,
    failed: usize,
    ignored: usize,
    filtered: usize,
    total: usize,
}

#[async_trait]
impl FoundryCommand for TestCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (filter, verbose, coverage) = Self::parse_args(&ctx.args);

        if coverage {
            warn!("Code-Coverage ist noch nicht vollständig implementiert");
        }

        // Run cargo test
        let output = Self::run_cargo_test(filter.as_deref(), verbose)?;

        let status = if output.success {
            CommandStatus::Success
        } else {
            CommandStatus::Failure
        };

        let message = match ctx.format {
            foundry_plugins::ResponseFormat::Human => {
                Some(Self::format_human_output(&output, coverage))
            }
            foundry_plugins::ResponseFormat::Json => None,
        };

        let data = json!({
            "exit_code": output.exit_code,
            "filter": filter,
            "verbose": verbose,
            "coverage": coverage,
            "statistics": {
                "passed": output.stats.passed,
                "failed": output.stats.failed,
                "ignored": output.stats.ignored,
                "filtered": output.stats.filtered,
                "total": output.stats.total,
            },
            "stdout": output.stdout,
            "stderr": output.stderr,
        });

        Ok(CommandResult {
            status,
            message,
            data: Some(data),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::ResponseFormat;
    use serde_json::Value;

    fn create_test_context(args: Vec<String>) -> CommandContext {
        CommandContext {
            args,
            format: ResponseFormat::Human,
            metadata: Value::Null,
            config: Value::Null,
            options: Default::default(),
            artifacts: std::sync::Arc::new(foundry_infra::LocalArtifactPort::default()),
            migrations: std::sync::Arc::new(foundry_infra::SeaOrmMigrationService::default()),
            seeds: std::sync::Arc::new(foundry_infra::SeaOrmSeedService::default()),
            validation: std::sync::Arc::new(foundry_infra::SimpleValidationService::default()),
            storage: std::sync::Arc::new(foundry_infra::FileStorageAdapter::new(
                std::sync::Arc::new(
                    foundry_storage::manager::StorageManager::new(
                        foundry_storage::config::StorageConfig::from_env(),
                    )
                    .unwrap(),
                ),
            )),
            cache: std::sync::Arc::new(foundry_infra::InMemoryCacheStore::default()),
            queue: std::sync::Arc::new(foundry_infra::InMemoryQueue::default()),
            events: std::sync::Arc::new(foundry_infra::InMemoryEventBus::default()),
        }
    }

    #[test]
    fn test_parse_args_empty() {
        let (filter, verbose, coverage) = TestCommand::parse_args(&[]);
        assert_eq!(filter, None);
        assert!(!verbose);
        assert!(!coverage);
    }

    #[test]
    fn test_parse_args_filter_with_equals() {
        let args = vec!["--filter=integration".to_string()];
        let (filter, verbose, coverage) = TestCommand::parse_args(&args);
        assert_eq!(filter, Some("integration".to_string()));
        assert!(!verbose);
        assert!(!coverage);
    }

    #[test]
    fn test_parse_args_filter_separate() {
        let args = vec!["--filter".to_string(), "unit_test".to_string()];
        let (filter, verbose, coverage) = TestCommand::parse_args(&args);
        assert_eq!(filter, Some("unit_test".to_string()));
        assert!(!verbose);
        assert!(!coverage);
    }

    #[test]
    fn test_parse_args_positional_filter() {
        let args = vec!["my_test".to_string()];
        let (filter, verbose, coverage) = TestCommand::parse_args(&args);
        assert_eq!(filter, Some("my_test".to_string()));
        assert!(!verbose);
        assert!(!coverage);
    }

    #[test]
    fn test_parse_args_verbose() {
        let args = vec!["--verbose".to_string()];
        let (filter, verbose, coverage) = TestCommand::parse_args(&args);
        assert_eq!(filter, None);
        assert!(verbose);
        assert!(!coverage);
    }

    #[test]
    fn test_parse_args_verbose_short() {
        let args = vec!["-v".to_string()];
        let (filter, verbose, coverage) = TestCommand::parse_args(&args);
        assert_eq!(filter, None);
        assert!(verbose);
        assert!(!coverage);
    }

    #[test]
    fn test_parse_args_coverage() {
        let args = vec!["--coverage".to_string()];
        let (filter, verbose, coverage) = TestCommand::parse_args(&args);
        assert_eq!(filter, None);
        assert!(!verbose);
        assert!(coverage);
    }

    #[test]
    fn test_parse_args_all_options() {
        let args = vec![
            "--filter=my_test".to_string(),
            "--verbose".to_string(),
            "--coverage".to_string(),
        ];
        let (filter, verbose, coverage) = TestCommand::parse_args(&args);
        assert_eq!(filter, Some("my_test".to_string()));
        assert!(verbose);
        assert!(coverage);
    }

    #[test]
    fn test_extract_number() {
        let text = "test result: ok. 15 passed; 0 failed";
        assert_eq!(TestCommand::extract_number(text, "passed"), Some(15));
        assert_eq!(TestCommand::extract_number(text, "failed"), Some(0));
    }

    #[test]
    fn test_parse_test_output_success() {
        let output = "test result: ok. 15 passed; 0 failed; 2 ignored; 0 measured; 3 filtered out";
        let stats = TestCommand::parse_test_output(output, "");
        assert_eq!(stats.passed, 15);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.ignored, 2);
        assert_eq!(stats.filtered, 3);
        assert_eq!(stats.total, 17);
    }

    #[test]
    fn test_parse_test_output_failure() {
        let output = "test result: FAILED. 10 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out";
        let stats = TestCommand::parse_test_output(output, "");
        assert_eq!(stats.passed, 10);
        assert_eq!(stats.failed, 2);
        assert_eq!(stats.ignored, 1);
        assert_eq!(stats.total, 13);
    }

    #[test]
    fn test_descriptor() {
        let command = TestCommand::new();
        let descriptor = command.descriptor();
        assert_eq!(descriptor.name, "test");
        assert_eq!(descriptor.category, CommandKind::Utility);
    }

    #[tokio::test]
    async fn test_execute_runs_tests() {
        let command = TestCommand::new();
        let ctx = create_test_context(vec![]);

        // This will actually run cargo test, which should pass for this project
        let result = command.execute(ctx).await;
        assert!(result.is_ok());

        let cmd_result = result.unwrap();
        // Verify we have data
        assert!(cmd_result.data.is_some());

        let data = cmd_result.data.unwrap();
        assert!(data.get("statistics").is_some());
        assert!(data.get("exit_code").is_some());
    }
}
