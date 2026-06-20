//! Test runner

use crate::output::{TestOutput, TestResult};
use crate::test_fn::{registry, TestFn};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

/// Test runner configuration
pub struct TestRunner {
    filter: Option<String>,
    verbose: bool,
    stop_on_failure: bool,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new() -> Self {
        Self {
            filter: None,
            verbose: false,
            stop_on_failure: false,
        }
    }

    /// Filter tests by name pattern
    pub fn filter(mut self, pattern: &str) -> Self {
        self.filter = Some(pattern.to_string());
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Stop on first failure
    pub fn stop_on_failure(mut self) -> Self {
        self.stop_on_failure = true;
        self
    }

    /// Run all registered tests
    pub fn run(&self) -> RunResult {
        let output = TestOutput::new();
        let mut results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        let start = Instant::now();

        // Get tests from registry
        let tests = if let Ok(reg) = registry().lock() {
            reg.tests()
                .iter()
                .map(|t| (t.name.clone(), t.group.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        output.print_header(tests.len());

        for (name, _group) in &tests {
            // Apply filter
            if let Some(ref filter) = self.filter {
                if !name.to_lowercase().contains(&filter.to_lowercase()) {
                    continue;
                }
            }

            // Check if skipped
            if name.contains("(SKIPPED)") || name.contains("(TODO)") {
                skipped += 1;
                output.print_skipped(name);
                results.push(TestResult::Skipped(name.clone()));
                continue;
            }

            // Run the test
            let result = self.run_single_test(name);

            match &result {
                TestResult::Passed(_, duration) => {
                    passed += 1;
                    output.print_passed(name, *duration);
                }
                TestResult::Failed(_, error) => {
                    failed += 1;
                    output.print_failed(name, error);

                    if self.stop_on_failure {
                        break;
                    }
                }
                TestResult::Skipped(_) => {
                    skipped += 1;
                    output.print_skipped(name);
                }
            }

            results.push(result);
        }

        let total_duration = start.elapsed();
        output.print_summary(passed, failed, skipped, total_duration);

        RunResult {
            passed,
            failed,
            skipped,
            duration: total_duration,
            results,
        }
    }

    /// Run a single test by name
    fn run_single_test(&self, name: &str) -> TestResult {
        let start = Instant::now();

        // Run the test from registry
        let result = if let Ok(reg) = registry().lock() {
            reg.tests().iter().find(|t| t.name == name).map(|t| {
                match &t.test_fn {
                    TestFn::Sync(f) => catch_unwind(AssertUnwindSafe(f)),
                    TestFn::Async(f) => {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        catch_unwind(AssertUnwindSafe(|| {
                            rt.block_on(f());
                        }))
                    }
                }
            })
        } else {
            None
        };

        let duration = start.elapsed();

        match result {
            Some(Ok(())) => TestResult::Passed(name.to_string(), duration),
            Some(Err(e)) => {
                let error = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Test panicked".to_string()
                };
                TestResult::Failed(name.to_string(), error)
            }
            None => TestResult::Failed(name.to_string(), "Test not found".to_string()),
        }
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of running all tests
pub struct RunResult {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: Duration,
    pub results: Vec<TestResult>,
}

impl RunResult {
    /// Check if all tests passed
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }

    /// Get exit code (0 for success, 1 for failure)
    pub fn exit_code(&self) -> i32 {
        if self.is_success() {
            0
        } else {
            1
        }
    }
}

/// Run all registered tests
pub fn run_tests() -> RunResult {
    TestRunner::new().run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fn::test;
    use crate::expect::expect;

    #[test]
    fn test_runner_creation() {
        let runner = TestRunner::new();
        assert!(runner.filter.is_none());
        assert!(!runner.verbose);
    }
}
