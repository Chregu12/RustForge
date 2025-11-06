use std::fmt;

/// Verbosity level for output control
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerbosityLevel {
    /// Quiet mode - only errors
    Quiet = 0,
    /// Normal mode - standard output
    Normal = 1,
    /// Verbose mode (-v) - includes info messages
    Verbose = 2,
    /// Very verbose mode (-vv) - includes debug messages
    VeryVerbose = 3,
    /// Trace mode (-vvv) - includes trace messages
    Trace = 4,
}

impl VerbosityLevel {
    /// Check if info messages should be shown
    pub fn shows_info(&self) -> bool {
        *self >= VerbosityLevel::Verbose
    }

    /// Check if debug messages should be shown
    pub fn shows_debug(&self) -> bool {
        *self >= VerbosityLevel::VeryVerbose
    }

    /// Check if trace messages should be shown
    pub fn shows_trace(&self) -> bool {
        *self >= VerbosityLevel::Trace
    }

    /// Check if normal messages should be shown
    pub fn shows_normal(&self) -> bool {
        *self >= VerbosityLevel::Normal
    }

    /// Check if error messages should be shown (always true except in extreme cases)
    pub fn shows_error(&self) -> bool {
        true
    }

    /// Create from count of verbose flags (-v, -vv, -vvv)
    pub fn from_count(count: u8) -> Self {
        match count {
            0 => Self::Normal,
            1 => Self::Verbose,
            2 => Self::VeryVerbose,
            _ => Self::Trace,
        }
    }

    /// Get the logging filter string for tracing
    pub fn to_filter_string(&self) -> &'static str {
        match self {
            Self::Quiet => "error",
            Self::Normal => "warn",
            Self::Verbose => "info",
            Self::VeryVerbose => "debug",
            Self::Trace => "trace",
        }
    }
}

impl Default for VerbosityLevel {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for VerbosityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quiet => write!(f, "quiet"),
            Self::Normal => write!(f, "normal"),
            Self::Verbose => write!(f, "verbose"),
            Self::VeryVerbose => write!(f, "very verbose"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

/// Clap integration for verbosity arguments
#[cfg(feature = "clap")]
#[derive(clap::Args, Debug, Clone)]
pub struct VerbosityArgs {
    /// Quiet mode (errors only)
    #[arg(short = 'q', long = "quiet", conflicts_with = "verbose")]
    pub quiet: bool,

    /// Increase verbosity (-v: info, -vv: debug, -vvv: trace)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[cfg(feature = "clap")]
impl VerbosityArgs {
    /// Convert to VerbosityLevel
    pub fn to_level(&self) -> VerbosityLevel {
        if self.quiet {
            VerbosityLevel::Quiet
        } else {
            VerbosityLevel::from_count(self.verbose)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_levels() {
        assert!(VerbosityLevel::Quiet < VerbosityLevel::Normal);
        assert!(VerbosityLevel::Normal < VerbosityLevel::Verbose);
        assert!(VerbosityLevel::Verbose < VerbosityLevel::VeryVerbose);
        assert!(VerbosityLevel::VeryVerbose < VerbosityLevel::Trace);
    }

    #[test]
    fn test_shows_info() {
        assert!(!VerbosityLevel::Quiet.shows_info());
        assert!(!VerbosityLevel::Normal.shows_info());
        assert!(VerbosityLevel::Verbose.shows_info());
        assert!(VerbosityLevel::VeryVerbose.shows_info());
        assert!(VerbosityLevel::Trace.shows_info());
    }

    #[test]
    fn test_shows_debug() {
        assert!(!VerbosityLevel::Quiet.shows_debug());
        assert!(!VerbosityLevel::Normal.shows_debug());
        assert!(!VerbosityLevel::Verbose.shows_debug());
        assert!(VerbosityLevel::VeryVerbose.shows_debug());
        assert!(VerbosityLevel::Trace.shows_debug());
    }

    #[test]
    fn test_shows_trace() {
        assert!(!VerbosityLevel::Quiet.shows_trace());
        assert!(!VerbosityLevel::Normal.shows_trace());
        assert!(!VerbosityLevel::Verbose.shows_trace());
        assert!(!VerbosityLevel::VeryVerbose.shows_trace());
        assert!(VerbosityLevel::Trace.shows_trace());
    }

    #[test]
    fn test_from_count() {
        assert_eq!(VerbosityLevel::from_count(0), VerbosityLevel::Normal);
        assert_eq!(VerbosityLevel::from_count(1), VerbosityLevel::Verbose);
        assert_eq!(VerbosityLevel::from_count(2), VerbosityLevel::VeryVerbose);
        assert_eq!(VerbosityLevel::from_count(3), VerbosityLevel::Trace);
        assert_eq!(VerbosityLevel::from_count(5), VerbosityLevel::Trace);
    }

    #[test]
    fn test_to_filter_string() {
        assert_eq!(VerbosityLevel::Quiet.to_filter_string(), "error");
        assert_eq!(VerbosityLevel::Normal.to_filter_string(), "warn");
        assert_eq!(VerbosityLevel::Verbose.to_filter_string(), "info");
        assert_eq!(VerbosityLevel::VeryVerbose.to_filter_string(), "debug");
        assert_eq!(VerbosityLevel::Trace.to_filter_string(), "trace");
    }
}
