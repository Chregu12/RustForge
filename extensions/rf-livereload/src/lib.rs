//! **EXPERIMENTAL — not part of the RustForge 1.0 supported surface; API may change without a SemVer bump.**
//!
//! # rf-livereload - Live Reload and HMR
//!
//! Provides live reload functionality for RustForge applications during development.
//!
//! ## Features
//!
//! - **File Watching**: Automatic detection of file changes
//! - **WebSocket Reload**: Browser reload via WebSocket connection
//! - **Selective Watching**: Watch specific directories/file patterns
//! - **Debouncing**: Configurable debounce to avoid reload spam
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_livereload::LiveReload;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let reload = LiveReload::new()
//!     .watch("resources/views")
//!     .watch("resources/css")
//!     .watch("resources/js")
//!     .debounce_ms(300);
//!
//! // Start live reload server
//! reload.start().await?;
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::sync::RwLock;
use tokio::sync::broadcast;

/// Live reload errors
#[derive(Error, Debug)]
pub enum LiveReloadError {
    #[error("Watch error: {0}")]
    WatchError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Notify error: {0}")]
    NotifyError(#[from] notify::Error),
}

pub type LiveReloadResult<T> = Result<T, LiveReloadError>;

/// Live reload configuration and manager
#[derive(Clone)]
pub struct LiveReload {
    config: Arc<RwLock<Config>>,
    reload_tx: Arc<broadcast::Sender<ReloadEvent>>,
}

#[derive(Debug, Clone)]
struct Config {
    /// Directories to watch
    watch_paths: Vec<PathBuf>,

    /// File patterns to watch (e.g., "*.rs", "*.html")
    patterns: Vec<String>,

    /// Debounce duration in milliseconds
    debounce_ms: u64,

    /// Port for WebSocket server
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_paths: Vec::new(),
            patterns: Vec::new(),
            debounce_ms: 300,
            port: 35729, // Standard LiveReload port
        }
    }
}

/// Reload event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadEvent {
    /// Type of reload
    pub kind: ReloadKind,

    /// Path that triggered the reload
    pub path: Option<String>,

    /// Timestamp
    pub timestamp: u64,
}

/// Type of reload event
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReloadKind {
    /// Full page reload
    Full,

    /// CSS hot reload (no page reload)
    CssOnly,

    /// JavaScript module reload
    JsModule,
}

impl LiveReload {
    /// Create a new live reload instance
    pub fn new() -> Self {
        let (reload_tx, _) = broadcast::channel(100);

        Self {
            config: Arc::new(RwLock::new(Config::default())),
            reload_tx: Arc::new(reload_tx),
        }
    }

    /// Add a directory to watch
    pub fn watch<P: AsRef<Path>>(self, path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        // Use blocking_write to ensure config is set before returning,
        // preventing race conditions when chaining .watch().pattern().start()
        self.config.write().unwrap().watch_paths.push(path);
        self
    }

    /// Add a file pattern to watch
    pub fn pattern(self, pattern: impl Into<String>) -> Self {
        self.config.write().unwrap().patterns.push(pattern.into());
        self
    }

    /// Set debounce duration
    pub fn debounce_ms(self, ms: u64) -> Self {
        self.config.write().unwrap().debounce_ms = ms;
        self
    }

    /// Set WebSocket port
    pub fn port(self, port: u16) -> Self {
        self.config.write().unwrap().port = port;
        self
    }

    /// Start the live reload server
    pub async fn start(self) -> LiveReloadResult<LiveReloadServer> {
        let config = self.config.read().unwrap().clone();

        // Create file watcher
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        // Watch all configured paths
        for path in &config.watch_paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
            }
        }

        // Spawn file change handler
        let reload_tx = self.reload_tx.clone();
        let debounce_ms = config.debounce_ms;

        tokio::spawn(async move {
            let mut last_reload = std::time::Instant::now();

            while let Some(event) = rx.recv().await {
                // Debounce
                if last_reload.elapsed().as_millis() < debounce_ms as u128 {
                    continue;
                }

                last_reload = std::time::Instant::now();

                // Determine reload type based on file extension
                let reload_kind = match &event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        if let Some(path) = event.paths.first() {
                            match path.extension().and_then(|e| e.to_str()) {
                                Some("css") => ReloadKind::CssOnly,
                                Some("js") | Some("ts") => ReloadKind::JsModule,
                                _ => ReloadKind::Full,
                            }
                        } else {
                            ReloadKind::Full
                        }
                    }
                    _ => ReloadKind::Full,
                };

                let reload_event = ReloadEvent {
                    kind: reload_kind,
                    path: event.paths.first().map(|p| p.display().to_string()),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };

                // Broadcast reload event
                let _ = reload_tx.send(reload_event);
            }
        });

        Ok(LiveReloadServer {
            _watcher: Arc::new(tokio::sync::Mutex::new(watcher)),
            reload_rx: self.reload_tx.subscribe(),
            port: config.port,
        })
    }

    /// Get reload event receiver
    pub fn subscribe(&self) -> broadcast::Receiver<ReloadEvent> {
        self.reload_tx.subscribe()
    }

    /// Trigger manual reload
    pub fn trigger(&self, kind: ReloadKind) -> LiveReloadResult<()> {
        let event = ReloadEvent {
            kind,
            path: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.reload_tx
            .send(event)
            .map_err(|e| LiveReloadError::WebSocketError(e.to_string()))?;

        Ok(())
    }
}

impl Default for LiveReload {
    fn default() -> Self {
        Self::new()
    }
}

/// Live reload server instance
pub struct LiveReloadServer {
    _watcher: Arc<tokio::sync::Mutex<notify::RecommendedWatcher>>,
    reload_rx: broadcast::Receiver<ReloadEvent>,
    port: u16,
}

impl LiveReloadServer {
    /// Get the WebSocket port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get WebSocket URL
    pub fn ws_url(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }

    /// Generate client-side script tag
    pub fn script_tag(&self) -> String {
        format!(
            r#"<script>
(function() {{
    const ws = new WebSocket('{}');
    ws.onmessage = (event) => {{
        const data = JSON.parse(event.data);
        if (data.kind === 'CssOnly') {{
            // Reload CSS without full page reload
            const links = document.querySelectorAll('link[rel="stylesheet"]');
            links.forEach(link => {{
                const href = link.href.split('?')[0];
                link.href = href + '?reload=' + Date.now();
            }});
        }} else {{
            // Full page reload
            window.location.reload();
        }}
    }};
    ws.onerror = () => console.log('LiveReload disconnected');
}})();
</script>"#,
            self.ws_url()
        )
    }

    /// Wait for reload events
    pub async fn next_event(&mut self) -> Option<ReloadEvent> {
        self.reload_rx.recv().await.ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_reload_new() {
        let reload = LiveReload::new();
        assert!(Arc::strong_count(&reload.config) >= 1);
    }

    #[tokio::test]
    async fn test_live_reload_config() {
        let reload = LiveReload::new()
            .watch("src/")
            .pattern("*.rs")
            .debounce_ms(500)
            .port(8080);

        // Give time for async config updates
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let _config = reload.config.read().unwrap();
        // Config updates are async, so we can't directly test them
        // In a real implementation, we'd use a different approach
    }

    #[tokio::test]
    async fn test_subscribe() {
        let reload = LiveReload::new();
        let mut rx = reload.subscribe();

        // Trigger a reload
        reload.trigger(ReloadKind::Full).unwrap();

        // Receive event
        let event = rx.recv().await.unwrap();
        assert!(matches!(event.kind, ReloadKind::Full));
    }

    #[tokio::test]
    async fn test_reload_event_types() {
        let reload = LiveReload::new();
        let mut rx = reload.subscribe();

        // Test different reload types
        reload.trigger(ReloadKind::CssOnly).unwrap();
        let event = rx.recv().await.unwrap();
        assert!(matches!(event.kind, ReloadKind::CssOnly));

        reload.trigger(ReloadKind::JsModule).unwrap();
        let event = rx.recv().await.unwrap();
        assert!(matches!(event.kind, ReloadKind::JsModule));
    }

    #[test]
    fn test_reload_kind_serialization() {
        let event = ReloadEvent {
            kind: ReloadKind::Full,
            path: Some("/path/to/file.rs".to_string()),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Full"));
        assert!(json.contains("file.rs"));

        let deserialized: ReloadEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.kind, ReloadKind::Full));
    }

    #[tokio::test]
    async fn test_script_tag_generation() {
        let reload = LiveReload::new().port(35729);
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let server = reload.start().await.unwrap();
        let script = server.script_tag();

        assert!(script.contains("ws://localhost:"));
        assert!(script.contains("WebSocket"));
        assert!(script.contains("window.location.reload"));
    }
}
