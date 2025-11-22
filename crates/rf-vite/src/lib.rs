//! # rf-vite - Vite Asset Pipeline Integration
//!
//! Integrates Vite dev server and build tools with RustForge for modern frontend development.
//!
//! ## Features
//!
//! - **Vite Dev Server**: Automatic Vite dev server management
//! - **Hot Module Replacement (HMR)**: Live reload for assets
//! - **Asset Manifest**: Production asset fingerprinting
//! - **Build Integration**: Automatic asset compilation
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_vite::ViteConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // In development
//! let config_dev = ViteConfig::new("./")
//!     .entry("resources/js/app.js")
//!     .entry("resources/css/app.css");
//!
//! let vite = config_dev.dev_server().await?;
//! let script_tag = vite.script("resources/js/app.js")?;
//!
//! // In production
//! let config_prod = ViteConfig::new("./")
//!     .entry("resources/js/app.js")
//!     .entry("resources/css/app.css");
//!
//! let manifest = config_prod.build().await?;
//! let prod_script_tag = manifest.script("resources/js/app.js")?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::fs;
use tokio::process::Command;

/// Vite errors
#[derive(Error, Debug)]
pub enum ViteError {
    #[error("Vite not found: {0}")]
    NotFound(String),

    #[error("Dev server error: {0}")]
    DevServerError(String),

    #[error("Build error: {0}")]
    BuildError(String),

    #[error("Manifest error: {0}")]
    ManifestError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type ViteResult<T> = Result<T, ViteError>;

/// Vite configuration
#[derive(Debug, Clone)]
pub struct ViteConfig {
    /// Project root directory
    root: PathBuf,

    /// Entry points
    entries: Vec<String>,

    /// Dev server port
    port: u16,

    /// Dev server host
    host: String,

    /// Build output directory
    build_dir: PathBuf,

    /// Manifest file name
    manifest: String,
}

impl ViteConfig {
    /// Create a new Vite configuration
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            entries: Vec::new(),
            port: 5173,
            host: "localhost".to_string(),
            build_dir: PathBuf::from("public/build"),
            manifest: "manifest.json".to_string(),
        }
    }

    /// Add an entry point
    pub fn entry(mut self, entry: impl Into<String>) -> Self {
        self.entries.push(entry.into());
        self
    }

    /// Set dev server port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set dev server host
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set build output directory
    pub fn build_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.build_dir = dir.into();
        self
    }

    /// Start Vite dev server
    pub async fn dev_server(self) -> ViteResult<ViteDevServer> {
        ViteDevServer::start(self).await
    }

    /// Build assets for production
    pub async fn build(self) -> ViteResult<ViteManifest> {
        ViteBuild::run(self).await
    }
}

/// Vite dev server
pub struct ViteDevServer {
    config: ViteConfig,
    process: Option<tokio::process::Child>,
}

impl ViteDevServer {
    /// Start the dev server
    pub async fn start(config: ViteConfig) -> ViteResult<Self> {
        // Check if Vite is available
        let vite_check = Command::new("npx")
            .args(&["vite", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if vite_check.is_err() || !vite_check?.success() {
            return Err(ViteError::NotFound(
                "Vite not found. Install with: npm install -D vite".to_string(),
            ));
        }

        // Start dev server
        let child = Command::new("npx")
            .args(&[
                "vite",
                "--port",
                &config.port.to_string(),
                "--host",
                &config.host,
            ])
            .current_dir(&config.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(Self {
            config,
            process: Some(child),
        })
    }

    /// Get dev server URL
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.config.host, self.config.port)
    }

    /// Generate script tag for dev server
    pub fn script(&self, entry: &str) -> ViteResult<String> {
        let base_url = self.url();

        // Vite dev server script tag
        Ok(format!(
            r#"<script type="module" src="{}/@vite/client"></script>
<script type="module" src="{}/{}"></script>"#,
            base_url, base_url, entry
        ))
    }

    /// Generate link tag for CSS
    pub fn link(&self, entry: &str) -> ViteResult<String> {
        let base_url = self.url();
        Ok(format!(
            r#"<link rel="stylesheet" href="{}/{}">"#,
            base_url, entry
        ))
    }

    /// Stop the dev server
    pub async fn stop(&mut self) -> ViteResult<()> {
        if let Some(mut child) = self.process.take() {
            child.kill().await?;
        }
        Ok(())
    }
}

impl Drop for ViteDevServer {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            // Try to kill the process
            let _ = child.start_kill();
        }
    }
}

/// Vite build manager
struct ViteBuild;

impl ViteBuild {
    /// Run Vite build
    async fn run(config: ViteConfig) -> ViteResult<ViteManifest> {
        // Check if Vite is available
        let vite_check = Command::new("npx")
            .args(&["vite", "--version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        if vite_check.is_err() || !vite_check?.success() {
            return Err(ViteError::NotFound(
                "Vite not found. Install with: npm install -D vite".to_string(),
            ));
        }

        // Run build
        let output = Command::new("npx")
            .args(&["vite", "build"])
            .current_dir(&config.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ViteError::BuildError(stderr.to_string()));
        }

        // Load manifest
        let manifest_path = config.root.join(&config.build_dir).join(&config.manifest);
        ViteManifest::load(manifest_path, config.build_dir).await
    }
}

/// Vite manifest (for production)
#[derive(Debug, Clone)]
pub struct ViteManifest {
    entries: HashMap<String, ManifestEntry>,
    build_dir: PathBuf,
}

impl ViteManifest {
    /// Load manifest from file
    pub async fn load<P: AsRef<Path>>(path: P, build_dir: PathBuf) -> ViteResult<Self> {
        let content = fs::read_to_string(path).await?;
        let entries: HashMap<String, ManifestEntry> = serde_json::from_str(&content)?;

        Ok(Self { entries, build_dir })
    }

    /// Get entry by name
    pub fn get(&self, entry: &str) -> Option<&ManifestEntry> {
        self.entries.get(entry)
    }

    /// Generate script tag for production
    pub fn script(&self, entry: &str) -> ViteResult<String> {
        let manifest_entry = self
            .entries
            .get(entry)
            .ok_or_else(|| ViteError::ManifestError(format!("Entry not found: {}", entry)))?;

        let build_path = format!("/{}/{}", self.build_dir.display(), manifest_entry.file);

        let mut tags = format!(r#"<script type="module" src="{}"></script>"#, build_path);

        // Include CSS files
        if let Some(css) = &manifest_entry.css {
            for css_file in css {
                let css_path = format!("/{}/{}", self.build_dir.display(), css_file);
                tags.push_str(&format!(r#"<link rel="stylesheet" href="{}">"#, css_path));
            }
        }

        Ok(tags)
    }

    /// Generate link tag for CSS
    pub fn link(&self, entry: &str) -> ViteResult<String> {
        let manifest_entry = self
            .entries
            .get(entry)
            .ok_or_else(|| ViteError::ManifestError(format!("Entry not found: {}", entry)))?;

        let build_path = format!("/{}/{}", self.build_dir.display(), manifest_entry.file);
        Ok(format!(r#"<link rel="stylesheet" href="{}">"#, build_path))
    }
}

/// Manifest entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Output file name
    pub file: String,

    /// Source file name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,

    /// Is entry point
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_entry: Option<bool>,

    /// CSS files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css: Option<Vec<String>>,

    /// Assets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<String>>,

    /// Imports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vite_config_builder() {
        let config = ViteConfig::new("./")
            .entry("resources/js/app.js")
            .entry("resources/css/app.css")
            .port(3000)
            .host("0.0.0.0");

        assert_eq!(config.entries.len(), 2);
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "0.0.0.0");
    }

    #[test]
    fn test_dev_server_url() {
        let config = ViteConfig::new("./").port(5173).host("localhost");
        let server = ViteDevServer {
            config,
            process: None,
        };

        assert_eq!(server.url(), "http://localhost:5173");
    }

    #[test]
    fn test_dev_server_script_tag() {
        let config = ViteConfig::new("./");
        let server = ViteDevServer {
            config,
            process: None,
        };

        let tag = server.script("resources/js/app.js").unwrap();

        assert!(tag.contains("@vite/client"));
        assert!(tag.contains("resources/js/app.js"));
    }

    #[tokio::test]
    async fn test_manifest_parsing() {
        let manifest_json = r#"{
            "resources/js/app.js": {
                "file": "assets/app-abc123.js",
                "src": "resources/js/app.js",
                "isEntry": true,
                "css": ["assets/app-xyz789.css"]
            }
        }"#;

        // Create temp manifest file
        let temp_dir = PathBuf::from("/tmp/rf-vite-test");
        fs::create_dir_all(&temp_dir).await.ok();

        let manifest_path = temp_dir.join("manifest.json");
        fs::write(&manifest_path, manifest_json).await.ok();

        let manifest = ViteManifest::load(&manifest_path, PathBuf::from("public/build"))
            .await
            .unwrap();

        let entry = manifest.get("resources/js/app.js").unwrap();
        assert_eq!(entry.file, "assets/app-abc123.js");
        assert_eq!(entry.css.as_ref().unwrap()[0], "assets/app-xyz789.css");

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_manifest_script_generation() {
        let manifest_json = r#"{
            "resources/js/app.js": {
                "file": "assets/app-abc123.js",
                "src": "resources/js/app.js",
                "isEntry": true,
                "css": ["assets/app-xyz789.css"]
            }
        }"#;

        // Create temp manifest file
        let temp_dir = PathBuf::from("/tmp/rf-vite-test-2");
        fs::create_dir_all(&temp_dir).await.ok();

        let manifest_path = temp_dir.join("manifest.json");
        fs::write(&manifest_path, manifest_json).await.ok();

        let manifest = ViteManifest::load(&manifest_path, PathBuf::from("public/build"))
            .await
            .unwrap();

        let script_tag = manifest.script("resources/js/app.js").unwrap();

        assert!(script_tag.contains("assets/app-abc123.js"));
        assert!(script_tag.contains("assets/app-xyz789.css"));
        assert!(script_tag.contains("<script type=\"module\""));
        assert!(script_tag.contains("<link rel=\"stylesheet\""));

        // Cleanup
        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[test]
    fn test_config_defaults() {
        let config = ViteConfig::new("./");

        assert_eq!(config.port, 5173);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.build_dir, PathBuf::from("public/build"));
        assert_eq!(config.manifest, "manifest.json");
    }
}
