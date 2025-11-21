/// Unit Tests
///
/// Unit tests test small, isolated pieces of code,
/// typically individual functions or methods.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        // Example unit test
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_string_concatenation() {
        let result = format!("{} {}", "Hello", "World");
        assert_eq!(result, "Hello World");
    }

    #[tokio::test]
    async fn test_async_function() {
        // Example async unit test
        let result = async_example().await;
        assert!(result.is_ok());
    }
}

async fn async_example() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
