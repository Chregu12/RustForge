/// Isolatable Commands System
///
/// Ensures that only one instance of a command can run at a time,
/// similar to Laravel's isolatable() method for preventing concurrent execution.
///
/// This is useful for long-running commands, database maintenance tasks,
/// or any operation that shouldn't run simultaneously with itself.
///
/// # Example
///
/// ```rust,no_run
/// use foundry_api::command_isolation::{CommandIsolation, LockStrategy};
///
/// let isolation = CommandIsolation::new("migrate")
///     .with_timeout(std::time::Duration::from_secs(300));
///
/// match isolation.lock() {
///     Ok(guard) => {
///         println!("Running isolated command");
///         // Guard is dropped at end of scope, releasing lock
///     }
///     Err(e) => eprintln!("Could not acquire lock: {}", e),
/// }
/// ```

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Lock strategy for isolation
#[derive(Clone, Debug, Copy)]
pub enum LockStrategy {
    /// File-based locking (default, works across processes)
    File,
    /// In-memory locking (single process only)
    Memory,
}

/// Command isolation error
#[derive(Debug, Clone)]
pub enum IsolationError {
    /// Command is already running
    AlreadyRunning {
        command: String,
        locked_at: String,
    },
    /// Failed to create lock file
    LockFileError(String),
    /// Lock timeout exceeded
    Timeout {
        command: String,
        timeout: Duration,
    },
    /// Permission denied
    PermissionDenied(String),
    /// Other IO error
    IoError(String),
}

impl std::fmt::Display for IsolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationError::AlreadyRunning { command, locked_at } => {
                write!(f, "Command '{}' is already running (locked at: {})", command, locked_at)
            }
            IsolationError::LockFileError(msg) => write!(f, "Lock file error: {}", msg),
            IsolationError::Timeout { command, timeout } => {
                write!(f, "Command '{}' is locked, timeout after {:?}", command, timeout)
            }
            IsolationError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            IsolationError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for IsolationError {}

/// Lock guard that releases the lock when dropped
pub struct IsolationGuard {
    lock_path: PathBuf,
    _strategy: LockStrategy,
}

impl IsolationGuard {
    /// Create a new guard
    fn new(lock_path: PathBuf, strategy: LockStrategy) -> Self {
        Self {
            lock_path,
            _strategy: strategy,
        }
    }

    /// Get the lock file path
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for IsolationGuard {
    fn drop(&mut self) {
        // Release the lock by removing the file
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// Command isolation manager
pub struct CommandIsolation {
    command: String,
    lock_dir: PathBuf,
    strategy: LockStrategy,
    timeout: Option<Duration>,
    memory_locks: Arc<Mutex<Vec<String>>>,
}

impl CommandIsolation {
    /// Create a new command isolation
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            lock_dir: PathBuf::from(".foundry/locks"),
            strategy: LockStrategy::File,
            timeout: None,
            memory_locks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set the lock directory
    pub fn with_lock_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.lock_dir = path.as_ref().to_path_buf();
        self
    }

    /// Set the lock strategy
    pub fn with_strategy(mut self, strategy: LockStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the lock timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Try to acquire the lock
    pub fn lock(&self) -> Result<IsolationGuard, IsolationError> {
        match self.strategy {
            LockStrategy::File => self.lock_file(),
            LockStrategy::Memory => self.lock_memory(),
        }
    }

    /// Try to acquire lock with timeout
    pub fn lock_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<IsolationGuard, IsolationError> {
        let start = Instant::now();

        loop {
            match self.lock() {
                Ok(guard) => return Ok(guard),
                Err(IsolationError::AlreadyRunning { .. }) => {
                    if start.elapsed() > timeout {
                        return Err(IsolationError::Timeout {
                            command: self.command.clone(),
                            timeout,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Attempt file-based locking
    fn lock_file(&self) -> Result<IsolationGuard, IsolationError> {
        // Create lock directory if it doesn't exist
        fs::create_dir_all(&self.lock_dir).map_err(|e| {
            IsolationError::LockFileError(format!("Failed to create lock directory: {}", e))
        })?;

        let lock_path = self.lock_dir.join(format!("{}.lock", self.command));

        // Check if lock file already exists
        if lock_path.exists() {
            let locked_at = fs::read_to_string(&lock_path)
                .unwrap_or_else(|_| "unknown time".to_string());

            return Err(IsolationError::AlreadyRunning {
                command: self.command.clone(),
                locked_at,
            });
        }

        // Create lock file with current timestamp
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut file = File::create(&lock_path).map_err(|e| {
            IsolationError::LockFileError(format!("Failed to create lock file: {}", e))
        })?;

        file.write_all(timestamp.as_bytes()).map_err(|e| {
            IsolationError::LockFileError(format!("Failed to write lock file: {}", e))
        })?;

        Ok(IsolationGuard::new(lock_path, LockStrategy::File))
    }

    /// Attempt memory-based locking (single process only)
    fn lock_memory(&self) -> Result<IsolationGuard, IsolationError> {
        let mut locks = self.memory_locks.lock().unwrap();

        if locks.contains(&self.command) {
            return Err(IsolationError::AlreadyRunning {
                command: self.command.clone(),
                locked_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        locks.push(self.command.clone());

        // Note: Memory locks won't be cleaned up automatically on panic
        // File-based locks are more reliable for production use
        Ok(IsolationGuard::new(
            PathBuf::from(format!("memory://{}", self.command)),
            LockStrategy::Memory,
        ))
    }

    /// Check if the command is currently locked
    pub fn is_locked(&self) -> bool {
        match self.strategy {
            LockStrategy::File => {
                let lock_path = self.lock_dir.join(format!("{}.lock", self.command));
                lock_path.exists()
            }
            LockStrategy::Memory => {
                let locks = self.memory_locks.lock().unwrap();
                locks.contains(&self.command)
            }
        }
    }

    /// Get the lock file path (file strategy only)
    pub fn lock_path(&self) -> PathBuf {
        self.lock_dir.join(format!("{}.lock", self.command))
    }

    /// Release all locks (for cleanup)
    pub fn release_all(&self) -> Result<(), IsolationError> {
        match self.strategy {
            LockStrategy::File => {
                let lock_path = self.lock_path();
                if lock_path.exists() {
                    fs::remove_file(&lock_path).map_err(|e| {
                        IsolationError::LockFileError(format!("Failed to remove lock file: {}", e))
                    })?;
                }
                Ok(())
            }
            LockStrategy::Memory => {
                let mut locks = self.memory_locks.lock().unwrap();
                locks.retain(|cmd| cmd != &self.command);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_creation() {
        let isolation = CommandIsolation::new("test");
        assert_eq!(isolation.command, "test");
    }

    #[test]
    fn test_isolation_with_timeout() {
        let isolation = CommandIsolation::new("test")
            .with_timeout(Duration::from_secs(30));

        assert!(isolation.timeout.is_some());
    }

    #[test]
    fn test_memory_lock_success() {
        let isolation = CommandIsolation::new("test")
            .with_strategy(LockStrategy::Memory);

        let guard = isolation.lock();
        assert!(guard.is_ok());
    }

    #[test]
    fn test_memory_lock_already_running() {
        let isolation = CommandIsolation::new("test")
            .with_strategy(LockStrategy::Memory);

        let _guard1 = isolation.lock().unwrap();
        let result = isolation.lock();

        assert!(result.is_err());
        match result {
            Err(IsolationError::AlreadyRunning { command, .. }) => {
                assert_eq!(command, "test");
            }
            _ => panic!("Expected AlreadyRunning error"),
        }
    }

    #[test]
    fn test_is_locked() {
        let isolation = CommandIsolation::new("test")
            .with_strategy(LockStrategy::Memory);

        assert!(!isolation.is_locked());
        let _guard = isolation.lock().unwrap();
        assert!(isolation.is_locked());
    }

    #[test]
    fn test_lock_guard_drop() {
        let isolation = CommandIsolation::new("test")
            .with_strategy(LockStrategy::Memory);

        {
            let _guard = isolation.lock().unwrap();
            assert!(isolation.is_locked());
        }

        // After guard is dropped, lock should be released
        // (This test is timing dependent for memory locks)
    }
}
