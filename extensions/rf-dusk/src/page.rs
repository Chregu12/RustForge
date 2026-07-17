//! Page Object pattern support

use crate::{Browser, DuskResult};
use async_trait::async_trait;

/// Page trait for Page Object pattern
#[async_trait]
pub trait Page: Send + Sync {
    /// Get the URL path for this page
    fn url(&self) -> &str;

    /// Navigate to this page
    async fn visit(&self, browser: &Browser) -> DuskResult<()> {
        browser.visit(self.url()).await?;
        Ok(())
    }

    /// Assert we are on this page
    async fn assert_on_page(&self, browser: &Browser) -> DuskResult<()> {
        browser.assert_path_is(self.url()).await?;
        Ok(())
    }
}

/// Login page example
pub struct LoginPage;

impl LoginPage {
    pub fn new() -> Self {
        Self
    }

    /// Fill login form
    pub async fn login(&self, browser: &Browser, email: &str, password: &str) -> DuskResult<()> {
        browser
            .type_text("#email", email)
            .await?
            .type_text("#password", password)
            .await?
            .click("button[type=submit]")
            .await?;
        Ok(())
    }
}

impl Default for LoginPage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Page for LoginPage {
    fn url(&self) -> &str {
        "/login"
    }
}

/// Dashboard page example
pub struct DashboardPage;

impl DashboardPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DashboardPage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Page for DashboardPage {
    fn url(&self) -> &str {
        "/dashboard"
    }
}

/// Component trait for reusable page components
#[async_trait]
pub trait Component: Send + Sync {
    /// Get the root selector for this component
    fn selector(&self) -> &str;

    /// Assert component is visible
    async fn assert_visible(&self, browser: &Browser) -> DuskResult<()> {
        browser.assert_visible(self.selector()).await?;
        Ok(())
    }
}

/// Navigation component example
pub struct NavigationComponent {
    selector: String,
}

impl NavigationComponent {
    pub fn new() -> Self {
        Self {
            selector: "nav".to_string(),
        }
    }

    pub fn with_selector(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
        }
    }

    /// Click a navigation link
    pub async fn click_link(&self, browser: &Browser, text: &str) -> DuskResult<()> {
        let selector = format!("{} a:contains('{}')", self.selector, text);
        browser.click(&selector).await?;
        Ok(())
    }
}

impl Default for NavigationComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Component for NavigationComponent {
    fn selector(&self) -> &str {
        &self.selector
    }
}
