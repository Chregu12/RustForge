//! Browser Testing Framework for RustForge
//!
//! This crate provides Laravel Dusk-like browser testing capabilities for Rust applications.
//! It uses WebDriver (via fantoccini) to control browsers and perform end-to-end tests.
//!
//! # Features
//!
//! - **Browser Automation**: Control Chrome, Firefox, Safari via WebDriver
//! - **Fluent API**: Chain browser actions in a readable way
//! - **Page Objects**: Organize tests with page object pattern
//! - **Screenshots**: Capture screenshots on failure or on demand
//! - **Wait Helpers**: Smart waiting for elements and conditions
//! - **Form Interactions**: Easy form filling, selecting, clicking
//! - **Assertions**: Built-in assertions for common checks
//!
//! # Quick Start
//!
//! ```ignore
//! use rf_dusk::{Browser, DuskTestCase};
//!
//! #[tokio::test]
//! async fn test_login() -> DuskResult<()> {
//!     let browser = Browser::new().await?;
//!
//!     browser
//!         .visit("http://localhost:8000/login")
//!         .await?
//!         .type_text("#email", "user@example.com")
//!         .await?
//!         .type_text("#password", "secret")
//!         .await?
//!         .click("button[type=submit]")
//!         .await?
//!         .assert_path_is("/dashboard")
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Browser Configuration
//!
//! ```ignore
//! use rf_dusk::{Browser, BrowserConfig, BrowserType};
//!
//! let config = BrowserConfig::new()
//!     .browser_type(BrowserType::Chrome)
//!     .headless(true)
//!     .window_size(1920, 1080)
//!     .timeout(Duration::from_secs(30));
//!
//! let browser = Browser::with_config(config).await?;
//! ```

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

pub mod assertions;
pub mod browser;
pub mod components;
pub mod element;
pub mod page;
pub mod screenshot;
pub mod waits;

pub use browser::{Browser, BrowserConfig, BrowserType};
pub use element::Element;
pub use page::Page;
pub use screenshot::Screenshot;

/// Dusk error types
#[derive(Debug, Error)]
pub enum DuskError {
    #[error("Browser error: {0}")]
    BrowserError(String),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Timeout waiting for: {0}")]
    Timeout(String),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    #[error("Screenshot error: {0}")]
    ScreenshotError(String),

    #[error("WebDriver error: {0}")]
    WebDriverError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type DuskResult<T> = Result<T, DuskError>;

/// Trait for test cases using Dusk
#[async_trait]
pub trait DuskTestCase {
    /// Set up the test (called before each test)
    async fn setup(&mut self) -> DuskResult<()> {
        Ok(())
    }

    /// Tear down the test (called after each test)
    async fn teardown(&mut self) -> DuskResult<()> {
        Ok(())
    }

    /// Get the base URL for the application
    fn base_url(&self) -> &str {
        "http://localhost:8000"
    }

    /// Get screenshot directory
    fn screenshot_dir(&self) -> PathBuf {
        PathBuf::from("tests/Browser/screenshots")
    }
}

/// Browser authentication trait
#[async_trait]
pub trait Authenticatable {
    /// Login as a user
    async fn login_as(&self, browser: &Browser, user_id: impl Into<String> + Send) -> DuskResult<()>;

    /// Logout
    async fn logout(&self, browser: &Browser) -> DuskResult<()>;
}

/// Macro for running browser tests
#[macro_export]
macro_rules! dusk_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> $crate::DuskResult<()> {
            let browser = $crate::Browser::new().await?;
            let result = $body(&browser).await;
            browser.quit().await?;
            result
        }
    };
}

/// Macro for asserting element visibility
#[macro_export]
macro_rules! assert_visible {
    ($browser:expr, $selector:expr) => {
        $browser.assert_visible($selector).await?
    };
}

/// Macro for asserting element is not visible
#[macro_export]
macro_rules! assert_not_visible {
    ($browser:expr, $selector:expr) => {
        $browser.assert_not_present($selector).await?
    };
}

/// Macro for asserting text content
#[macro_export]
macro_rules! assert_see {
    ($browser:expr, $text:expr) => {
        $browser.assert_see($text).await?
    };
}

/// Macro for asserting text is not present
#[macro_export]
macro_rules! assert_dont_see {
    ($browser:expr, $text:expr) => {
        $browser.assert_dont_see($text).await?
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dusk_error_display() {
        let error = DuskError::ElementNotFound("button#submit".to_string());
        assert!(error.to_string().contains("button#submit"));
    }

    #[test]
    fn test_dusk_result() {
        let result: DuskResult<()> = Ok(());
        assert!(result.is_ok());

        let result: DuskResult<()> = Err(DuskError::Timeout("element".to_string()));
        assert!(result.is_err());
    }
}
