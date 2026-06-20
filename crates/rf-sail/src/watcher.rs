//! File watching for auto-rebuild

use crate::{SailError, SailResult};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

/// File watcher for auto-rebuild
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<Event>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new() -> SailResult<Self> {
        let (tx, rx) = mpsc::channel(100);

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            Config::default(),
        )
        .map_err(|e| SailError::IoError(std::io::Error::other(e)))?;

        Ok(Self { watcher, rx })
    }

    /// Watch a path
    pub fn watch(&mut self, path: impl AsRef<Path>) -> SailResult<()> {
        self.watcher
            .watch(path.as_ref(), RecursiveMode::Recursive)
            .map_err(|e| SailError::IoError(std::io::Error::other(e)))
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> SailResult<()> {
        self.watcher
            .unwatch(path.as_ref())
            .map_err(|e| SailError::IoError(std::io::Error::other(e)))
    }

    /// Wait for the next event
    pub async fn next_event(&mut self) -> Option<WatchEvent> {
        while let Some(result) = self.rx.recv().await {
            match result {
                Ok(event) => {
                    // Filter out events we don't care about
                    if should_trigger_rebuild(&event) {
                        return Some(WatchEvent::from(event));
                    }
                }
                Err(e) => {
                    tracing::warn!("Watch error: {:?}", e);
                }
            }
        }
        None
    }
}

/// Watch event
#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub kind: WatchEventKind,
    pub paths: Vec<std::path::PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    Create,
    Modify,
    Remove,
    Other,
}

impl From<Event> for WatchEvent {
    fn from(event: Event) -> Self {
        let kind = match event.kind {
            notify::EventKind::Create(_) => WatchEventKind::Create,
            notify::EventKind::Modify(_) => WatchEventKind::Modify,
            notify::EventKind::Remove(_) => WatchEventKind::Remove,
            _ => WatchEventKind::Other,
        };

        Self {
            kind,
            paths: event.paths,
        }
    }
}

/// Check if an event should trigger a rebuild
fn should_trigger_rebuild(event: &Event) -> bool {
    // Only care about create, modify, remove
    match event.kind {
        notify::EventKind::Create(_)
        | notify::EventKind::Modify(_)
        | notify::EventKind::Remove(_) => {}
        _ => return false,
    }

    // Check file extensions
    for path in &event.paths {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            if matches!(ext.as_str(), "rs" | "toml" | "sql" | "html" | "css" | "js") {
                // Ignore target directory
                if !path.to_string_lossy().contains("/target/") {
                    return true;
                }
            }
        }
    }

    false
}

/// Auto-rebuild configuration
#[derive(Debug, Clone)]
pub struct AutoRebuildConfig {
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
    /// Paths to watch
    pub watch_paths: Vec<String>,
    /// Paths to ignore
    pub ignore_paths: Vec<String>,
    /// File extensions to watch
    pub extensions: Vec<String>,
}

impl Default for AutoRebuildConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            watch_paths: vec!["src".to_string(), "Cargo.toml".to_string()],
            ignore_paths: vec!["target".to_string(), ".git".to_string()],
            extensions: vec![
                "rs".to_string(),
                "toml".to_string(),
                "sql".to_string(),
            ],
        }
    }
}

/// Auto-rebuild runner
pub struct AutoRebuild {
    config: AutoRebuildConfig,
    watcher: FileWatcher,
    rebuild_fn: Arc<dyn Fn() + Send + Sync>,
}

impl AutoRebuild {
    /// Create a new auto-rebuild runner
    pub fn new<F>(config: AutoRebuildConfig, rebuild_fn: F) -> SailResult<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut watcher = FileWatcher::new()?;

        for path in &config.watch_paths {
            watcher.watch(path)?;
        }

        Ok(Self {
            config,
            watcher,
            rebuild_fn: Arc::new(rebuild_fn),
        })
    }

    /// Start watching and rebuilding
    pub async fn run(&mut self) -> SailResult<()> {
        use std::time::{Duration, Instant};

        let debounce = Duration::from_millis(self.config.debounce_ms);
        let mut last_rebuild = Instant::now();

        println!("🔍 Watching for file changes...");

        while let Some(event) = self.watcher.next_event().await {
            // Debounce
            if last_rebuild.elapsed() < debounce {
                continue;
            }

            println!(
                "📝 File changed: {:?}",
                event.paths.first().map(|p| p.display())
            );
            println!("🔨 Rebuilding...");

            (self.rebuild_fn)();
            last_rebuild = Instant::now();

            println!("✅ Rebuild complete");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_rebuild_config_default() {
        let config = AutoRebuildConfig::default();
        assert!(!config.watch_paths.is_empty());
        assert!(!config.ignore_paths.is_empty());
    }

    #[test]
    fn test_watch_event_kind() {
        assert_ne!(WatchEventKind::Create, WatchEventKind::Modify);
    }
}
