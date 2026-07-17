//! Wait helpers for browser tests

use crate::{Browser, DuskError, DuskResult};
use std::time::Duration;
use tokio::time::sleep;

/// Wait condition trait
pub trait WaitCondition: Send + Sync {
    /// Check if the condition is met
    fn check(&self) -> impl std::future::Future<Output = bool> + Send;

    /// Description of what we're waiting for
    fn description(&self) -> String;
}

/// Wait for a condition with timeout
pub async fn wait_until<F, Fut>(
    condition: F,
    timeout: Duration,
    description: &str,
) -> DuskResult<()>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        if condition().await {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }

    Err(DuskError::Timeout(format!(
        "Timeout waiting for: {}",
        description
    )))
}

/// Wait helpers
pub struct Wait<'a> {
    browser: &'a Browser,
    timeout: Duration,
    poll_interval: Duration,
}

impl<'a> Wait<'a> {
    pub fn new(browser: &'a Browser) -> Self {
        Self {
            browser,
            timeout: Duration::from_secs(30),
            poll_interval: Duration::from_millis(100),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Wait until element exists
    pub async fn until_present(&self, selector: &str) -> DuskResult<()> {
        self.browser
            .wait_for_with_timeout(selector, self.timeout)
            .await?;
        Ok(())
    }

    /// Wait until element does not exist
    pub async fn until_not_present(&self, selector: &str) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            if self.browser.assert_not_present(selector).await.is_ok() {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "Element '{}' still present after {:?}",
            selector, self.timeout
        )))
    }

    /// Wait until text appears
    pub async fn until_text(&self, text: &str) -> DuskResult<()> {
        self.browser.wait_for_text(text).await?;
        Ok(())
    }

    /// Wait until text disappears
    pub async fn until_no_text(&self, text: &str) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            let source = self.browser.source().await?;
            if !source.contains(text) {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "Text '{}' still present after {:?}",
            text, self.timeout
        )))
    }

    /// Wait until URL matches
    pub async fn until_url(&self, expected: &str) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            let url = self.browser.current_url().await?;
            if url == expected || url.ends_with(expected) {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "URL did not match '{}' after {:?}",
            expected, self.timeout
        )))
    }

    /// Wait until URL contains
    pub async fn until_url_contains(&self, text: &str) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            let url = self.browser.current_url().await?;
            if url.contains(text) {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "URL did not contain '{}' after {:?}",
            text, self.timeout
        )))
    }

    /// Wait for page load
    pub async fn until_page_loaded(&self) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            let result = self
                .browser
                .execute_script("return document.readyState")
                .await?;

            if result.as_str() == Some("complete") {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "Page did not load after {:?}",
            self.timeout
        )))
    }

    /// Wait for AJAX requests to complete (jQuery)
    pub async fn until_ajax_complete(&self) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            let result = self
                .browser
                .execute_script("return typeof jQuery !== 'undefined' ? jQuery.active === 0 : true")
                .await?;

            if result.as_bool() == Some(true) {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "AJAX requests did not complete after {:?}",
            self.timeout
        )))
    }

    /// Wait for Vue.js to be ready
    pub async fn until_vue_ready(&self) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            let result = self
                .browser
                .execute_script(
                    "return typeof Vue !== 'undefined' || document.querySelector('[data-v-app]') !== null"
                )
                .await?;

            if result.as_bool() == Some(true) {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "Vue.js did not become ready after {:?}",
            self.timeout
        )))
    }

    /// Wait for Livewire to be ready
    pub async fn until_livewire_ready(&self) -> DuskResult<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            let result = self
                .browser
                .execute_script("return typeof window.Livewire !== 'undefined'")
                .await?;

            if result.as_bool() == Some(true) {
                return Ok(());
            }
            sleep(self.poll_interval).await;
        }

        Err(DuskError::Timeout(format!(
            "Livewire did not become ready after {:?}",
            self.timeout
        )))
    }
}

/// Extension trait for Browser to get Wait helpers
pub trait BrowserWaitExt {
    fn wait(&self) -> Wait<'_>;
}

impl BrowserWaitExt for Browser {
    fn wait(&self) -> Wait<'_> {
        Wait::new(self)
    }
}
