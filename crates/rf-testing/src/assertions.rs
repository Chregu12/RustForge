//! Custom test assertions

use std::fmt::Debug;

/// Assert vectors are equal (order-independent)
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_vec_eq;
///
/// let a = vec![1, 2, 3];
/// let b = vec![3, 1, 2];
/// assert_vec_eq(a, b); // Passes
/// ```
pub fn assert_vec_eq<T: PartialEq + Debug + Ord>(mut a: Vec<T>, mut b: Vec<T>) {
    a.sort();
    b.sort();
    assert_eq!(
        a, b,
        "Vectors not equal (order-independent):\nLeft: {:?}\nRight: {:?}",
        a, b
    );
}

/// Assert option is Some and matches expected value
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_some_eq;
///
/// let value = Some(42);
/// assert_some_eq(value, 42);
/// ```
pub fn assert_some_eq<T: PartialEq + Debug>(actual: Option<T>, expected: T) {
    match actual {
        Some(val) => assert_eq!(val, expected),
        None => panic!("Expected Some({:?}), got None", expected),
    }
}

/// Assert option is Some
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_some;
///
/// let value = Some(42);
/// assert_some(value);
/// ```
pub fn assert_some<T: Debug>(actual: Option<T>) -> T {
    match actual {
        Some(val) => val,
        None => panic!("Expected Some, got None"),
    }
}

/// Assert option is None
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_none;
///
/// let value: Option<i32> = None;
/// assert_none(value);
/// ```
pub fn assert_none<T: Debug>(actual: Option<T>) {
    if let Some(val) = actual {
        panic!("Expected None, got Some({:?})", val);
    }
}

/// Assert result is Ok and matches expected value
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_ok_eq;
///
/// let result: Result<i32, String> = Ok(42);
/// assert_ok_eq(result, 42);
/// ```
pub fn assert_ok_eq<T: PartialEq + Debug, E: Debug>(actual: Result<T, E>, expected: T) {
    match actual {
        Ok(val) => assert_eq!(val, expected),
        Err(e) => panic!("Expected Ok({:?}), got Err({:?})", expected, e),
    }
}

/// Assert result is Ok
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_ok;
///
/// let result: Result<i32, String> = Ok(42);
/// let value = assert_ok(result);
/// assert_eq!(value, 42);
/// ```
pub fn assert_ok<T: Debug, E: Debug>(actual: Result<T, E>) -> T {
    match actual {
        Ok(val) => val,
        Err(e) => panic!("Expected Ok, got Err({:?})", e),
    }
}

/// Assert result is Err
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_err;
///
/// let result: Result<i32, String> = Err("error".to_string());
/// assert_err(result);
/// ```
pub fn assert_err<T: Debug, E: Debug>(actual: Result<T, E>) -> E {
    match actual {
        Ok(val) => panic!("Expected Err, got Ok({:?})", val),
        Err(e) => e,
    }
}

/// Assert string contains substring
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_contains;
///
/// assert_contains("Hello, World!", "World");
/// ```
pub fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "String {:?} does not contain {:?}",
        haystack,
        needle
    );
}

/// Assert string does not contain substring
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_not_contains;
///
/// assert_not_contains("Hello, World!", "Goodbye");
/// ```
pub fn assert_not_contains(haystack: &str, needle: &str) {
    assert!(
        !haystack.contains(needle),
        "String {:?} unexpectedly contains {:?}",
        haystack,
        needle
    );
}

/// Assert value is within range
///
/// # Example
///
/// ```
/// use rf_testing::assertions::assert_in_range;
///
/// assert_in_range(5, 1, 10);
/// ```
pub fn assert_in_range<T: PartialOrd + Debug>(value: T, min: T, max: T) {
    assert!(
        value >= min && value <= max,
        "Value {:?} not in range [{:?}, {:?}]",
        value,
        min,
        max
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_vec_eq() {
        assert_vec_eq(vec![1, 2, 3], vec![3, 2, 1]);
    }

    #[test]
    #[should_panic]
    fn test_assert_vec_eq_fail() {
        assert_vec_eq(vec![1, 2, 3], vec![1, 2, 4]);
    }

    #[test]
    fn test_assert_some_eq() {
        assert_some_eq(Some(42), 42);
    }

    #[test]
    #[should_panic]
    fn test_assert_some_eq_fail() {
        assert_some_eq(Some(42), 43);
    }

    #[test]
    fn test_assert_some() {
        let val = assert_some(Some(42));
        assert_eq!(val, 42);
    }

    #[test]
    #[should_panic]
    fn test_assert_some_fail() {
        assert_some::<i32>(None);
    }

    #[test]
    fn test_assert_none() {
        assert_none::<i32>(None);
    }

    #[test]
    #[should_panic]
    fn test_assert_none_fail() {
        assert_none(Some(42));
    }

    #[test]
    fn test_assert_ok_eq() {
        let result: Result<i32, String> = Ok(42);
        assert_ok_eq(result, 42);
    }

    #[test]
    #[should_panic]
    fn test_assert_ok_eq_fail() {
        let result: Result<i32, String> = Err("error".to_string());
        assert_ok_eq(result, 42);
    }

    #[test]
    fn test_assert_ok() {
        let result: Result<i32, String> = Ok(42);
        let val = assert_ok(result);
        assert_eq!(val, 42);
    }

    #[test]
    #[should_panic]
    fn test_assert_ok_fail() {
        let result: Result<i32, String> = Err("error".to_string());
        assert_ok(result);
    }

    #[test]
    fn test_assert_err() {
        let result: Result<i32, String> = Err("error".to_string());
        let err = assert_err(result);
        assert_eq!(err, "error");
    }

    #[test]
    #[should_panic]
    fn test_assert_err_fail() {
        let result: Result<i32, String> = Ok(42);
        assert_err(result);
    }

    #[test]
    fn test_assert_contains() {
        assert_contains("Hello, World!", "World");
    }

    #[test]
    #[should_panic]
    fn test_assert_contains_fail() {
        assert_contains("Hello, World!", "Goodbye");
    }

    #[test]
    fn test_assert_not_contains() {
        assert_not_contains("Hello, World!", "Goodbye");
    }

    #[test]
    #[should_panic]
    fn test_assert_not_contains_fail() {
        assert_not_contains("Hello, World!", "World");
    }

    #[test]
    fn test_assert_in_range() {
        assert_in_range(5, 1, 10);
    }

    #[test]
    #[should_panic]
    fn test_assert_in_range_fail() {
        assert_in_range(15, 1, 10);
    }
}
