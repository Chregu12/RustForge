//! `forge list` — list all available forge commands

use colored::*;

/// A single command entry shown in the list output.
struct CommandEntry {
    name: &'static str,
    description: &'static str,
}

impl CommandEntry {
    const fn new(name: &'static str, description: &'static str) -> Self {
        Self { name, description }
    }
}

/// All commands grouped by category.
struct CommandGroup {
    title: &'static str,
    commands: &'static [CommandEntry],
}

static GROUPS: &[CommandGroup] = &[
    CommandGroup {
        title: "Project",
        commands: &[
            CommandEntry::new("new <name>", "Create a new RustForge project"),
            CommandEntry::new("serve [--port <PORT>]", "Start the development server"),
            CommandEntry::new("about", "Show RustForge version and info"),
            CommandEntry::new("inspire", "Display an inspiring quote"),
            CommandEntry::new("tinker", "Interactive REPL for the application"),
            CommandEntry::new("optimize", "Optimize the application for production"),
        ],
    },
    CommandGroup {
        title: "Code Generation  (make:*)",
        commands: &[
            CommandEntry::new("make:model <Name> [--migration]", "Generate a SeaORM entity model"),
            CommandEntry::new("make:controller <Name> [--api]", "Generate an Axum controller handler"),
            CommandEntry::new("make:migration <name>", "Generate a SeaORM migration file"),
            CommandEntry::new("make:seeder <Name>", "Generate a database seeder"),
            CommandEntry::new("make:command <Name>", "Generate a CLI command"),
            CommandEntry::new("make:factory <Name> [--model <Model>]", "Generate a model factory"),
            CommandEntry::new("make:request <Name>", "Generate a form request validator"),
            CommandEntry::new("make:policy <Name> [--model <Model>]", "Generate a policy class"),
            CommandEntry::new("make:event <Name>", "Generate an event"),
            CommandEntry::new("make:listener <Name> [--event <Event>]", "Generate an event listener"),
            CommandEntry::new("make:job <Name> [--queue <queue>]", "Generate a background job"),
            CommandEntry::new("make:mail <Name>", "Generate a mailable class"),
            CommandEntry::new("make:notification <Name>", "Generate a notification"),
            CommandEntry::new("make:resource <Name> [--collection]", "Generate an API resource"),
            CommandEntry::new("make:test <Name> [--unit]", "Generate a test file"),
            CommandEntry::new("make:middleware <Name>", "Generate middleware"),
        ],
    },
    CommandGroup {
        title: "Database",
        commands: &[
            CommandEntry::new("migrate:run", "Run pending migrations"),
            CommandEntry::new("migrate:rollback [--step <N>]", "Rollback the last migration batch"),
            CommandEntry::new("migrate:fresh [--seed]", "Drop all tables and re-run migrations"),
            CommandEntry::new("migrate:reset", "Reset and re-run all migrations"),
            CommandEntry::new("migrate:status", "Show the status of each migration"),
            CommandEntry::new("db:seed [--class <Seeder>]", "Seed the database with records"),
        ],
    },
    CommandGroup {
        title: "Routing",
        commands: &[
            CommandEntry::new("route:list [--method <M>] [--path <P>]", "List all registered routes"),
            CommandEntry::new("route:cache", "Cache routes for faster registration"),
            CommandEntry::new("route:clear", "Clear the route cache"),
        ],
    },
    CommandGroup {
        title: "Cache",
        commands: &[
            CommandEntry::new("cache:clear [--store <store>]", "Clear all cache"),
            CommandEntry::new("cache:forget <key>", "Forget a specific cache key"),
        ],
    },
    CommandGroup {
        title: "Configuration",
        commands: &[
            CommandEntry::new("config:cache", "Cache the configuration files"),
            CommandEntry::new("config:clear", "Clear the config cache"),
        ],
    },
    CommandGroup {
        title: "Queue",
        commands: &[
            CommandEntry::new("queue:work [--queue <Q>]", "Start processing jobs on the queue"),
            CommandEntry::new("queue:listen [--queue <Q>]", "Listen to the queue"),
            CommandEntry::new("queue:retry <id>", "Retry a failed job"),
            CommandEntry::new("queue:failed", "List failed jobs"),
            CommandEntry::new("queue:flush", "Flush all failed jobs"),
        ],
    },
    CommandGroup {
        title: "Utilities",
        commands: &[
            CommandEntry::new("list", "List all available commands"),
            CommandEntry::new("completion <shell>", "Generate shell completion scripts"),
            CommandEntry::new("aliases", "Show command aliases"),
            CommandEntry::new("docs [<command>]", "Show extended documentation"),
        ],
    },
];

/// Print the full command list.
pub fn run() {
    println!();
    println!(
        "{}",
        "RustForge — Laravel-inspired Rust Framework"
            .bright_cyan()
            .bold()
    );
    println!();
    println!("{}", "Usage:".yellow().bold());
    println!("  forge <command> [arguments] [options]");
    println!();

    for group in GROUPS {
        println!("{}", format!("{}:", group.title).cyan().bold());
        for cmd in group.commands {
            println!(
                "  {:<48} {}",
                format!("forge {}", cmd.name).green(),
                cmd.description.bright_black()
            );
        }
        println!();
    }

    println!(
        "{}",
        "Run `forge <command> --help` for details on any command."
            .bright_black()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_groups_have_commands() {
        for group in GROUPS {
            assert!(
                !group.commands.is_empty(),
                "Group '{}' has no commands",
                group.title
            );
        }
    }

    #[test]
    fn test_all_command_names_non_empty() {
        for group in GROUPS {
            for cmd in group.commands {
                assert!(!cmd.name.is_empty(), "Command name must not be empty");
                assert!(
                    !cmd.description.is_empty(),
                    "Description for '{}' must not be empty",
                    cmd.name
                );
            }
        }
    }

    #[test]
    fn test_make_commands_are_listed() {
        let all_names: Vec<&str> = GROUPS
            .iter()
            .flat_map(|g| g.commands.iter())
            .map(|c| c.name)
            .collect();

        let required = [
            "make:model",
            "make:controller",
            "make:migration",
            "make:seeder",
        ];
        for req in &required {
            assert!(
                all_names.iter().any(|n| n.contains(req)),
                "Expected '{}' in command list",
                req
            );
        }
    }
}
