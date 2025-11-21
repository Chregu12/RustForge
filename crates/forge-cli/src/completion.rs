//! Shell completion generation
//!
//! This module provides shell completion script generation for bash, zsh, fish,
//! and PowerShell.

use clap::Command;
use clap_complete::{generate, Generator, Shell};
use std::io;

/// Generate completion script for the specified shell
pub fn generate_completion<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

/// Generate completion for the forge CLI
pub fn generate_for_shell(shell: Shell, cmd: &mut Command) {
    match shell {
        Shell::Bash => generate_completion(Shell::Bash, cmd),
        Shell::Zsh => generate_completion(Shell::Zsh, cmd),
        Shell::Fish => generate_completion(Shell::Fish, cmd),
        Shell::PowerShell => generate_completion(Shell::PowerShell, cmd),
        _ => {
            eprintln!("Unsupported shell");
        }
    }
}

/// Print installation instructions for the given shell
pub fn print_install_instructions(shell: Shell) {
    println!();
    match shell {
        Shell::Bash => {
            println!("# Bash completion installation:");
            println!();
            println!("  # On Linux:");
            println!("  forge completion bash > /etc/bash_completion.d/forge");
            println!();
            println!("  # On macOS:");
            println!("  forge completion bash > /usr/local/etc/bash_completion.d/forge");
            println!();
            println!("  # Or add to ~/.bashrc:");
            println!("  source <(forge completion bash)");
        }
        Shell::Zsh => {
            println!("# Zsh completion installation:");
            println!();
            println!("  # Add to ~/.zshrc:");
            println!("  autoload -U compinit; compinit");
            println!("  source <(forge completion zsh)");
            println!();
            println!("  # Or install to fpath:");
            println!("  forge completion zsh > /usr/local/share/zsh/site-functions/_forge");
            println!();
            println!("  # Make sure to add fpath before compinit in ~/.zshrc:");
            println!("  fpath=(/usr/local/share/zsh/site-functions $fpath)");
        }
        Shell::Fish => {
            println!("# Fish completion installation:");
            println!();
            println!("  forge completion fish > ~/.config/fish/completions/forge.fish");
            println!();
            println!("  # Or system-wide:");
            println!("  forge completion fish > /usr/share/fish/vendor_completions.d/forge.fish");
        }
        Shell::PowerShell => {
            println!("# PowerShell completion installation:");
            println!();
            println!("  # Add to your PowerShell profile:");
            println!("  forge completion powershell | Out-String | Invoke-Expression");
            println!();
            println!("  # To find your profile location:");
            println!("  echo $PROFILE");
        }
        _ => {
            println!("Unsupported shell");
        }
    }
    println!();
}

/// Get shell from environment or string
pub fn parse_shell(shell_str: &str) -> Option<Shell> {
    match shell_str.to_lowercase().as_str() {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        "powershell" | "pwsh" => Some(Shell::PowerShell),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shell_bash() {
        assert_eq!(parse_shell("bash"), Some(Shell::Bash));
        assert_eq!(parse_shell("BASH"), Some(Shell::Bash));
    }

    #[test]
    fn test_parse_shell_zsh() {
        assert_eq!(parse_shell("zsh"), Some(Shell::Zsh));
        assert_eq!(parse_shell("ZSH"), Some(Shell::Zsh));
    }

    #[test]
    fn test_parse_shell_fish() {
        assert_eq!(parse_shell("fish"), Some(Shell::Fish));
        assert_eq!(parse_shell("FISH"), Some(Shell::Fish));
    }

    #[test]
    fn test_parse_shell_powershell() {
        assert_eq!(parse_shell("powershell"), Some(Shell::PowerShell));
        assert_eq!(parse_shell("pwsh"), Some(Shell::PowerShell));
        assert_eq!(parse_shell("POWERSHELL"), Some(Shell::PowerShell));
    }

    #[test]
    fn test_parse_shell_invalid() {
        assert_eq!(parse_shell("invalid"), None);
        assert_eq!(parse_shell(""), None);
    }
}
