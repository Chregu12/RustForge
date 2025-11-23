/// Parse a CSV-style array from a string
///
/// # Examples
///
/// ```
/// use foundry_advanced_input::parse_array;
///
/// let tags = parse_array("tag1,tag2,tag3");
/// assert_eq!(tags, vec!["tag1", "tag2", "tag3"]);
///
/// let empty: Vec<&str> = parse_array("");
/// assert_eq!(empty, Vec::<&str>::new());
/// ```
pub fn parse_array(input: &str) -> Vec<&str> {
    if input.is_empty() {
        return Vec::new();
    }

    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a numeric array from a string
///
/// # Examples
///
/// ```
/// use foundry_advanced_input::parse_numeric_array;
///
/// let ids = parse_numeric_array::<i32>("1,2,3,4,5").unwrap();
/// assert_eq!(ids, vec![1, 2, 3, 4, 5]);
///
/// let floats = parse_numeric_array::<f64>("1.5,2.5,3.5").unwrap();
/// assert_eq!(floats, vec![1.5, 2.5, 3.5]);
/// ```
pub fn parse_numeric_array<T>(input: &str) -> Result<Vec<T>, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    if input.is_empty() {
        return Ok(Vec::new());
    }

    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<T>()
                .map_err(|e| format!("Failed to parse '{}': {}", s, e))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_array_basic() {
        let result = parse_array("a,b,c");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_array_with_spaces() {
        let result = parse_array("a, b , c");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_array_empty() {
        let result: Vec<&str> = parse_array("");
        assert_eq!(result, Vec::<&str>::new());
    }

    #[test]
    fn test_parse_array_single() {
        let result = parse_array("single");
        assert_eq!(result, vec!["single"]);
    }

    #[test]
    fn test_parse_array_with_empty_elements() {
        let result = parse_array("a,,b");
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_parse_numeric_array_integers() {
        let result = parse_numeric_array::<i32>("1,2,3,4,5").unwrap();
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parse_numeric_array_floats() {
        let result = parse_numeric_array::<f64>("1.5,2.5,3.5").unwrap();
        assert_eq!(result, vec![1.5, 2.5, 3.5]);
    }

    #[test]
    fn test_parse_numeric_array_with_spaces() {
        let result = parse_numeric_array::<i32>("1, 2 , 3").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_numeric_array_empty() {
        let result = parse_numeric_array::<i32>("").unwrap();
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_parse_numeric_array_invalid() {
        let result = parse_numeric_array::<i32>("1,abc,3");
        assert!(result.is_err());
    }
}
