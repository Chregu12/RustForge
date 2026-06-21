//! Expectation API for fluent assertions

use std::fmt::Debug;

/// Create an expectation for a value
///
/// ```rust,ignore
/// expect(&value).to_equal(&expected);
/// expect(&value).not().to_equal(&other);
/// ```
pub fn expect<T>(value: &T) -> Expectation<'_, T>
where
    T: Debug,
{
    Expectation {
        value,
        negated: false,
    }
}

/// Fluent expectation builder
pub struct Expectation<'a, T>
where
    T: Debug,
{
    value: &'a T,
    negated: bool,
}

impl<'a, T> Expectation<'a, T>
where
    T: Debug,
{
    /// Negate the expectation
    ///
    /// ```rust,ignore
    /// expect(&value).not().to_equal(&other);
    /// ```
    #[allow(clippy::should_implement_trait)] // intentional fluent assertion negation, not std Not
    pub fn not(self) -> Self {
        Self {
            value: self.value,
            negated: !self.negated,
        }
    }

    fn assert(&self, condition: bool, message: &str) {
        let result = if self.negated { !condition } else { condition };
        if !result {
            let prefix = if self.negated { "not " } else { "" };
            panic!(
                "Expectation failed: expected {:?} {}{}",
                self.value, prefix, message
            );
        }
    }
}

// Equality expectations
impl<'a, T> Expectation<'a, T>
where
    T: Debug + PartialEq,
{
    /// Assert that values are equal
    ///
    /// ```rust,ignore
    /// expect(&42).to_equal(&42);
    /// ```
    pub fn to_equal(&self, expected: &T) {
        let condition = self.value == expected;
        self.assert(condition, &format!("to equal {:?}", expected));
    }

    /// Assert that value is in a list
    pub fn to_be_one_of(&self, options: &[T]) {
        let condition = options.contains(self.value);
        self.assert(condition, &format!("to be one of {:?}", options));
    }
}

// Boolean expectations
impl<'a> Expectation<'a, bool> {
    /// Assert that value is true
    pub fn to_be_true(&self) {
        self.assert(*self.value, "to be true");
    }

    /// Assert that value is false
    pub fn to_be_false(&self) {
        self.assert(!*self.value, "to be false");
    }
}

// Option expectations
impl<'a, T> Expectation<'a, Option<T>>
where
    T: Debug,
{
    /// Assert that option is Some
    pub fn to_be_some(&self) {
        let condition = self.value.is_some();
        self.assert(condition, "to be Some");
    }

    /// Assert that option is None
    pub fn to_be_none(&self) {
        let condition = self.value.is_none();
        self.assert(condition, "to be None");
    }
}

// Result expectations
impl<'a, T, E> Expectation<'a, Result<T, E>>
where
    T: Debug,
    E: Debug,
{
    /// Assert that result is Ok
    pub fn to_be_ok(&self) {
        let condition = self.value.is_ok();
        self.assert(condition, "to be Ok");
    }

    /// Assert that result is Err
    pub fn to_be_err(&self) {
        let condition = self.value.is_err();
        self.assert(condition, "to be Err");
    }
}

// String expectations
impl<'a> Expectation<'a, String> {
    /// Assert that string contains substring
    pub fn to_contain(&self, substring: &str) {
        let condition = self.value.contains(substring);
        self.assert(condition, &format!("to contain {:?}", substring));
    }

    /// Assert that string starts with prefix
    pub fn to_start_with(&self, prefix: &str) {
        let condition = self.value.starts_with(prefix);
        self.assert(condition, &format!("to start with {:?}", prefix));
    }

    /// Assert that string ends with suffix
    pub fn to_end_with(&self, suffix: &str) {
        let condition = self.value.ends_with(suffix);
        self.assert(condition, &format!("to end with {:?}", suffix));
    }

    /// Assert that string matches regex
    pub fn to_match(&self, pattern: &str) {
        let regex = regex::Regex::new(pattern).expect("Invalid regex pattern");
        let condition = regex.is_match(self.value);
        self.assert(condition, &format!("to match {:?}", pattern));
    }

    /// Assert that string is empty
    pub fn to_be_empty(&self) {
        let condition = self.value.is_empty();
        self.assert(condition, "to be empty");
    }

    /// Assert string length
    pub fn to_have_length(&self, length: usize) {
        let condition = self.value.len() == length;
        self.assert(condition, &format!("to have length {}", length));
    }
}

// &str expectations
impl<'a> Expectation<'a, &str> {
    /// Assert that string contains substring
    pub fn to_contain(&self, substring: &str) {
        let condition = self.value.contains(substring);
        self.assert(condition, &format!("to contain {:?}", substring));
    }

    /// Assert that string starts with prefix
    pub fn to_start_with(&self, prefix: &str) {
        let condition = self.value.starts_with(prefix);
        self.assert(condition, &format!("to start with {:?}", prefix));
    }

    /// Assert that string ends with suffix
    pub fn to_end_with(&self, suffix: &str) {
        let condition = self.value.ends_with(suffix);
        self.assert(condition, &format!("to end with {:?}", suffix));
    }

    /// Assert that string is empty
    pub fn to_be_empty(&self) {
        let condition = self.value.is_empty();
        self.assert(condition, "to be empty");
    }
}

// Vec expectations
impl<'a, T> Expectation<'a, Vec<T>>
where
    T: Debug + PartialEq,
{
    /// Assert that vec has specific count
    pub fn to_have_count(&self, count: usize) {
        let condition = self.value.len() == count;
        self.assert(condition, &format!("to have count {}", count));
    }

    /// Assert that vec contains item
    pub fn to_contain_item(&self, item: &T) {
        let condition = self.value.contains(item);
        self.assert(condition, &format!("to contain {:?}", item));
    }

    /// Assert that vec is empty
    pub fn to_be_empty(&self) {
        let condition = self.value.is_empty();
        self.assert(condition, "to be empty");
    }

    /// Assert that vec is not empty
    pub fn to_not_be_empty(&self) {
        let condition = !self.value.is_empty();
        self.assert(condition, "to not be empty");
    }
}

// Numeric expectations for common types
macro_rules! impl_numeric_expectations {
    ($($t:ty),*) => {
        $(
            impl<'a> Expectation<'a, $t> {
                /// Assert that number is greater than expected
                pub fn to_be_greater_than(&self, expected: &$t) {
                    let condition = *self.value > *expected;
                    self.assert(condition, &format!("to be greater than {}", expected));
                }

                /// Assert that number is greater than or equal to expected
                pub fn to_be_greater_than_or_equal(&self, expected: &$t) {
                    let condition = *self.value >= *expected;
                    self.assert(condition, &format!("to be greater than or equal to {}", expected));
                }

                /// Assert that number is less than expected
                pub fn to_be_less_than(&self, expected: &$t) {
                    let condition = *self.value < *expected;
                    self.assert(condition, &format!("to be less than {}", expected));
                }

                /// Assert that number is less than or equal to expected
                pub fn to_be_less_than_or_equal(&self, expected: &$t) {
                    let condition = *self.value <= *expected;
                    self.assert(condition, &format!("to be less than or equal to {}", expected));
                }

                /// Assert that number is between min and max (inclusive)
                pub fn to_be_between(&self, min: &$t, max: &$t) {
                    let condition = *self.value >= *min && *self.value <= *max;
                    self.assert(condition, &format!("to be between {} and {}", min, max));
                }

                /// Assert that number is positive
                pub fn to_be_positive(&self) {
                    let condition = *self.value > 0 as $t;
                    self.assert(condition, "to be positive");
                }

                /// Assert that number is negative
                pub fn to_be_negative(&self) {
                    let condition = *self.value < 0 as $t;
                    self.assert(condition, "to be negative");
                }

                /// Assert that number is zero
                pub fn to_be_zero(&self) {
                    let condition = *self.value == 0 as $t;
                    self.assert(condition, "to be zero");
                }
            }
        )*
    };
}

impl_numeric_expectations!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

// JSON value expectations
impl<'a> Expectation<'a, serde_json::Value> {
    /// Assert that JSON is an object
    pub fn to_be_object(&self) {
        let condition = self.value.is_object();
        self.assert(condition, "to be an object");
    }

    /// Assert that JSON is an array
    pub fn to_be_array(&self) {
        let condition = self.value.is_array();
        self.assert(condition, "to be an array");
    }

    /// Assert that JSON is a string
    pub fn to_be_string(&self) {
        let condition = self.value.is_string();
        self.assert(condition, "to be a string");
    }

    /// Assert that JSON is a number
    pub fn to_be_number(&self) {
        let condition = self.value.is_number();
        self.assert(condition, "to be a number");
    }

    /// Assert that JSON is a boolean
    pub fn to_be_boolean(&self) {
        let condition = self.value.is_boolean();
        self.assert(condition, "to be a boolean");
    }

    /// Assert that JSON is null
    pub fn to_be_null(&self) {
        let condition = self.value.is_null();
        self.assert(condition, "to be null");
    }

    /// Assert that JSON has a key
    pub fn to_have_key(&self, key: &str) {
        let condition = self.value.get(key).is_some();
        self.assert(condition, &format!("to have key {:?}", key));
    }

    /// Assert that JSON array has count
    pub fn to_have_count(&self, count: usize) {
        let condition = self.value.as_array().map(|a| a.len() == count).unwrap_or(false);
        self.assert(condition, &format!("to have count {}", count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equal() {
        expect(&42).to_equal(&42);
    }

    #[test]
    fn test_not_equal() {
        expect(&42).not().to_equal(&43);
    }

    #[test]
    #[should_panic(expected = "Expectation failed")]
    fn test_equal_fails() {
        expect(&42).to_equal(&43);
    }

    #[test]
    fn test_string_contains() {
        expect(&"Hello World".to_string()).to_contain("World");
    }

    #[test]
    fn test_vec_count() {
        expect(&vec![1, 2, 3]).to_have_count(3);
    }

    #[test]
    fn test_numeric_greater() {
        expect(&10i32).to_be_greater_than(&5i32);
    }

    #[test]
    fn test_between() {
        expect(&5i32).to_be_between(&1i32, &10i32);
    }
}
