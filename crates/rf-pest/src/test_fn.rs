//! Test function registration and execution

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};

/// Global test registry
static TEST_REGISTRY: OnceLock<Mutex<TestRegistry>> = OnceLock::new();

/// Get the global test registry
fn get_registry() -> &'static Mutex<TestRegistry> {
    TEST_REGISTRY.get_or_init(|| Mutex::new(TestRegistry::new()))
}

/// A test case
pub struct Test {
    pub name: String,
    pub group: Option<String>,
    pub test_fn: TestFn,
}

/// Test function type
pub enum TestFn {
    Sync(Box<dyn Fn() + Send + Sync>),
    Async(Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>),
}

/// Registry of all tests
pub struct TestRegistry {
    tests: HashMap<String, Test>,
    order: Vec<String>,
}

impl TestRegistry {
    /// Create a new test registry
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Register a test
    pub fn register(&mut self, test: Test) {
        let name = test.name.clone();
        self.tests.insert(name.clone(), test);
        self.order.push(name);
    }

    /// Get all tests in order
    pub fn tests(&self) -> Vec<&Test> {
        self.order
            .iter()
            .filter_map(|name| self.tests.get(name))
            .collect()
    }

    /// Get test count
    pub fn count(&self) -> usize {
        self.tests.len()
    }

    /// Clear all tests
    pub fn clear(&mut self) {
        self.tests.clear();
        self.order.clear();
    }
}

impl Default for TestRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Register a synchronous test
///
/// ```rust,ignore
/// test("it works", || {
///     expect(&(2 + 2)).to_equal(&4);
/// });
/// ```
pub fn test<F>(name: &str, test_fn: F)
where
    F: Fn() + Send + Sync + 'static,
{
    let test = Test {
        name: name.to_string(),
        group: None,
        test_fn: TestFn::Sync(Box::new(test_fn)),
    };

    if let Ok(mut registry) = get_registry().lock() {
        registry.register(test);
    }
}

/// Register a test with a group
pub fn test_in_group<F>(group: &str, name: &str, test_fn: F)
where
    F: Fn() + Send + Sync + 'static,
{
    let test = Test {
        name: name.to_string(),
        group: Some(group.to_string()),
        test_fn: TestFn::Sync(Box::new(test_fn)),
    };

    if let Ok(mut registry) = get_registry().lock() {
        registry.register(test);
    }
}

/// Register an async test
///
/// ```rust,ignore
/// test_async("async works", || async {
///     let result = some_async_fn().await;
///     expect(&result).to_be_ok();
/// });
/// ```
pub fn test_async<F, Fut>(name: &str, test_fn: F)
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let test_fn = Arc::new(test_fn);

    let test = Test {
        name: name.to_string(),
        group: None,
        test_fn: TestFn::Async(Box::new(move || {
            let test_fn = Arc::clone(&test_fn);
            Box::pin(async move {
                test_fn().await;
            })
        })),
    };

    if let Ok(mut registry) = get_registry().lock() {
        registry.register(test);
    }
}

/// Get the test registry for running tests
pub fn registry() -> &'static Mutex<TestRegistry> {
    get_registry()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registration() {
        // Clear registry first
        if let Ok(mut reg) = get_registry().lock() {
            reg.clear();
        }

        test("test 1", || {
            assert!(true);
        });

        test("test 2", || {
            assert!(true);
        });

        if let Ok(reg) = get_registry().lock() {
            assert_eq!(reg.count(), 2);
        }
    }
}
