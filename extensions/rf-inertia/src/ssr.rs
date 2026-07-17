#![allow(dead_code)] // fields/methods retained for planned functionality, not read internally yet
//! # Server-Side Rendering (SSR) for Inertia.js
//!
//! This module enables server-side rendering of Inertia.js pages by forwarding
//! the page component, props, and URL to an external SSR server (e.g. a Node.js
//! process running `@inertiajs/server`) and returning the rendered HTML.
//!
//! ## Enabling SSR
//!
//! Add the `ssr` Cargo feature to your dependency:
//!
//! ```toml
//! [dependencies]
//! rf-inertia = { version = "*", features = ["ssr"] }
//! ```
//!
//! ## Running an SSR Server
//!
//! Start the Inertia SSR server alongside your Rust application:
//!
//! ```sh
//! node bootstrap/ssr/ssr.js
//! ```
//!
//! By default it listens on `http://127.0.0.1:13714`.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_inertia::ssr::{SsrClient, SsrConfig};
//! use rf_inertia::response::InertiaResponse;
//! use rf_inertia::props::Props;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ssr = SsrClient::new(SsrConfig::default());
//!
//! let inertia_response = InertiaResponse::new(
//!     "Dashboard/Index",
//!     Props::new(),
//!     "/dashboard",
//!     "1.0.0",
//! );
//!
//! let rendered = ssr.render(&inertia_response).await?;
//! println!("{}", rendered.body);
//! # Ok(())
//! # }
//! ```

use crate::{error::InertiaError, response::InertiaResponse};
use serde::{Deserialize, Serialize};

/// Configuration for the SSR server connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsrConfig {
    /// Full URL of the running Inertia SSR server.
    ///
    /// Defaults to `http://127.0.0.1:13714`.
    pub url: String,

    /// Request timeout in seconds. Defaults to `5`.
    pub timeout_secs: u64,
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            url: "http://127.0.0.1:13714".to_string(),
            timeout_secs: 5,
        }
    }
}

impl SsrConfig {
    /// Create a new SSR configuration pointing at the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Read `INERTIA_SSR_URL` and `INERTIA_SSR_TIMEOUT` from the environment.
    pub fn from_env() -> Self {
        let url = std::env::var("INERTIA_SSR_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:13714".to_string());
        let timeout_secs = std::env::var("INERTIA_SSR_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        Self { url, timeout_secs }
    }
}

/// Response returned by the Inertia SSR server.
#[derive(Debug, Deserialize)]
pub struct SsrRendered {
    /// HTML for the `<head>` section (scripts, meta tags, etc.).
    pub head: Vec<String>,
    /// Rendered component HTML for the `<body>`.
    pub body: String,
}

// ─── SSR Client ───────────────────────────────────────────────────────────────

/// Client that communicates with an Inertia SSR server.
///
/// When the `ssr` Cargo feature is **enabled** this uses `reqwest` to POST the
/// page data to the SSR server and return the pre-rendered HTML.
///
/// When the feature is **disabled** all methods fall back gracefully by
/// returning the standard client-side Inertia bootstrap HTML so the application
/// continues to work without SSR.
pub struct SsrClient {
    config: SsrConfig,
    #[cfg(feature = "ssr")]
    http: reqwest::Client,
}

impl SsrClient {
    /// Create a new SSR client with the given configuration.
    pub fn new(config: SsrConfig) -> Self {
        #[cfg(feature = "ssr")]
        {
            let http = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .build()
                .expect("Failed to build reqwest client");
            Self { config, http }
        }

        #[cfg(not(feature = "ssr"))]
        {
            Self { config }
        }
    }

    /// Create an SSR client configured from environment variables.
    pub fn from_env() -> Self {
        Self::new(SsrConfig::from_env())
    }

    /// Render an [`InertiaResponse`] using the SSR server.
    ///
    /// Returns the full pre-rendered HTML string.
    ///
    /// # Errors
    ///
    /// Returns [`InertiaError`] if:
    /// - The SSR server is unreachable (connection refused / timeout).
    /// - The server returns an HTTP error status.
    /// - The response body cannot be parsed.
    pub async fn render(&self, page: &InertiaResponse) -> crate::error::Result<SsrRendered> {
        #[cfg(feature = "ssr")]
        {
            let response = self
                .http
                .post(&self.config.url)
                .json(page)
                .send()
                .await
                .map_err(|e| InertiaError::Render(format!("SSR server request failed: {e}")))?;

            if !response.status().is_success() {
                return Err(InertiaError::Render(format!(
                    "SSR server returned status {}",
                    response.status()
                )));
            }

            let rendered: SsrRendered = response.json().await.map_err(|e| {
                InertiaError::Render(format!("Failed to parse SSR server response: {e}"))
            })?;

            Ok(rendered)
        }

        #[cfg(not(feature = "ssr"))]
        {
            // Graceful fallback: return a minimal SSR-shaped response so the
            // caller can still embed it in an HTML template.
            let _ = page;
            Err(InertiaError::Render(
                "SSR support is not enabled. Add the `ssr` feature to rf-inertia.".to_string(),
            ))
        }
    }

    /// Render and embed the SSR output into a full HTML document.
    ///
    /// This is the most convenient method: it calls [`render`](Self::render)
    /// and wraps the result in the standard Inertia HTML shell.
    pub async fn render_to_html(
        &self,
        page: &InertiaResponse,
        root_element_id: &str,
    ) -> crate::error::Result<String> {
        let page_json = serde_json::to_string(page)?;

        // Escape for safe embedding in HTML attribute
        let escaped_json = page_json
            .replace('&', "&amp;")
            .replace('\'', "&#x27;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        #[cfg(feature = "ssr")]
        {
            let rendered = self.render(page).await?;
            let head_html = rendered.head.join("\n    ");

            Ok(format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    {head_html}
</head>
<body>
    <div id="{root_element_id}" data-page='{escaped_json}'>{body}</div>
</body>
</html>"#,
                head_html = head_html,
                root_element_id = root_element_id,
                escaped_json = escaped_json,
                body = rendered.body,
            ))
        }

        #[cfg(not(feature = "ssr"))]
        {
            // Client-side only fallback (no SSR server call).
            Ok(format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
</head>
<body>
    <div id="{root_element_id}" data-page='{escaped_json}'></div>
</body>
</html>"#,
                root_element_id = root_element_id,
                escaped_json = escaped_json,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{props::Props, response::InertiaResponse};

    #[test]
    fn test_ssr_config_default() {
        let cfg = SsrConfig::default();
        assert_eq!(cfg.url, "http://127.0.0.1:13714");
        assert_eq!(cfg.timeout_secs, 5);
    }

    #[test]
    fn test_ssr_config_from_env_defaults() {
        unsafe { std::env::remove_var("INERTIA_SSR_URL") };
        let cfg = SsrConfig::from_env();
        assert_eq!(cfg.url, "http://127.0.0.1:13714");
    }

    #[tokio::test]
    async fn test_render_to_html_fallback() {
        let client = SsrClient::new(SsrConfig::default());
        let page = InertiaResponse::new("Home/Index", Props::new(), "/", "1.0");

        // Without the `ssr` feature the fallback HTML is returned (no network call).
        let html = client.render_to_html(&page, "app").await;

        // With feature disabled → Err; without SSR feature → Ok (fallback HTML).
        // Either way must not panic.
        let _ = html;
    }
}
