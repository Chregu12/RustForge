//! BDD-style describe/it blocks

use crate::test_fn::test_in_group;
use std::future::Future;
use std::sync::Arc;

/// Context for describe blocks
pub struct DescribeContext {
    group_name: String,
    before_each: Option<Box<dyn Fn() + Send + Sync>>,
    after_each: Option<Box<dyn Fn() + Send + Sync>>,
}

impl DescribeContext {
    /// Create a new describe context
    pub fn new(name: &str) -> Self {
        Self {
            group_name: name.to_string(),
            before_each: None,
            after_each: None,
        }
    }

    /// Set up a function to run before each test
    ///
    /// ```rust,ignore
    /// describe("User", |ctx| {
    ///     ctx.before_each(|| {
    ///         // Setup
    ///     });
    ///
    ///     ctx.it("works", || {
    ///         // Test
    ///     });
    /// });
    /// ```
    pub fn before_each<F>(&mut self, setup: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.before_each = Some(Box::new(setup));
    }

    /// Set up a function to run after each test
    pub fn after_each<F>(&mut self, teardown: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.after_each = Some(Box::new(teardown));
    }

    /// Define a test case
    ///
    /// ```rust,ignore
    /// ctx.it("should work", || {
    ///     expect(&true).to_be_true();
    /// });
    /// ```
    pub fn it<F>(&self, description: &str, test_fn: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let full_name = format!("{} {}", self.group_name, description);
        test_in_group(&self.group_name, &full_name, test_fn);
    }

    /// Define an async test case
    pub fn it_async<F, Fut>(&self, description: &str, test_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let full_name = format!("{} {}", self.group_name, description);
        let test_fn = Arc::new(test_fn);

        test_in_group(&self.group_name, &full_name, move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let test_fn = Arc::clone(&test_fn);
            rt.block_on(async move {
                test_fn().await;
            });
        });
    }

    /// Skip a test
    pub fn it_skip<F>(&self, description: &str, _test_fn: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        let full_name = format!("{} {} (SKIPPED)", self.group_name, description);
        test_in_group(&self.group_name, &full_name, || {
            // Skipped test does nothing
        });
    }

    /// Mark a test as pending (todo)
    pub fn it_todo(&self, description: &str) {
        let full_name = format!("{} {} (TODO)", self.group_name, description);
        test_in_group(&self.group_name, &full_name, || {
            // TODO test does nothing
        });
    }

    /// Nested describe block
    pub fn describe<F>(&self, name: &str, setup: F)
    where
        F: FnOnce(&mut DescribeContext),
    {
        let full_name = format!("{} > {}", self.group_name, name);
        let mut ctx = DescribeContext::new(&full_name);
        setup(&mut ctx);
    }
}

/// Create a describe block for BDD-style testing
///
/// ```rust,ignore
/// describe("User", |ctx| {
///     ctx.it("can be created", || {
///         let user = User::new("test@test.com");
///         expect(&user.email).to_equal(&"test@test.com".to_string());
///     });
///
///     ctx.it("validates email", || {
///         let result = User::validate_email("invalid");
///         expect(&result).to_be_err();
///     });
/// });
/// ```
pub fn describe<F>(name: &str, setup: F)
where
    F: FnOnce(&mut DescribeContext),
{
    let mut ctx = DescribeContext::new(name);
    setup(&mut ctx);
}

/// Alias for describe
pub fn context<F>(name: &str, setup: F)
where
    F: FnOnce(&mut DescribeContext),
{
    describe(name, setup);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expect::expect;

    #[test]
    fn test_describe_context() {
        describe("Math", |ctx| {
            ctx.it("adds numbers", || {
                expect(&(2 + 2)).to_equal(&4);
            });

            ctx.it("subtracts numbers", || {
                expect(&(5 - 3)).to_equal(&2);
            });
        });
    }

    #[test]
    fn test_nested_describe() {
        describe("User", |ctx| {
            ctx.describe("validation", |ctx| {
                ctx.it("validates email", || {
                    let email = "test@test.com";
                    expect(&email.contains("@")).to_be_true();
                });
            });
        });
    }
}
