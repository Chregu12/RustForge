/// Example demonstrating Verbosity Levels in RustForge
///
/// This example shows how to use the Verbosity system to control command output.
///
/// Run with:
/// cargo run --example verbosity_example -- -vv

use std::env;

fn main() {
    println!("RustForge Verbosity Example\n");

    // Get command line arguments
    let args: Vec<String> = env::args().skip(1).collect();

    // Example 1: Parse verbosity from arguments
    println!("=== Example 1: Parsing Verbosity ===");
    example_parse_verbosity(&args);

    println!("\n=== Example 2: Using Verbosity Checks ===");
    example_verbosity_checks(&args);

    println!("\n=== Example 3: Simulating Command Execution ===");
    example_command_execution(&args);
}

fn example_parse_verbosity(args: &[String]) {
    // This would normally use foundry_api::Verbosity::from_args()
    // For this example, we'll demonstrate the concept
    let verbosity_level = parse_verbosity_demo(args);
    println!("Parsed verbosity level: {}", verbosity_level);
}

fn example_verbosity_checks(args: &[String]) {
    let is_quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let is_verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let is_very_verbose = args.iter().any(|a| a == "-vv" || a == "--very-verbose");
    let is_debug = args.iter().any(|a| a == "-vvv" || a == "--debug");

    if is_quiet {
        println!("🔇 Quiet mode: Suppressing most output");
    } else if is_debug {
        println!("🔍 Debug mode: Showing all information");
    } else if is_very_verbose {
        println!("📊 Very verbose mode: Showing detailed information");
    } else if is_verbose {
        println!("📋 Verbose mode: Showing additional information");
    } else {
        println!("ℹ️  Normal mode: Standard output");
    }
}

fn example_command_execution(args: &[String]) {
    let is_quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let is_verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let is_very_verbose = args.iter().any(|a| a == "-vv" || a == "--very-verbose");
    let is_debug = args.iter().any(|a| a == "-vvv" || a == "--debug");

    println!("Simulating: foundry migrate {}", args.join(" "));
    println!();

    // Always shown
    println!("✓ Starting database migration");

    // Shown unless quiet
    if !is_quiet {
        println!("  Connected to database: postgresql://localhost/rustforge");
    }

    // Shown with -v
    if is_verbose {
        println!("  [VERBOSE] Checking for pending migrations...");
        println!("  [VERBOSE] Found 3 pending migrations");
    }

    // Shown with -vv
    if is_very_verbose {
        println!("  [VERY_VERBOSE] Migration 1: 2024_01_create_users");
        println!("  [VERY_VERBOSE] Migration 2: 2024_02_create_posts");
        println!("  [VERY_VERBOSE] Migration 3: 2024_03_add_timestamps");
    }

    // Shown with -vvv
    if is_debug {
        println!("  [DEBUG] Executing SQL: BEGIN TRANSACTION");
        println!("  [DEBUG] Row affected: 1");
        println!("  [DEBUG] Query time: 12ms");
        println!("  [DEBUG] Executing SQL: COMMIT");
        println!("  [DEBUG] Query time: 5ms");
    }

    // Always shown (success)
    if !is_quiet {
        println!("  ✓ Migration completed successfully");
    }
}

fn parse_verbosity_demo(args: &[String]) -> String {
    let mut level = "normal".to_string();

    for arg in args {
        match arg.as_str() {
            "-q" | "--quiet" => level = "quiet".to_string(),
            "-v" | "--verbose" => level = "verbose".to_string(),
            "-vv" | "--very-verbose" => level = "very_verbose".to_string(),
            "-vvv" | "--debug" => level = "debug".to_string(),
            _ => {}
        }
    }

    level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verbosity() {
        assert_eq!(
            parse_verbosity_demo(&["-v".to_string()]),
            "verbose"
        );

        assert_eq!(
            parse_verbosity_demo(&["-vv".to_string()]),
            "very_verbose"
        );

        assert_eq!(
            parse_verbosity_demo(&[]),
            "normal"
        );
    }
}
