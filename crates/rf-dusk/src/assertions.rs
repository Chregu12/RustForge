//! Browser assertions

use crate::{Browser, DuskResult};

/// Assertion helpers for browser tests
pub struct BrowserAssertions<'a> {
    browser: &'a Browser,
}

impl<'a> BrowserAssertions<'a> {
    pub fn new(browser: &'a Browser) -> Self {
        Self { browser }
    }

    /// Assert URL path
    pub async fn path_is(&self, path: &str) -> DuskResult<&Self> {
        self.browser.assert_path_is(path).await?;
        Ok(self)
    }

    /// Assert URL contains
    pub async fn url_contains(&self, text: &str) -> DuskResult<&Self> {
        let url = self.browser.current_url().await?;
        if !url.contains(text) {
            return Err(crate::DuskError::AssertionFailed(format!(
                "URL '{}' does not contain '{}'",
                url, text
            )));
        }
        Ok(self)
    }

    /// Assert page has text
    pub async fn see(&self, text: &str) -> DuskResult<&Self> {
        self.browser.assert_see(text).await?;
        Ok(self)
    }

    /// Assert page does not have text
    pub async fn dont_see(&self, text: &str) -> DuskResult<&Self> {
        self.browser.assert_dont_see(text).await?;
        Ok(self)
    }

    /// Assert element is visible
    pub async fn visible(&self, selector: &str) -> DuskResult<&Self> {
        self.browser.assert_visible(selector).await?;
        Ok(self)
    }

    /// Assert element is not present
    pub async fn not_present(&self, selector: &str) -> DuskResult<&Self> {
        self.browser.assert_not_present(selector).await?;
        Ok(self)
    }

    /// Assert input value
    pub async fn input_value(&self, selector: &str, value: &str) -> DuskResult<&Self> {
        self.browser.assert_input_value(selector, value).await?;
        Ok(self)
    }

    /// Assert page title
    pub async fn title(&self, expected: &str) -> DuskResult<&Self> {
        self.browser.assert_title(expected).await?;
        Ok(self)
    }

    /// Assert page title contains
    pub async fn title_contains(&self, text: &str) -> DuskResult<&Self> {
        self.browser.assert_title_contains(text).await?;
        Ok(self)
    }
}

/// Extension trait for Browser to get assertions
pub trait BrowserAssertionExt {
    fn assertions(&self) -> BrowserAssertions<'_>;
}

impl BrowserAssertionExt for Browser {
    fn assertions(&self) -> BrowserAssertions<'_> {
        BrowserAssertions::new(self)
    }
}
