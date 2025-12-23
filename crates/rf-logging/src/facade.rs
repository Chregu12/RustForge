//! Laravel-style Log facade for RustForge

/// The Log facade providing a static API for logging.
///
/// # Examples
///
/// ```rust
/// use rf_logging::Log;
///
/// Log::info("User logged in");
/// Log::error("Database connection failed");
/// Log::warning("Disk space running low");
/// ```
pub struct Log;

impl Log {
    pub fn info(message: &str) {
        tracing::info!("{}", message);
    }

    pub fn error(message: &str) {
        tracing::error!("{}", message);
    }

    pub fn warning(message: &str) {
        tracing::warn!("{}", message);
    }

    pub fn debug(message: &str) {
        tracing::debug!("{}", message);
    }

    pub fn emergency(message: &str) {
        tracing::error!("[EMERGENCY] {}", message);
    }

    pub fn alert(message: &str) {
        tracing::error!("[ALERT] {}", message);
    }

    pub fn critical(message: &str) {
        tracing::error!("[CRITICAL] {}", message);
    }

    pub fn notice(message: &str) {
        tracing::info!("[NOTICE] {}", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_methods() {
        // These tests just verify the methods compile and run
        Log::info("Test info message");
        Log::error("Test error message");
        Log::warning("Test warning message");
        Log::debug("Test debug message");
        Log::emergency("Test emergency message");
        Log::alert("Test alert message");
        Log::critical("Test critical message");
        Log::notice("Test notice message");
    }
}
