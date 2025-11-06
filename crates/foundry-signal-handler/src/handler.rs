//! Main signal handler implementation

use crate::callback::{CallbackCollection, SignalCallback};
use crate::error::{SignalError, SignalResult};
use crate::shutdown::{ShutdownManager, ShutdownPhase};
use crate::signal_types::Signal;
use futures::stream::StreamExt;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Signal handler for managing signal callbacks
pub struct SignalHandler {
    callbacks: Arc<RwLock<HashMap<Signal, CallbackCollection>>>,
    shutdown_manager: Arc<ShutdownManager>,
    signals: Option<Signals>,
}

impl SignalHandler {
    /// Create new signal handler
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            shutdown_manager: Arc::new(ShutdownManager::new()),
            signals: None,
        }
    }

    /// Register a signal handler (trap)
    ///
    /// # Example
    /// ```no_run
    /// # use foundry_signal_handler::{SignalHandler, Signal};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = SignalHandler::new();
    /// handler.trap(Signal::SIGTERM, || {
    ///     println!("Received SIGTERM");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn trap<F>(&mut self, signal: Signal, callback: F) -> SignalResult<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_signal(signal, callback).await
    }

    /// Register callback for a specific signal
    ///
    /// # Example
    /// ```no_run
    /// # use foundry_signal_handler::{SignalHandler, Signal};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = SignalHandler::new();
    /// handler.on_signal(Signal::SIGINT, || {
    ///     println!("Received Ctrl+C");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn on_signal<F>(&mut self, signal: Signal, callback: F) -> SignalResult<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        if !signal.is_supported() {
            warn!("Signal {} is not supported on this platform", signal);
        }

        let callback = SignalCallback::sync(format!("{}_handler", signal.name()), callback);
        let mut callbacks = self.callbacks.write().await;
        callbacks
            .entry(signal)
            .or_insert_with(CallbackCollection::new)
            .add(callback);

        debug!("Registered handler for signal: {}", signal);
        Ok(())
    }

    /// Register async callback for a signal
    ///
    /// # Example
    /// ```no_run
    /// # use foundry_signal_handler::{SignalHandler, Signal};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let handler = SignalHandler::new();
    /// handler.on_signal_async(Signal::SIGTERM, || async {
    ///     // Async cleanup work
    ///     println!("Async cleanup");
    ///     Ok(())
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn on_signal_async<F, Fut>(
        &mut self,
        signal: Signal,
        callback: F,
    ) -> SignalResult<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = SignalResult<()>> + Send + 'static,
    {
        if !signal.is_supported() {
            warn!("Signal {} is not supported on this platform", signal);
        }

        let callback = SignalCallback::new(format!("{}_async_handler", signal.name()), callback);
        let mut callbacks = self.callbacks.write().await;
        callbacks
            .entry(signal)
            .or_insert_with(CallbackCollection::new)
            .add(callback);

        debug!("Registered async handler for signal: {}", signal);
        Ok(())
    }

    /// Register cleanup handler for shutdown phases
    pub async fn register_cleanup<F>(
        &self,
        phase: ShutdownPhase,
        name: impl Into<String>,
        callback: F,
    ) -> SignalResult<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let callback = SignalCallback::sync(name, callback);
        self.shutdown_manager
            .register_cleanup(phase, callback)
            .await
    }

    /// Start listening for signals
    pub async fn listen(&mut self) -> SignalResult<()> {
        let signal_nums: Vec<i32> = Signal::all_supported()
            .iter()
            .map(|s| s.as_raw())
            .collect();

        let signals = Signals::new(&signal_nums)
            .map_err(|e| SignalError::RegistrationFailed(e.to_string()))?;

        self.signals = Some(signals);
        info!("Signal handler started listening");
        Ok(())
    }

    /// Wait for a signal and handle it
    pub async fn wait(&mut self) -> SignalResult<i32> {
        if self.signals.is_none() {
            self.listen().await?;
        }

        let mut signals = self
            .signals
            .take()
            .ok_or_else(|| SignalError::HandlingFailed("Signals not initialized".to_string()))?;

        let callbacks = self.callbacks.clone();
        let shutdown_manager = self.shutdown_manager.clone();

        while let Some(signal_num) = signals.next().await {
            let signal = Self::map_signal_num_static(signal_num);
            info!("Received signal: {} ({})", signal, signal_num);

            // Execute signal callbacks
            if let Some(collection) = callbacks.read().await.get(&signal) {
                if let Err(e) = collection.execute_all().await {
                    warn!("Error executing signal callbacks: {}", e);
                }
            }

            // Initiate graceful shutdown for terminal signals
            if matches!(signal, Signal::SIGTERM | Signal::SIGINT) {
                info!("Terminal signal received, initiating shutdown");
                return shutdown_manager.shutdown().await;
            }
        }

        Ok(0)
    }

    /// Wait for a signal without blocking
    pub async fn wait_once(&mut self) -> SignalResult<Signal> {
        if self.signals.is_none() {
            self.listen().await?;
        }

        let signals = self
            .signals
            .as_mut()
            .ok_or_else(|| SignalError::HandlingFailed("Signals not initialized".to_string()))?;

        if let Some(signal_num) = signals.next().await {
            let signal = Self::map_signal_num_static(signal_num);
            info!("Received signal: {} ({})", signal, signal_num);

            // Execute callbacks
            if let Some(collection) = self.callbacks.read().await.get(&signal) {
                if let Err(e) = collection.execute_all().await {
                    warn!("Error executing signal callbacks: {}", e);
                }
            }

            Ok(signal)
        } else {
            Err(SignalError::HandlingFailed("No signal received".to_string()))
        }
    }

    /// Map signal number to Signal enum (static version for use without self)
    fn map_signal_num_static(signal_num: i32) -> Signal {
        match signal_num {
            SIGTERM => Signal::SIGTERM,
            SIGINT => Signal::SIGINT,
            #[cfg(unix)]
            SIGHUP => Signal::SIGHUP,
            #[cfg(unix)]
            SIGQUIT => Signal::SIGQUIT,
            #[cfg(unix)]
            SIGUSR1 => Signal::SIGUSR1,
            #[cfg(unix)]
            SIGUSR2 => Signal::SIGUSR2,
            _ => {
                warn!("Unknown signal: {}", signal_num);
                Signal::SIGTERM
            }
        }
    }

    /// Get shutdown manager
    pub fn shutdown_manager(&self) -> &Arc<ShutdownManager> {
        &self.shutdown_manager
    }

    /// Check if shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_manager.is_shutting_down()
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_signal_handler_creation() {
        let handler = SignalHandler::new();
        assert!(!handler.is_shutting_down());
    }

    #[tokio::test]
    async fn test_register_signal_handler() {
        let mut handler = SignalHandler::new();
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        handler
            .on_signal(Signal::SIGTERM, move || {
                executed_clone.store(true, Ordering::SeqCst);
            })
            .await
            .unwrap();

        // Manually execute the callback for testing
        if let Some(collection) = handler.callbacks.read().await.get(&Signal::SIGTERM) {
            collection.execute_all().await.unwrap();
            assert!(executed.load(Ordering::SeqCst));
        }
    }

    #[tokio::test]
    async fn test_register_async_handler() {
        let mut handler = SignalHandler::new();
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        handler
            .on_signal_async(Signal::SIGINT, move || {
                let executed = executed_clone.clone();
                async move {
                    executed.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await
            .unwrap();

        // Manually execute the callback for testing
        if let Some(collection) = handler.callbacks.read().await.get(&Signal::SIGINT) {
            collection.execute_all().await.unwrap();
            assert!(executed.load(Ordering::SeqCst));
        }
    }

    #[tokio::test]
    async fn test_register_cleanup() {
        let handler = SignalHandler::new();
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        handler
            .register_cleanup(ShutdownPhase::Shutdown, "test_cleanup", move || {
                executed_clone.store(true, Ordering::SeqCst);
            })
            .await
            .unwrap();

        handler.shutdown_manager.shutdown().await.unwrap();
        assert!(executed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_trap_alias() {
        let mut handler = SignalHandler::new();
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        handler
            .trap(Signal::SIGTERM, move || {
                executed_clone.store(true, Ordering::SeqCst);
            })
            .await
            .unwrap();

        // Verify the callback was registered
        assert!(handler.callbacks.read().await.contains_key(&Signal::SIGTERM));
    }
}
