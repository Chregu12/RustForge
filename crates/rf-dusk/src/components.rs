//! Reusable browser test components

use crate::{Browser, DuskResult};
use async_trait::async_trait;

/// Modal component helper
pub struct Modal {
    selector: String,
}

impl Modal {
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
        }
    }

    pub fn default_modal() -> Self {
        Self::new(".modal")
    }

    /// Wait for modal to be visible
    pub async fn wait_for_open(&self, browser: &Browser) -> DuskResult<&Self> {
        browser.wait_for(&self.selector).await?;
        Ok(self)
    }

    /// Close the modal
    pub async fn close(&self, browser: &Browser) -> DuskResult<&Self> {
        let close_btn = format!("{} .close, {} [data-dismiss=modal]", self.selector, self.selector);
        browser.click(&close_btn).await?;
        Ok(self)
    }

    /// Assert modal is visible
    pub async fn assert_visible(&self, browser: &Browser) -> DuskResult<&Self> {
        browser.assert_visible(&self.selector).await?;
        Ok(self)
    }

    /// Assert modal is closed
    pub async fn assert_closed(&self, browser: &Browser) -> DuskResult<&Self> {
        browser.assert_not_present(&self.selector).await?;
        Ok(self)
    }
}

/// Dropdown component helper
pub struct Dropdown {
    trigger_selector: String,
    menu_selector: String,
}

impl Dropdown {
    pub fn new(trigger: impl Into<String>, menu: impl Into<String>) -> Self {
        Self {
            trigger_selector: trigger.into(),
            menu_selector: menu.into(),
        }
    }

    /// Open the dropdown
    pub async fn open(&self, browser: &Browser) -> DuskResult<&Self> {
        browser.click(&self.trigger_selector).await?;
        browser.wait_for(&self.menu_selector).await?;
        Ok(self)
    }

    /// Select an option
    pub async fn select(&self, browser: &Browser, option: &str) -> DuskResult<&Self> {
        let option_selector = format!("{} [data-value='{}'], {} li:contains('{}')",
            self.menu_selector, option, self.menu_selector, option);
        browser.click(&option_selector).await?;
        Ok(self)
    }

    /// Assert dropdown is open
    pub async fn assert_open(&self, browser: &Browser) -> DuskResult<&Self> {
        browser.assert_visible(&self.menu_selector).await?;
        Ok(self)
    }
}

/// Form component helper
pub struct Form {
    selector: String,
}

impl Form {
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
        }
    }

    /// Fill a text input
    pub async fn fill(&self, browser: &Browser, name: &str, value: &str) -> DuskResult<&Self> {
        let input_selector = format!("{} [name='{}'], {} #{}",
            self.selector, name, self.selector, name);
        browser.clear_and_type(&input_selector, value).await?;
        Ok(self)
    }

    /// Select an option
    pub async fn select(&self, browser: &Browser, name: &str, value: &str) -> DuskResult<&Self> {
        let select_selector = format!("{} select[name='{}']", self.selector, name);
        browser.select(&select_selector, value).await?;
        Ok(self)
    }

    /// Check a checkbox
    pub async fn check(&self, browser: &Browser, name: &str) -> DuskResult<&Self> {
        let checkbox_selector = format!("{} [name='{}'][type=checkbox]", self.selector, name);
        browser.check(&checkbox_selector).await?;
        Ok(self)
    }

    /// Submit the form
    pub async fn submit(&self, browser: &Browser) -> DuskResult<&Self> {
        let submit_selector = format!("{} [type=submit], {} button:not([type])",
            self.selector, self.selector);
        browser.click(&submit_selector).await?;
        Ok(self)
    }

    /// Assert form has error
    pub async fn assert_has_error(&self, browser: &Browser, field: &str) -> DuskResult<&Self> {
        let error_selector = format!(
            "{} .error[data-field='{}'], {} [name='{}'] ~ .error, {} .field-error.{}",
            self.selector, field, self.selector, field, self.selector, field
        );
        browser.assert_visible(&error_selector).await?;
        Ok(self)
    }
}

/// Table component helper
pub struct Table {
    selector: String,
}

impl Table {
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
        }
    }

    /// Get row count (via JavaScript)
    pub async fn row_count(&self, browser: &Browser) -> DuskResult<usize> {
        let script = format!(
            "return document.querySelectorAll('{} tbody tr').length",
            self.selector
        );
        let result = browser.execute_script(&script).await?;
        Ok(result.as_u64().unwrap_or(0) as usize)
    }

    /// Assert row count
    pub async fn assert_row_count(&self, browser: &Browser, expected: usize) -> DuskResult<&Self> {
        let count = self.row_count(browser).await?;
        if count != expected {
            return Err(crate::DuskError::AssertionFailed(format!(
                "Expected {} rows, found {}",
                expected, count
            )));
        }
        Ok(self)
    }

    /// Click row action
    pub async fn click_row_action(&self, browser: &Browser, row: usize, action: &str) -> DuskResult<&Self> {
        let action_selector = format!(
            "{} tbody tr:nth-child({}) [data-action='{}'], {} tbody tr:nth-child({}) .action-{}",
            self.selector, row, action, self.selector, row, action
        );
        browser.click(&action_selector).await?;
        Ok(self)
    }

    /// Assert table contains text
    pub async fn assert_contains(&self, browser: &Browser, text: &str) -> DuskResult<&Self> {
        let source = browser.source().await?;
        if !source.contains(text) {
            return Err(crate::DuskError::AssertionFailed(format!(
                "Table does not contain text '{}'",
                text
            )));
        }
        Ok(self)
    }
}

/// Alert/Toast component helper
pub struct Alert {
    selector: String,
}

impl Alert {
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
        }
    }

    pub fn success() -> Self {
        Self::new(".alert-success, .toast-success")
    }

    pub fn error() -> Self {
        Self::new(".alert-error, .alert-danger, .toast-error")
    }

    pub fn warning() -> Self {
        Self::new(".alert-warning, .toast-warning")
    }

    /// Wait for alert to appear
    pub async fn wait_for(&self, browser: &Browser) -> DuskResult<&Self> {
        browser.wait_for(&self.selector).await?;
        Ok(self)
    }

    /// Assert alert is visible
    pub async fn assert_visible(&self, browser: &Browser) -> DuskResult<&Self> {
        browser.assert_visible(&self.selector).await?;
        Ok(self)
    }

    /// Assert alert contains text
    pub async fn assert_contains(&self, browser: &Browser, text: &str) -> DuskResult<&Self> {
        browser.wait_for(&self.selector).await?;
        let source = browser.source().await?;
        if !source.contains(text) {
            return Err(crate::DuskError::AssertionFailed(format!(
                "Alert does not contain text '{}'",
                text
            )));
        }
        Ok(self)
    }

    /// Dismiss the alert
    pub async fn dismiss(&self, browser: &Browser) -> DuskResult<&Self> {
        let close_btn = format!("{} .close, {} [data-dismiss]", self.selector, self.selector);
        if browser.wait_for(&close_btn).await.is_ok() {
            browser.click(&close_btn).await?;
        }
        Ok(self)
    }
}
