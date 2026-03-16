//! Browser control and automation

use crate::{DuskError, DuskResult, Element};
use fantoccini::{Client, ClientBuilder, Locator};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

/// Browser types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
}

impl Default for BrowserType {
    fn default() -> Self {
        Self::Chrome
    }
}

/// Browser configuration
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub browser_type: BrowserType,
    pub headless: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub timeout: Duration,
    pub webdriver_url: String,
    pub base_url: String,
    pub screenshot_on_failure: bool,
    pub screenshot_dir: String,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            browser_type: BrowserType::Chrome,
            headless: true,
            window_width: 1920,
            window_height: 1080,
            timeout: Duration::from_secs(30),
            webdriver_url: "http://localhost:4444".to_string(),
            base_url: "http://localhost:8000".to_string(),
            screenshot_on_failure: true,
            screenshot_dir: "tests/Browser/screenshots".to_string(),
        }
    }
}

impl BrowserConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn browser_type(mut self, browser_type: BrowserType) -> Self {
        self.browser_type = browser_type;
        self
    }

    pub fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.window_width = width;
        self.window_height = height;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn webdriver_url(mut self, url: impl Into<String>) -> Self {
        self.webdriver_url = url.into();
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn screenshot_on_failure(mut self, enabled: bool) -> Self {
        self.screenshot_on_failure = enabled;
        self
    }

    pub fn screenshot_dir(mut self, dir: impl Into<String>) -> Self {
        self.screenshot_dir = dir.into();
        self
    }
}

/// Main browser automation struct
pub struct Browser {
    client: Arc<RwLock<Option<Client>>>,
    config: BrowserConfig,
}

impl Browser {
    /// Create a new browser with default configuration
    pub async fn new() -> DuskResult<Self> {
        Self::with_config(BrowserConfig::default()).await
    }

    /// Create a new browser with custom configuration
    pub async fn with_config(config: BrowserConfig) -> DuskResult<Self> {
        let mut caps = serde_json::Map::new();

        match config.browser_type {
            BrowserType::Chrome => {
                let mut chrome_opts = serde_json::Map::new();
                let mut args = vec![];

                if config.headless {
                    args.push("--headless");
                    args.push("--disable-gpu");
                }

                args.push("--no-sandbox");
                args.push("--disable-dev-shm-usage");

                chrome_opts.insert(
                    "args".to_string(),
                    serde_json::Value::Array(
                        args.into_iter()
                            .map(|s| serde_json::Value::String(s.to_string()))
                            .collect(),
                    ),
                );

                caps.insert(
                    "goog:chromeOptions".to_string(),
                    serde_json::Value::Object(chrome_opts),
                );
            }
            BrowserType::Firefox => {
                let mut firefox_opts = serde_json::Map::new();
                let mut args = vec![];

                if config.headless {
                    args.push("-headless");
                }

                firefox_opts.insert(
                    "args".to_string(),
                    serde_json::Value::Array(
                        args.into_iter()
                            .map(|s| serde_json::Value::String(s.to_string()))
                            .collect(),
                    ),
                );

                caps.insert(
                    "moz:firefoxOptions".to_string(),
                    serde_json::Value::Object(firefox_opts),
                );
            }
            _ => {}
        }

        let client = ClientBuilder::native()
            .capabilities(caps)
            .connect(&config.webdriver_url)
            .await
            .map_err(|e| DuskError::WebDriverError(e.to_string()))?;

        Ok(Self {
            client: Arc::new(RwLock::new(Some(client))),
            config,
        })
    }

    /// Get client reference
    async fn with_client<F, T>(&self, f: F) -> DuskResult<T>
    where
        F: FnOnce(&Client) -> futures::future::BoxFuture<'_, DuskResult<T>>,
    {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;
        f(client).await
    }

    /// Navigate to a URL
    pub async fn visit(&self, url: &str) -> DuskResult<&Self> {
        let full_url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{}", self.config.base_url, url)
        };

        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .goto(&full_url)
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        Ok(self)
    }

    /// Click on an element
    pub async fn click(&self, selector: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let element = client
            .find(Locator::Css(selector))
            .await
            .map_err(|_| DuskError::ElementNotFound(selector.to_string()))?;

        element
            .click()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        Ok(self)
    }

    /// Type text into an element
    pub async fn type_text(&self, selector: &str, text: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let element = client
            .find(Locator::Css(selector))
            .await
            .map_err(|_| DuskError::ElementNotFound(selector.to_string()))?;

        element
            .send_keys(text)
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        Ok(self)
    }

    /// Clear and type text into an element
    pub async fn clear_and_type(&self, selector: &str, text: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let element = client
            .find(Locator::Css(selector))
            .await
            .map_err(|_| DuskError::ElementNotFound(selector.to_string()))?;

        element
            .clear()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        element
            .send_keys(text)
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        Ok(self)
    }

    /// Select an option from a dropdown
    pub async fn select(&self, selector: &str, value: &str) -> DuskResult<&Self> {
        // Escape special CSS characters in value to prevent selector injection
        let escaped_value = value.replace('\\', "\\\\").replace('"', "\\\"");
        let option_selector = format!("{} option[value=\"{}\"]", selector, escaped_value);
        self.click(&option_selector).await
    }

    /// Check a checkbox
    pub async fn check(&self, selector: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let element = client
            .find(Locator::Css(selector))
            .await
            .map_err(|_| DuskError::ElementNotFound(selector.to_string()))?;

        let is_checked = element
            .prop("checked")
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?
            .map(|v| v == "true")
            .unwrap_or(false);

        if !is_checked {
            element
                .click()
                .await
                .map_err(|e| DuskError::BrowserError(e.to_string()))?;
        }

        Ok(self)
    }

    /// Uncheck a checkbox
    pub async fn uncheck(&self, selector: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let element = client
            .find(Locator::Css(selector))
            .await
            .map_err(|_| DuskError::ElementNotFound(selector.to_string()))?;

        let is_checked = element
            .prop("checked")
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?
            .map(|v| v == "true")
            .unwrap_or(false);

        if is_checked {
            element
                .click()
                .await
                .map_err(|e| DuskError::BrowserError(e.to_string()))?;
        }

        Ok(self)
    }

    /// Press Enter key
    pub async fn press_enter(&self, selector: &str) -> DuskResult<&Self> {
        self.type_text(selector, "\u{E007}").await
    }

    /// Wait for an element to be present
    pub async fn wait_for(&self, selector: &str) -> DuskResult<&Self> {
        self.wait_for_with_timeout(selector, self.config.timeout)
            .await
    }

    /// Wait for an element with custom timeout
    pub async fn wait_for_with_timeout(
        &self,
        selector: &str,
        timeout: Duration,
    ) -> DuskResult<&Self> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            let guard = self.client.read().await;
            let client = guard
                .as_ref()
                .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

            if client.find(Locator::Css(selector)).await.is_ok() {
                return Ok(self);
            }

            drop(guard);
            sleep(Duration::from_millis(100)).await;
        }

        Err(DuskError::Timeout(format!(
            "Element '{}' not found after {:?}",
            selector, timeout
        )))
    }

    /// Wait for text to appear on the page
    pub async fn wait_for_text(&self, text: &str) -> DuskResult<&Self> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.config.timeout {
            let guard = self.client.read().await;
            let client = guard
                .as_ref()
                .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

            let source = client
                .source()
                .await
                .map_err(|e| DuskError::BrowserError(e.to_string()))?;

            if source.contains(text) {
                return Ok(self);
            }

            drop(guard);
            sleep(Duration::from_millis(100)).await;
        }

        Err(DuskError::Timeout(format!(
            "Text '{}' not found after {:?}",
            text, self.config.timeout
        )))
    }

    /// Pause execution for a duration
    pub async fn pause(&self, duration: Duration) -> DuskResult<&Self> {
        sleep(duration).await;
        Ok(self)
    }

    /// Get the current URL
    pub async fn current_url(&self) -> DuskResult<String> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .current_url()
            .await
            .map(|u| u.to_string())
            .map_err(|e| DuskError::BrowserError(e.to_string()))
    }

    /// Get the page title
    pub async fn title(&self) -> DuskResult<String> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .title()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))
    }

    /// Get the page source
    pub async fn source(&self) -> DuskResult<String> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .source()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))
    }

    /// Take a screenshot
    pub async fn screenshot(&self, name: &str) -> DuskResult<Vec<u8>> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let screenshot = client
            .screenshot()
            .await
            .map_err(|e| DuskError::ScreenshotError(e.to_string()))?;

        // Optionally save to file
        let path = format!("{}/{}.png", self.config.screenshot_dir, name);
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &screenshot)?;

        Ok(screenshot)
    }

    /// Execute JavaScript
    pub async fn execute_script(&self, script: &str) -> DuskResult<serde_json::Value> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .execute(script, vec![])
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))
    }

    /// Scroll to an element
    pub async fn scroll_to(&self, selector: &str) -> DuskResult<&Self> {
        self.execute_script(&format!(
            "document.querySelector('{}').scrollIntoView({{behavior: 'smooth', block: 'center'}})",
            selector
        ))
        .await?;
        Ok(self)
    }

    /// Scroll to top
    pub async fn scroll_to_top(&self) -> DuskResult<&Self> {
        self.execute_script("window.scrollTo(0, 0)").await?;
        Ok(self)
    }

    /// Scroll to bottom
    pub async fn scroll_to_bottom(&self) -> DuskResult<&Self> {
        self.execute_script("window.scrollTo(0, document.body.scrollHeight)")
            .await?;
        Ok(self)
    }

    /// Go back in browser history
    pub async fn back(&self) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .back()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        Ok(self)
    }

    /// Go forward in browser history
    pub async fn forward(&self) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .forward()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        Ok(self)
    }

    /// Refresh the current page
    pub async fn refresh(&self) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        client
            .refresh()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        Ok(self)
    }

    /// Assert that the current path matches
    pub async fn assert_path_is(&self, path: &str) -> DuskResult<&Self> {
        let url = self.current_url().await?;
        // Extract path from URL
        let current_path = url
            .split('?')
            .next()
            .and_then(|s| s.split("://").nth(1))
            .and_then(|s| s.find('/').map(|i| &s[i..]))
            .unwrap_or("/")
            .to_string();

        if current_path != path {
            return Err(DuskError::AssertionFailed(format!(
                "Expected path '{}', got '{}'",
                path, current_path
            )));
        }

        Ok(self)
    }

    /// Assert that text is visible on the page
    pub async fn assert_see(&self, text: &str) -> DuskResult<&Self> {
        let source = self.source().await?;
        if !source.contains(text) {
            return Err(DuskError::AssertionFailed(format!(
                "Text '{}' not found on page",
                text
            )));
        }
        Ok(self)
    }

    /// Assert that text is NOT visible on the page
    pub async fn assert_dont_see(&self, text: &str) -> DuskResult<&Self> {
        let source = self.source().await?;
        if source.contains(text) {
            return Err(DuskError::AssertionFailed(format!(
                "Text '{}' was found on page but should not be",
                text
            )));
        }
        Ok(self)
    }

    /// Assert that an element is visible
    pub async fn assert_visible(&self, selector: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let element = client
            .find(Locator::Css(selector))
            .await
            .map_err(|_| {
                DuskError::AssertionFailed(format!("Element '{}' not found", selector))
            })?;

        let is_displayed = element
            .is_displayed()
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?;

        if !is_displayed {
            return Err(DuskError::AssertionFailed(format!(
                "Element '{}' is not visible",
                selector
            )));
        }

        Ok(self)
    }

    /// Assert that an element is NOT present
    pub async fn assert_not_present(&self, selector: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        if client.find(Locator::Css(selector)).await.is_ok() {
            return Err(DuskError::AssertionFailed(format!(
                "Element '{}' is present but should not be",
                selector
            )));
        }

        Ok(self)
    }

    /// Assert input has a value
    pub async fn assert_input_value(&self, selector: &str, expected: &str) -> DuskResult<&Self> {
        let guard = self.client.read().await;
        let client = guard
            .as_ref()
            .ok_or_else(|| DuskError::BrowserError("Browser closed".to_string()))?;

        let element = client
            .find(Locator::Css(selector))
            .await
            .map_err(|_| DuskError::ElementNotFound(selector.to_string()))?;

        let value = element
            .prop("value")
            .await
            .map_err(|e| DuskError::BrowserError(e.to_string()))?
            .unwrap_or_default();

        if value != expected {
            return Err(DuskError::AssertionFailed(format!(
                "Expected input value '{}', got '{}'",
                expected, value
            )));
        }

        Ok(self)
    }

    /// Assert title matches
    pub async fn assert_title(&self, expected: &str) -> DuskResult<&Self> {
        let title = self.title().await?;
        if title != expected {
            return Err(DuskError::AssertionFailed(format!(
                "Expected title '{}', got '{}'",
                expected, title
            )));
        }
        Ok(self)
    }

    /// Assert title contains text
    pub async fn assert_title_contains(&self, text: &str) -> DuskResult<&Self> {
        let title = self.title().await?;
        if !title.contains(text) {
            return Err(DuskError::AssertionFailed(format!(
                "Title '{}' does not contain '{}'",
                title, text
            )));
        }
        Ok(self)
    }

    /// Quit the browser
    pub async fn quit(&self) -> DuskResult<()> {
        let mut guard = self.client.write().await;
        if let Some(client) = guard.take() {
            client
                .close()
                .await
                .map_err(|e| DuskError::BrowserError(e.to_string()))?;
        }
        Ok(())
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        // Note: Async drop not supported, user should call quit() explicitly
    }
}
