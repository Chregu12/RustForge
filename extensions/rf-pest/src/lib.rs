//! # rf-pest - Pest-style Testing Framework
//!
//! A beautiful, expressive testing framework for RustForge, inspired by Pest PHP.
//!
//! ## Features
//!
//! - **Expressive Syntax**: `test()`, `describe()`, `it()` blocks
//! - **Fluent Expectations**: `expect(&value).to_equal(expected)`
//! - **Async Support**: Full async/await support
//! - **Beautiful Output**: Colored, formatted test results
//! - **Laravel Integration**: Works with RustForge models and facades
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rf_pest::prelude::*;
//!
//! // Simple test
//! test("it works", || {
//!     expect(&(2 + 2)).to_equal(&4);
//! });
//!
//! // Async test
//! test_async("users can be created", async || {
//!     let user = User::create(json!({"email": "test@test.com"})).await?;
//!     expect(&user.email).to_equal(&"test@test.com".to_string());
//! });
//!
//! // BDD-style
//! describe("User Registration", |ctx| {
//!     ctx.it("validates email format", || {
//!         let result = validate_email("invalid");
//!         expect(&result).to_be_err();
//!     });
//!
//!     ctx.it("hashes passwords", || {
//!         let hash = Hash::make("password")?;
//!         expect(&hash).not().to_equal(&"password".to_string());
//!     });
//! });
//! ```
//!
//! ## Expectation API
//!
//! ```rust,ignore
//! // Equality
//! expect(&value).to_equal(&expected);
//! expect(&value).not().to_equal(&other);
//!
//! // Booleans
//! expect(&value).to_be_true();
//! expect(&value).to_be_false();
//!
//! // Options
//! expect(&option).to_be_some();
//! expect(&option).to_be_none();
//!
//! // Results
//! expect(&result).to_be_ok();
//! expect(&result).to_be_err();
//!
//! // Strings
//! expect(&string).to_contain("substring");
//! expect(&string).to_start_with("prefix");
//! expect(&string).to_end_with("suffix");
//! expect(&string).to_match(r"\d+");
//!
//! // Collections
//! expect(&vec).to_have_count(5);
//! expect(&vec).to_contain_item(&item);
//! expect(&vec).to_be_empty();
//!
//! // Numbers
//! expect(&num).to_be_greater_than(&5);
//! expect(&num).to_be_less_than(&10);
//! expect(&num).to_be_between(&1, &10);
//!
//! // Types
//! expect(&value).to_be_type::<String>();
//! ```

pub mod describe;
pub mod expect;
pub mod output;
pub mod runner;
pub mod test_fn;

pub use describe::{describe, DescribeContext};
pub use expect::{expect, Expectation};
pub use output::{TestOutput, TestResult as PestTestResult};
pub use runner::{run_tests, TestRunner};
pub use test_fn::{test, test_async, Test, TestRegistry};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::describe::{describe, DescribeContext};
    pub use crate::expect::{expect, Expectation};
    pub use crate::test_fn::{test, test_async};
    pub use crate::{PestTestResult, TestOutput, TestRunner};
}

/// Pest facade for static access
pub struct Pest;

impl Pest {
    /// Create a simple test
    ///
    /// ```rust,ignore
    /// Pest::test("it works", || {
    ///     expect(&(2 + 2)).to_equal(&4);
    /// });
    /// ```
    pub fn test<F>(name: &str, test_fn: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        test(name, test_fn);
    }

    /// Create an async test
    ///
    /// ```rust,ignore
    /// Pest::test_async("async works", async || {
    ///     let result = some_async_fn().await;
    ///     expect(&result).to_be_ok();
    /// });
    /// ```
    pub fn test_async<F, Fut>(name: &str, test_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        test_async(name, test_fn);
    }

    /// Create a describe block for BDD-style tests
    ///
    /// ```rust,ignore
    /// Pest::describe("User", |ctx| {
    ///     ctx.it("can be created", || {
    ///         // test
    ///     });
    /// });
    /// ```
    pub fn describe<F>(name: &str, setup: F)
    where
        F: FnOnce(&mut DescribeContext),
    {
        describe(name, setup);
    }

    /// Run all registered tests
    pub fn run() -> TestRunner {
        TestRunner::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expect_equal() {
        expect(&42).to_equal(&42);
    }

    #[test]
    fn test_expect_not_equal() {
        expect(&42).not().to_equal(&43);
    }

    #[test]
    fn test_expect_true() {
        expect(&true).to_be_true();
    }

    #[test]
    fn test_expect_contains() {
        expect(&"Hello World".to_string()).to_contain("World");
    }

    #[test]
    fn test_expect_some() {
        expect(&Some(42)).to_be_some();
    }

    #[test]
    fn test_expect_none() {
        let none: Option<i32> = None;
        expect(&none).to_be_none();
    }
}
