//! # Foundry Verbosity
//!
//! Verbosity level support for CLI commands with -v, -vv, -vvv flags.
//!
//! ## Features
//!
//! - **Multiple Levels**: Quiet, Normal, Verbose, Very Verbose, and Trace
//! - **Output Filtering**: Automatic message filtering based on verbosity level
//! - **Conditional Printing**: Print messages only when appropriate
//! - **Integration**: Easy integration with CLI parsers like Clap
//!
//! ## Example
//!
//! ```rust
//! use foundry_verbosity::{VerbosityLevel, Output};
//!
//! let level = VerbosityLevel::Verbose;
//! let output = Output::new(level);
//!
//! output.info("This is shown at -v and above");
//! output.debug("This is shown at -vv and above");
//! output.trace("This is shown at -vvv");
//! output.error("This is always shown");
//! ```

mod level;
mod output;

pub use level::VerbosityLevel;
pub use output::Output;

#[cfg(feature = "clap")]
pub use level::VerbosityArgs;
