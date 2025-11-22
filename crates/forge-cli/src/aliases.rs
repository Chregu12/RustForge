//! Command aliases system
//!
//! This module provides short aliases for common commands and support for
//! user-defined aliases via .forge.toml configuration.

use std::collections::HashMap;

/// Built-in command aliases
pub fn get_builtin_aliases() -> HashMap<String, String> {
    let mut aliases = HashMap::new();

    // Make shortcuts
    aliases.insert("m:m".to_string(), "make:model".to_string());
    aliases.insert("m:c".to_string(), "make:controller".to_string());
    aliases.insert("m:mg".to_string(), "make:migration".to_string());
    aliases.insert("m:f".to_string(), "make:factory".to_string());
    aliases.insert("m:s".to_string(), "make:seeder".to_string());
    aliases.insert("m:r".to_string(), "make:request".to_string());
    aliases.insert("m:p".to_string(), "make:policy".to_string());
    aliases.insert("m:j".to_string(), "make:job".to_string());
    aliases.insert("m:e".to_string(), "make:event".to_string());
    aliases.insert("m:l".to_string(), "make:listener".to_string());
    aliases.insert("m:md".to_string(), "make:middleware".to_string());

    // Database shortcuts
    aliases.insert("mg".to_string(), "migrate".to_string());
    aliases.insert("mg:fresh".to_string(), "migrate fresh --seed".to_string());
    aliases.insert("mg:reset".to_string(), "migrate reset".to_string());
    aliases.insert("db:s".to_string(), "db:seed".to_string());

    // Route shortcuts
    aliases.insert("r:l".to_string(), "route:list".to_string());

    // Cache shortcuts
    aliases.insert("c:c".to_string(), "cache:clear".to_string());

    // Queue shortcuts
    aliases.insert("q:w".to_string(), "queue:work".to_string());
    aliases.insert("q:l".to_string(), "queue:listen".to_string());
    aliases.insert("q:f".to_string(), "queue:failed".to_string());

    // Other shortcuts
    aliases.insert("serve".to_string(), "serve --port 8000".to_string());
    aliases.insert("s".to_string(), "serve".to_string());
    aliases.insert("t".to_string(), "tinker".to_string());

    aliases
}

/// Expand an alias to its full command
pub fn expand_alias(input: &str, user_aliases: &HashMap<String, String>) -> String {
    let builtin = get_builtin_aliases();

    // First check user aliases (they take precedence)
    if let Some(expanded) = user_aliases.get(input) {
        return expanded.clone();
    }

    // Then check builtin aliases
    if let Some(expanded) = builtin.get(input) {
        return expanded.clone();
    }

    // No alias found, return original
    input.to_string()
}

/// Get all aliases (builtin + user)
pub fn get_all_aliases(user_aliases: &HashMap<String, String>) -> HashMap<String, String> {
    let mut all = get_builtin_aliases();
    all.extend(user_aliases.clone());
    all
}

/// Display all available aliases
pub fn display_aliases(user_aliases: &HashMap<String, String>) {
    use colored::*;

    println!();
    println!("{}", "Available Command Aliases:".bold().cyan());
    println!();

    // Display builtin aliases
    println!("{}", "Built-in Aliases:".yellow().bold());
    let builtin = get_builtin_aliases();
    let mut builtin_vec: Vec<_> = builtin.iter().collect();
    builtin_vec.sort_by_key(|(k, _)| k.as_str());

    for (alias, command) in builtin_vec {
        println!("  {} {} {}", alias.green(), "→".dimmed(), command.cyan());
    }

    // Display user aliases if any
    if !user_aliases.is_empty() {
        println!();
        println!("{}", "Custom Aliases (from .forge.toml):".yellow().bold());
        let mut user_vec: Vec<_> = user_aliases.iter().collect();
        user_vec.sort_by_key(|(k, _)| k.as_str());

        for (alias, command) in user_vec {
            println!("  {} {} {}", alias.green(), "→".dimmed(), command.cyan());
        }
    }

    println!();
    println!("{}", "Usage:".bold());
    println!("  forge <alias> [OPTIONS]");
    println!();
    println!("{}", "Examples:".bold());
    println!(
        "  {} {} {}",
        "forge m:m User".cyan(),
        "→".dimmed(),
        "forge make:model User".dimmed()
    );
    println!(
        "  {} {} {}",
        "forge mg:fresh".cyan(),
        "→".dimmed(),
        "forge migrate fresh --seed".dimmed()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_aliases_exist() {
        let aliases = get_builtin_aliases();
        assert!(aliases.contains_key("m:m"));
        assert!(aliases.contains_key("m:c"));
        assert!(aliases.contains_key("mg"));
    }

    #[test]
    fn test_builtin_alias_values() {
        let aliases = get_builtin_aliases();
        assert_eq!(aliases.get("m:m"), Some(&"make:model".to_string()));
        assert_eq!(aliases.get("m:c"), Some(&"make:controller".to_string()));
        assert_eq!(aliases.get("mg"), Some(&"migrate".to_string()));
    }

    #[test]
    fn test_expand_alias_builtin() {
        let user = HashMap::new();
        assert_eq!(expand_alias("m:m", &user), "make:model");
        assert_eq!(expand_alias("m:c", &user), "make:controller");
    }

    #[test]
    fn test_expand_alias_user() {
        let mut user = HashMap::new();
        user.insert("custom".to_string(), "some:command".to_string());

        assert_eq!(expand_alias("custom", &user), "some:command");
    }

    #[test]
    fn test_expand_alias_user_overrides_builtin() {
        let mut user = HashMap::new();
        user.insert("m:m".to_string(), "custom:model".to_string());

        assert_eq!(expand_alias("m:m", &user), "custom:model");
    }

    #[test]
    fn test_expand_alias_no_match() {
        let user = HashMap::new();
        assert_eq!(expand_alias("unknown", &user), "unknown");
    }

    #[test]
    fn test_get_all_aliases() {
        let mut user = HashMap::new();
        user.insert("custom".to_string(), "custom:command".to_string());

        let all = get_all_aliases(&user);
        assert!(all.contains_key("m:m"));
        assert!(all.contains_key("custom"));
    }

    #[test]
    fn test_get_all_aliases_user_overrides() {
        let mut user = HashMap::new();
        user.insert("m:m".to_string(), "custom:model".to_string());

        let all = get_all_aliases(&user);
        assert_eq!(all.get("m:m"), Some(&"custom:model".to_string()));
    }
}
