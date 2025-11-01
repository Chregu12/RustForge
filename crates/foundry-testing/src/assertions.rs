/// Custom assertion macros for testing

/// Assert that a value is Some and unwrap it
#[macro_export]
macro_rules! assert_some {
    ($expr:expr) => {
        match $expr {
            Some(val) => val,
            None => panic!("assertion failed: {} is None", stringify!($expr)),
        }
    };
    ($expr:expr, $($arg:tt)+) => {
        match $expr {
            Some(val) => val,
            None => panic!("assertion failed: {} is None: {}", stringify!($expr), format!($($arg)+)),
        }
    };
}

/// Assert that a value is None
#[macro_export]
macro_rules! assert_none {
    ($expr:expr) => {
        match $expr {
            None => (),
            Some(val) => panic!(
                "assertion failed: {} is Some({:?})",
                stringify!($expr),
                val
            ),
        }
    };
    ($expr:expr, $($arg:tt)+) => {
        match $expr {
            None => (),
            Some(val) => panic!(
                "assertion failed: {} is Some({:?}): {}",
                stringify!($expr),
                val,
                format!($($arg)+)
            ),
        }
    };
}

/// Assert that a Result is Ok and unwrap it
#[macro_export]
macro_rules! assert_ok {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => panic!("assertion failed: {} is Err({:?})", stringify!($expr), err),
        }
    };
    ($expr:expr, $($arg:tt)+) => {
        match $expr {
            Ok(val) => val,
            Err(err) => panic!(
                "assertion failed: {} is Err({:?}): {}",
                stringify!($expr),
                err,
                format!($($arg)+)
            ),
        }
    };
}

/// Assert that a Result is Err
#[macro_export]
macro_rules! assert_err {
    ($expr:expr) => {
        match $expr {
            Err(_) => (),
            Ok(val) => panic!("assertion failed: {} is Ok({:?})", stringify!($expr), val),
        }
    };
    ($expr:expr, $($arg:tt)+) => {
        match $expr {
            Err(_) => (),
            Ok(val) => panic!(
                "assertion failed: {} is Ok({:?}): {}",
                stringify!($expr),
                val,
                format!($($arg)+)
            ),
        }
    };
}

/// Assert that a string contains a substring
#[macro_export]
macro_rules! assert_contains {
    ($haystack:expr, $needle:expr) => {
        assert!(
            $haystack.contains($needle),
            "assertion failed: '{}' does not contain '{}'",
            $haystack,
            $needle
        )
    };
    ($haystack:expr, $needle:expr, $($arg:tt)+) => {
        assert!(
            $haystack.contains($needle),
            "assertion failed: '{}' does not contain '{}': {}",
            $haystack,
            $needle,
            format!($($arg)+)
        )
    };
}

/// Assert that a string does not contain a substring
#[macro_export]
macro_rules! assert_not_contains {
    ($haystack:expr, $needle:expr) => {
        assert!(
            !$haystack.contains($needle),
            "assertion failed: '{}' contains '{}'",
            $haystack,
            $needle
        )
    };
    ($haystack:expr, $needle:expr, $($arg:tt)+) => {
        assert!(
            !$haystack.contains($needle),
            "assertion failed: '{}' contains '{}': {}",
            $haystack,
            $needle,
            format!($($arg)+)
        )
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_assert_some() {
        let value = Some(42);
        let unwrapped = assert_some!(value);
        assert_eq!(unwrapped, 42);
    }

    #[test]
    #[should_panic(expected = "is None")]
    fn test_assert_some_panics() {
        let value: Option<i32> = None;
        assert_some!(value);
    }

    #[test]
    fn test_assert_none() {
        let value: Option<i32> = None;
        assert_none!(value);
    }

    #[test]
    fn test_assert_ok() {
        let result: Result<i32, &str> = Ok(42);
        let unwrapped = assert_ok!(result);
        assert_eq!(unwrapped, 42);
    }

    #[test]
    fn test_assert_err() {
        let result: Result<i32, &str> = Err("error");
        assert_err!(result);
    }

    #[test]
    fn test_assert_contains() {
        assert_contains!("hello world", "world");
    }

    #[test]
    fn test_assert_not_contains() {
        assert_not_contains!("hello world", "foo");
    }
}
