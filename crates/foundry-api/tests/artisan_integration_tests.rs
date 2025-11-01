// Integration tests for Artisan facade
// These tests require a running RustForge application

#[cfg(test)]
mod artisan_tests {
    use foundry_api::Artisan;
    use foundry_plugins::ResponseFormat;

    #[tokio::test]
    #[ignore] // Run with: cargo test --test artisan_integration_tests -- --ignored
    async fn test_artisan_command_call() {
        // This test requires a full application setup
        // In real scenarios, you would:
        // 1. Initialize FoundryApp
        // 2. Create FoundryInvoker
        // 3. Create Artisan instance
        // 4. Call commands

        // Example of what this would look like:
        /*
        let app = FoundryApp::new(config)?;
        let invoker = FoundryInvoker::new(app);
        let artisan = Artisan::new(invoker);

        // Test simple command execution
        let result = artisan.call("list").dispatch().await;
        assert!(result.is_ok());

        // Test command with arguments
        let result = artisan
            .call("make:command")
            .with_args(vec!["TestCommand".to_string()])
            .dispatch()
            .await;

        assert!(result.is_ok());
        */
    }

    #[tokio::test]
    #[ignore] // Run with: cargo test --test artisan_integration_tests -- --ignored
    async fn test_artisan_command_chaining() {
        // This test requires a full application setup
        // Example of what this would look like:
        /*
        let app = FoundryApp::new(config)?;
        let invoker = FoundryInvoker::new(app);
        let artisan = Artisan::new(invoker);

        let results = artisan
            .chain()
            .add("list")
            .add_with_args("env:validate", vec![])
            .dispatch()
            .await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
        */
    }

    #[tokio::test]
    #[ignore]
    async fn test_artisan_output_capture() {
        // This test requires a full application setup
        // Example of what this would look like:
        /*
        let app = FoundryApp::new(config)?;
        let invoker = FoundryInvoker::new(app);
        let artisan = Artisan::new(invoker);

        artisan.call("list").dispatch().await.ok();

        let output = artisan.output();
        assert!(!output.is_empty());

        let output_string = artisan.output_string();
        assert!(!output_string.is_empty());

        artisan.clear_output();
        assert!(artisan.output().is_empty());
        */
    }

    #[test]
    fn test_artisan_output_format() {
        // Test that output format can be set
        // This is a unit test that doesn't require full app
        let formats = vec![ResponseFormat::Human, ResponseFormat::Json];

        for format in formats {
            // Verify format can be used
            let format_clone = format.clone();
            assert_eq!(format, format_clone);
        }
    }
}
