//! Signal types and cross-platform signal definitions

use std::fmt;

/// Cross-platform signal types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    /// Termination signal (SIGTERM on Unix, Ctrl+C on Windows)
    SIGTERM,
    /// Interrupt signal (SIGINT / Ctrl+C)
    SIGINT,
    /// Hangup signal (SIGHUP on Unix, not available on Windows)
    SIGHUP,
    /// Quit signal (SIGQUIT on Unix, not available on Windows)
    SIGQUIT,
    /// User-defined signal 1 (SIGUSR1 on Unix, not available on Windows)
    SIGUSR1,
    /// User-defined signal 2 (SIGUSR2 on Unix, not available on Windows)
    SIGUSR2,
}

impl Signal {
    /// Get the signal number for this platform
    pub fn as_raw(&self) -> i32 {
        #[cfg(unix)]
        {
            match self {
                Signal::SIGTERM => signal_hook::consts::SIGTERM,
                Signal::SIGINT => signal_hook::consts::SIGINT,
                Signal::SIGHUP => signal_hook::consts::SIGHUP,
                Signal::SIGQUIT => signal_hook::consts::SIGQUIT,
                Signal::SIGUSR1 => signal_hook::consts::SIGUSR1,
                Signal::SIGUSR2 => signal_hook::consts::SIGUSR2,
            }
        }

        #[cfg(not(unix))]
        {
            // On Windows, we only support SIGTERM and SIGINT
            match self {
                Signal::SIGTERM | Signal::SIGINT => signal_hook::consts::SIGTERM,
                _ => {
                    tracing::warn!("Signal {:?} not supported on Windows, using SIGTERM", self);
                    signal_hook::consts::SIGTERM
                }
            }
        }
    }

    /// Check if signal is supported on this platform
    pub fn is_supported(&self) -> bool {
        #[cfg(unix)]
        {
            true
        }

        #[cfg(not(unix))]
        {
            matches!(self, Signal::SIGTERM | Signal::SIGINT)
        }
    }

    /// Get all supported signals for this platform
    pub fn all_supported() -> Vec<Signal> {
        #[cfg(unix)]
        {
            vec![
                Signal::SIGTERM,
                Signal::SIGINT,
                Signal::SIGHUP,
                Signal::SIGQUIT,
                Signal::SIGUSR1,
                Signal::SIGUSR2,
            ]
        }

        #[cfg(not(unix))]
        {
            vec![Signal::SIGTERM, Signal::SIGINT]
        }
    }

    /// Get signal name
    pub fn name(&self) -> &'static str {
        match self {
            Signal::SIGTERM => "SIGTERM",
            Signal::SIGINT => "SIGINT",
            Signal::SIGHUP => "SIGHUP",
            Signal::SIGQUIT => "SIGQUIT",
            Signal::SIGUSR1 => "SIGUSR1",
            Signal::SIGUSR2 => "SIGUSR2",
        }
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl From<Signal> for i32 {
    fn from(signal: Signal) -> Self {
        signal.as_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_name() {
        assert_eq!(Signal::SIGTERM.name(), "SIGTERM");
        assert_eq!(Signal::SIGINT.name(), "SIGINT");
        assert_eq!(Signal::SIGHUP.name(), "SIGHUP");
    }

    #[test]
    fn test_signal_display() {
        assert_eq!(format!("{}", Signal::SIGTERM), "SIGTERM");
        assert_eq!(format!("{}", Signal::SIGINT), "SIGINT");
    }

    #[test]
    #[cfg(unix)]
    fn test_all_signals_supported_on_unix() {
        assert!(Signal::SIGTERM.is_supported());
        assert!(Signal::SIGINT.is_supported());
        assert!(Signal::SIGHUP.is_supported());
        assert!(Signal::SIGQUIT.is_supported());
        assert!(Signal::SIGUSR1.is_supported());
        assert!(Signal::SIGUSR2.is_supported());
    }

    #[test]
    #[cfg(not(unix))]
    fn test_limited_signals_on_windows() {
        assert!(Signal::SIGTERM.is_supported());
        assert!(Signal::SIGINT.is_supported());
        assert!(!Signal::SIGHUP.is_supported());
        assert!(!Signal::SIGQUIT.is_supported());
    }

    #[test]
    fn test_signal_to_raw() {
        let sig = Signal::SIGTERM;
        let raw: i32 = sig.into();
        assert!(raw > 0);
    }
}
