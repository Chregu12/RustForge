//! Enhanced help system with examples and tips
//!
//! This module provides rich help output with formatting, examples,
//! and cross-references to related commands.

use console::style;

/// Command help information
#[derive(Debug, Clone)]
pub struct CommandHelp {
    pub name: String,
    pub description: String,
    pub usage: Vec<String>,
    pub arguments: Vec<Argument>,
    pub options: Vec<CommandOption>,
    pub examples: Vec<Example>,
    pub see_also: Vec<String>,
    pub tips: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct CommandOption {
    pub short: Option<String>,
    pub long: String,
    pub description: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Example {
    pub description: String,
    pub command: String,
}

impl CommandHelp {
    pub fn display(&self) {
        // Header
        let border = "─".repeat(self.name.len() + 14);
        println!();
        println!("┌{}┐", border);
        println!("│  {}  │", style(format!("forge {}", self.name)).bold().cyan());
        println!("├{}┤", border);
        println!("│  {}  │", self.description);
        println!("└{}┘", border);
        println!();

        // Usage
        if !self.usage.is_empty() {
            println!("{}", style("Usage:").bold().green());
            for usage in &self.usage {
                println!("  {}", usage);
            }
            println!();
        }

        // Arguments
        if !self.arguments.is_empty() {
            println!("{}", style("Arguments:").bold().green());
            for arg in &self.arguments {
                let required = if arg.required { " (required)" } else { "" };
                println!("  {}{}", style(&arg.name).cyan().bold(), style(required).red());
                println!("    {}", arg.description);
            }
            println!();
        }

        // Options
        if !self.options.is_empty() {
            println!("{}", style("Options:").bold().green());
            for opt in &self.options {
                let short = opt.short
                    .as_ref()
                    .map(|s| format!("-{}, ", s))
                    .unwrap_or_default();
                let default_str = opt.default
                    .as_ref()
                    .map(|d| format!(" (default: {})", style(d).yellow()))
                    .unwrap_or_default();

                println!("  {}--{}{}", short, style(&opt.long).cyan(), default_str);
                println!("    {}", opt.description);
            }
            println!();
        }

        // Examples
        if !self.examples.is_empty() {
            println!("{}", style("Examples:").bold().green());
            for example in &self.examples {
                println!("  # {}", example.description);
                println!("  {}", style(&example.command).cyan());
                println!();
            }
        }

        // Tips
        if !self.tips.is_empty() {
            println!("{}", style("Tips:").bold().yellow());
            for tip in &self.tips {
                println!("  {} {}", style("•").yellow(), tip);
            }
            println!();
        }

        // See also
        if !self.see_also.is_empty() {
            println!("{}", style("See also:").bold().blue());
            for cmd in &self.see_also {
                println!("  {} forge {}", style("•").blue(), style(cmd).cyan());
            }
            println!();
        }
    }
}

/// Get help for make:model command
pub fn make_model_help() -> CommandHelp {
    CommandHelp {
        name: "make:model".to_string(),
        description: "Create a new Eloquent model".to_string(),
        usage: vec![
            "forge make:model <NAME> [OPTIONS]".to_string(),
            "forge make:model  (interactive mode)".to_string(),
        ],
        arguments: vec![
            Argument {
                name: "<NAME>".to_string(),
                description: "The name of the model (e.g., User, BlogPost)".to_string(),
                required: false,
            },
        ],
        options: vec![
            CommandOption {
                short: Some("m".to_string()),
                long: "migration".to_string(),
                description: "Create a migration file".to_string(),
                default: None,
            },
            CommandOption {
                short: Some("f".to_string()),
                long: "factory".to_string(),
                description: "Create a factory file".to_string(),
                default: None,
            },
            CommandOption {
                short: Some("s".to_string()),
                long: "seeder".to_string(),
                description: "Create a seeder file".to_string(),
                default: None,
            },
            CommandOption {
                short: None,
                long: "timestamps".to_string(),
                description: "Add created_at/updated_at fields".to_string(),
                default: Some("true".to_string()),
            },
            CommandOption {
                short: None,
                long: "soft-delete".to_string(),
                description: "Add soft delete support".to_string(),
                default: Some("false".to_string()),
            },
        ],
        examples: vec![
            Example {
                description: "Create a simple model".to_string(),
                command: "forge make:model User".to_string(),
            },
            Example {
                description: "Create model with migration and factory".to_string(),
                command: "forge make:model Post --migration --factory".to_string(),
            },
            Example {
                description: "Create model with all features".to_string(),
                command: "forge make:model Product -m -f -s --soft-delete".to_string(),
            },
            Example {
                description: "Interactive mode (recommended)".to_string(),
                command: "forge make:model".to_string(),
            },
        ],
        see_also: vec![
            "make:migration".to_string(),
            "make:factory".to_string(),
            "make:controller".to_string(),
        ],
        tips: vec![
            "Model names should be singular and PascalCase (e.g., User, BlogPost)".to_string(),
            "Table names will be automatically pluralized (User -> users)".to_string(),
            "Use interactive mode to get helpful prompts and validation".to_string(),
        ],
    }
}

/// Get help for make:controller command
pub fn make_controller_help() -> CommandHelp {
    CommandHelp {
        name: "make:controller".to_string(),
        description: "Create a new controller".to_string(),
        usage: vec![
            "forge make:controller <NAME> [OPTIONS]".to_string(),
            "forge make:controller  (interactive mode)".to_string(),
        ],
        arguments: vec![
            Argument {
                name: "<NAME>".to_string(),
                description: "The name of the controller (e.g., UserController)".to_string(),
                required: false,
            },
        ],
        options: vec![
            CommandOption {
                short: None,
                long: "api".to_string(),
                description: "Generate an API controller (JSON responses)".to_string(),
                default: None,
            },
            CommandOption {
                short: Some("r".to_string()),
                long: "resource".to_string(),
                description: "Generate a resource controller (CRUD methods)".to_string(),
                default: None,
            },
            CommandOption {
                short: None,
                long: "invokable".to_string(),
                description: "Generate an invokable controller (single action)".to_string(),
                default: None,
            },
        ],
        examples: vec![
            Example {
                description: "Create a basic controller".to_string(),
                command: "forge make:controller UserController".to_string(),
            },
            Example {
                description: "Create an API controller".to_string(),
                command: "forge make:controller Api/PostController --api".to_string(),
            },
            Example {
                description: "Create a resource controller".to_string(),
                command: "forge make:controller ProductController --resource".to_string(),
            },
            Example {
                description: "Interactive mode (recommended)".to_string(),
                command: "forge make:controller".to_string(),
            },
        ],
        see_also: vec![
            "make:model".to_string(),
            "make:request".to_string(),
            "route:list".to_string(),
        ],
        tips: vec![
            "Controller names should end with 'Controller' (e.g., UserController)".to_string(),
            "Use --resource for standard CRUD operations".to_string(),
            "API controllers return JSON responses by default".to_string(),
        ],
    }
}

/// Get help for migrate command
pub fn migrate_help() -> CommandHelp {
    CommandHelp {
        name: "migrate".to_string(),
        description: "Run database migrations".to_string(),
        usage: vec![
            "forge migrate [SUBCOMMAND]".to_string(),
        ],
        arguments: vec![],
        options: vec![
            CommandOption {
                short: None,
                long: "step".to_string(),
                description: "Number of migrations to run".to_string(),
                default: None,
            },
            CommandOption {
                short: None,
                long: "force".to_string(),
                description: "Force migrations in production".to_string(),
                default: None,
            },
        ],
        examples: vec![
            Example {
                description: "Run all pending migrations".to_string(),
                command: "forge migrate".to_string(),
            },
            Example {
                description: "Rollback last migration".to_string(),
                command: "forge migrate rollback".to_string(),
            },
            Example {
                description: "Drop all tables and re-run migrations".to_string(),
                command: "forge migrate fresh".to_string(),
            },
            Example {
                description: "Fresh migrations and seed database".to_string(),
                command: "forge migrate fresh --seed".to_string(),
            },
            Example {
                description: "Check migration status".to_string(),
                command: "forge migrate status".to_string(),
            },
        ],
        see_also: vec![
            "make:migration".to_string(),
            "db:seed".to_string(),
        ],
        tips: vec![
            "Always backup your database before running migrations in production".to_string(),
            "Use 'migrate fresh' in development to reset your database".to_string(),
            "Check 'migrate status' to see which migrations have run".to_string(),
        ],
    }
}

/// Display help for all commands (main help screen)
pub fn display_main_help() {
    println!();
    println!("┌─────────────────────────────────────────────┐");
    println!("│  {}  │", style("RustForge CLI").bold().cyan());
    println!("├─────────────────────────────────────────────┤");
    println!("│  Laravel-inspired development tool for Rust │");
    println!("└─────────────────────────────────────────────┘");
    println!();

    println!("{}", style("Usage:").bold().green());
    println!("  forge <COMMAND> [OPTIONS]");
    println!();

    println!("{}", style("Commands:").bold().green());
    println!();

    // Project Management
    println!("  {}", style("Project Management:").yellow().bold());
    println!("    {}  Create a new RustForge project", style("new").cyan());
    println!("    {}  Show framework information", style("about").cyan());
    println!("    {}  Optimize application for production", style("optimize").cyan());
    println!();

    // Code Generation
    println!("  {}", style("Code Generation (make:*):").yellow().bold());
    println!("    {}  Generate a new model", style("make:model").cyan());
    println!("    {}  Generate a new controller", style("make:controller").cyan());
    println!("    {}  Generate a new migration", style("make:migration").cyan());
    println!("    {}  Generate a new factory", style("make:factory").cyan());
    println!("    {}  Generate a new seeder", style("make:seeder").cyan());
    println!("    {}  Generate a new request", style("make:request").cyan());
    println!("    {}  Generate a new policy", style("make:policy").cyan());
    println!("    {}  Generate a new job", style("make:job").cyan());
    println!("    {}  Generate a new event", style("make:event").cyan());
    println!("    {}  Generate a new listener", style("make:listener").cyan());
    println!("    {}  Generate a new middleware", style("make:middleware").cyan());
    println!();

    // Database
    println!("  {}", style("Database:").yellow().bold());
    println!("    {}  Run database migrations", style("migrate").cyan());
    println!("    {}  Seed the database", style("db:seed").cyan());
    println!();

    // Development
    println!("  {}", style("Development:").yellow().bold());
    println!("    {}  Start development server", style("serve").cyan());
    println!("    {}  Interactive REPL", style("tinker").cyan());
    println!();

    // System
    println!("  {}", style("System:").yellow().bold());
    println!("    {}  List all routes", style("route:list").cyan());
    println!("    {}  Manage cache", style("cache:clear").cyan());
    println!("    {}  Manage queue", style("queue:work").cyan());
    println!("    {}  Generate shell completions", style("completion").cyan());
    println!();

    println!("{}", style("Options:").bold().green());
    println!("  {}, {}  Print help information", style("-h").cyan(), style("--help").cyan());
    println!("  {}, {}  Print version information", style("-V").cyan(), style("--version").cyan());
    println!();

    println!("{}", style("Examples:").bold().green());
    println!("  # Create a new project");
    println!("  {}", style("forge new my-app").cyan());
    println!();
    println!("  # Generate a model with migration and factory");
    println!("  {}", style("forge make:model User --migration --factory").cyan());
    println!();
    println!("  # Run migrations and seed database");
    println!("  {}", style("forge migrate fresh --seed").cyan());
    println!();
    println!("  # Start development server");
    println!("  {}", style("forge serve").cyan());
    println!();

    println!("{}", style("For more information on a specific command:").bold());
    println!("  forge <COMMAND> --help");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_help_creation() {
        let help = make_model_help();
        assert_eq!(help.name, "make:model");
        assert!(!help.description.is_empty());
        assert!(!help.usage.is_empty());
        assert!(!help.examples.is_empty());
    }

    #[test]
    fn test_argument_creation() {
        let arg = Argument {
            name: "NAME".to_string(),
            description: "Test".to_string(),
            required: true,
        };
        assert_eq!(arg.name, "NAME");
        assert!(arg.required);
    }

    #[test]
    fn test_option_creation() {
        let opt = CommandOption {
            short: Some("m".to_string()),
            long: "migration".to_string(),
            description: "Create migration".to_string(),
            default: None,
        };
        assert_eq!(opt.short, Some("m".to_string()));
        assert_eq!(opt.long, "migration");
    }

    #[test]
    fn test_example_creation() {
        let example = Example {
            description: "Test".to_string(),
            command: "forge test".to_string(),
        };
        assert_eq!(example.command, "forge test");
    }

    #[test]
    fn test_make_controller_help() {
        let help = make_controller_help();
        assert_eq!(help.name, "make:controller");
        assert!(!help.examples.is_empty());
        assert!(!help.tips.is_empty());
    }

    #[test]
    fn test_migrate_help() {
        let help = migrate_help();
        assert_eq!(help.name, "migrate");
        assert!(!help.examples.is_empty());
        assert!(!help.see_also.is_empty());
    }
}
