/// Verbosity level support for RustForge commands
///
/// Provides a system for commands to control output verbosity based on flags:
/// - None: -q, --quiet - Suppress most output
/// - Normal: Default - Standard output
/// - Verbose: -v - Additional information
/// - VeryVerbose: -vv - Much more details
/// - Debug: -vvv - All debug information
///
/// # Example
///
/// ```rust
/// use foundry_api::verbosity::{Verbosity, VerbosityLevel};
///
/// let verbosity = Verbosity::from_args(&["-vv".to_string()]);
/// assert_eq!(verbosity.level(), VerbosityLevel::VeryVerbose);
///
/// // Output conditionally
/// if verbosity.is_debug() {
///     println!("Debug information");
/// }
///
/// if verbosity.is_verbose() {
///     println!("Detailed information");
/// }
/// ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Verbosity level enumeration
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerbosityLevel {
    /// Quiet mode - suppress most output
    Quiet = 0,
    /// Normal mode - standard output (default)
    Normal = 1,
    /// Verbose mode - additional information (-v)
    Verbose = 2,
    /// Very verbose mode - much more details (-vv)
    VeryVerbose = 3,
    /// Debug mode - all debug information (-vvv)
    Debug = 4,
}

impl VerbosityLevel {
    /// Check if this level includes information
    pub fn is_quiet(self) -> bool {
        self == VerbosityLevel::Quiet
    }

    /// Check if this is normal or more verbose
    pub fn is_normal(self) -> bool {
        self >= VerbosityLevel::Normal
    }

    /// Check if this is verbose or more
    pub fn is_verbose(self) -> bool {
        self >= VerbosityLevel::Verbose
    }

    /// Check if this is very verbose or more
    pub fn is_very_verbose(self) -> bool {
        self >= VerbosityLevel::VeryVerbose
    }

    /// Check if this is debug mode
    pub fn is_debug(self) -> bool {
        self >= VerbosityLevel::Debug
    }

    /// Get numeric representation (0-4)
    pub fn level(self) -> u8 {
        self as u8
    }

    /// Parse from string like "-v", "-vv", "-vvv", or "--verbose"
    pub fn parse(arg: &str) -> Option<Self> {
        match arg {
            "-q" | "--quiet" => Some(VerbosityLevel::Quiet),
            "-v" | "--verbose" => Some(VerbosityLevel::Verbose),
            "-vv" | "--very-verbose" => Some(VerbosityLevel::VeryVerbose),
            "-vvv" | "--debug" => Some(VerbosityLevel::Debug),
            _ => None,
        }
    }

    /// Get human-readable name
    pub fn name(self) -> &'static str {
        match self {
            VerbosityLevel::Quiet => "quiet",
            VerbosityLevel::Normal => "normal",
            VerbosityLevel::Verbose => "verbose",
            VerbosityLevel::VeryVerbose => "very_verbose",
            VerbosityLevel::Debug => "debug",
        }
    }
}

impl Default for VerbosityLevel {
    fn default() -> Self {
        VerbosityLevel::Normal
    }
}

impl fmt::Display for VerbosityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Verbosity configuration for command output
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verbosity {
    level: VerbosityLevel,
}

impl Verbosity {
    /// Create a new Verbosity with the given level
    pub fn new(level: VerbosityLevel) -> Self {
        Self { level }
    }

    /// Get the current verbosity level
    pub fn level(&self) -> VerbosityLevel {
        self.level
    }

    /// Check if output is quiet
    pub fn is_quiet(&self) -> bool {
        self.level.is_quiet()
    }

    /// Check if output is normal (default)
    pub fn is_normal(&self) -> bool {
        self.level.is_normal()
    }

    /// Check if output is verbose or higher
    pub fn is_verbose(&self) -> bool {
        self.level.is_verbose()
    }

    /// Check if output is very verbose or higher
    pub fn is_very_verbose(&self) -> bool {
        self.level.is_very_verbose()
    }

    /// Check if output is debug
    pub fn is_debug(&self) -> bool {
        self.level.is_debug()
    }

    /// Parse verbosity from command arguments
    ///
    /// Looks for -q, -v, -vv, -vvv flags and returns the highest verbosity found
    pub fn from_args(args: &[String]) -> Self {
        let mut level = VerbosityLevel::Normal;

        for arg in args {
            if let Some(parsed) = VerbosityLevel::parse(arg) {
                if parsed > level {
                    level = parsed;
                }
            }
        }

        Self { level }
    }

    /// Set the verbosity level
    pub fn set_level(&mut self, level: VerbosityLevel) {
        self.level = level;
    }

    /// Output a message only if not quiet
    pub fn println(&self, msg: impl AsRef<str>) {
        if !self.is_quiet() {
            println!("{}", msg.as_ref());
        }
    }

    /// Output a verbose message
    pub fn println_verbose(&self, msg: impl AsRef<str>) {
        if self.is_verbose() {
            println!("{}", msg.as_ref());
        }
    }

    /// Output a very verbose message
    pub fn println_very_verbose(&self, msg: impl AsRef<str>) {
        if self.is_very_verbose() {
            println!("{}", msg.as_ref());
        }
    }

    /// Output a debug message
    pub fn println_debug(&self, msg: impl AsRef<str>) {
        if self.is_debug() {
            println!("{}", msg.as_ref());
        }
    }

    /// Output error only if not quiet
    pub fn eprintln(&self, msg: impl AsRef<str>) {
        if !self.is_quiet() {
            eprintln!("{}", msg.as_ref());
        }
    }

    /// Get the appropriate format string based on verbosity
    ///
    /// Returns one of the three provided strings based on current verbosity level
    pub fn format<'a>(&self, normal: &'a str, verbose: &'a str, debug: &'a str) -> &'a str {
        if self.is_debug() {
            debug
        } else if self.is_verbose() {
            verbose
        } else {
            normal
        }
    }
}

impl Default for Verbosity {
    fn default() -> Self {
        Self {
            level: VerbosityLevel::Normal,
        }
    }
}

impl From<VerbosityLevel> for Verbosity {
    fn from(level: VerbosityLevel) -> Self {
        Self::new(level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_level_comparison() {
        assert!(VerbosityLevel::Debug > VerbosityLevel::VeryVerbose);
        assert!(VerbosityLevel::Verbose > VerbosityLevel::Normal);
        assert!(VerbosityLevel::Quiet < VerbosityLevel::Normal);
    }

    #[test]
    fn test_verbosity_level_parse() {
        assert_eq!(VerbosityLevel::parse("-q"), Some(VerbosityLevel::Quiet));
        assert_eq!(VerbosityLevel::parse("-v"), Some(VerbosityLevel::Verbose));
        assert_eq!(VerbosityLevel::parse("-vv"), Some(VerbosityLevel::VeryVerbose));
        assert_eq!(VerbosityLevel::parse("-vvv"), Some(VerbosityLevel::Debug));
        assert_eq!(VerbosityLevel::parse("--quiet"), Some(VerbosityLevel::Quiet));
        assert_eq!(VerbosityLevel::parse("--verbose"), Some(VerbosityLevel::Verbose));
        assert_eq!(VerbosityLevel::parse("--very-verbose"), Some(VerbosityLevel::VeryVerbose));
        assert_eq!(VerbosityLevel::parse("--debug"), Some(VerbosityLevel::Debug));
        assert_eq!(VerbosityLevel::parse("invalid"), None);
    }

    #[test]
    fn test_verbosity_level_checks() {
        let quiet = VerbosityLevel::Quiet;
        let normal = VerbosityLevel::Normal;
        let verbose = VerbosityLevel::Verbose;
        let very_verbose = VerbosityLevel::VeryVerbose;
        let debug = VerbosityLevel::Debug;

        assert!(quiet.is_quiet());
        assert!(!quiet.is_verbose());
        assert!(!quiet.is_debug());

        assert!(normal.is_normal());
        assert!(!normal.is_verbose());

        assert!(verbose.is_verbose());
        assert!(!verbose.is_very_verbose());

        assert!(very_verbose.is_very_verbose());
        assert!(!very_verbose.is_debug());

        assert!(debug.is_debug());
        assert!(debug.is_very_verbose());
        assert!(debug.is_verbose());
    }

    #[test]
    fn test_verbosity_from_args() {
        let args = vec!["-v".to_string(), "command".to_string()];
        let verbosity = Verbosity::from_args(&args);
        assert_eq!(verbosity.level(), VerbosityLevel::Verbose);

        let args = vec!["-vv".to_string(), "arg1".to_string()];
        let verbosity = Verbosity::from_args(&args);
        assert_eq!(verbosity.level(), VerbosityLevel::VeryVerbose);

        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let verbosity = Verbosity::from_args(&args);
        assert_eq!(verbosity.level(), VerbosityLevel::Normal);

        let args = vec!["-v".to_string(), "-vv".to_string()];
        let verbosity = Verbosity::from_args(&args);
        assert_eq!(verbosity.level(), VerbosityLevel::VeryVerbose);
    }

    #[test]
    fn test_verbosity_format() {
        let verbosity = Verbosity::new(VerbosityLevel::Verbose);
        assert_eq!(
            verbosity.format("normal", "verbose", "debug"),
            "verbose"
        );

        let verbosity = Verbosity::new(VerbosityLevel::Normal);
        assert_eq!(
            verbosity.format("normal", "verbose", "debug"),
            "normal"
        );

        let verbosity = Verbosity::new(VerbosityLevel::Debug);
        assert_eq!(
            verbosity.format("normal", "verbose", "debug"),
            "debug"
        );
    }

    #[test]
    fn test_verbosity_level_name() {
        assert_eq!(VerbosityLevel::Quiet.name(), "quiet");
        assert_eq!(VerbosityLevel::Normal.name(), "normal");
        assert_eq!(VerbosityLevel::Verbose.name(), "verbose");
        assert_eq!(VerbosityLevel::VeryVerbose.name(), "very_verbose");
        assert_eq!(VerbosityLevel::Debug.name(), "debug");
    }
}
