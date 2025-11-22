mod aliases;
mod commands;
mod completion;
mod config;
mod errors;
mod help;
mod interactive;
mod progress;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use colored::*;

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "RustForge CLI - Laravel-inspired development tool", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new RustForge project
    New {
        /// Name of the new project
        name: String,
    },

    /// Generate code scaffolding
    #[command(subcommand)]
    Make(MakeCommands),

    /// Run database migrations
    #[command(subcommand)]
    Migrate(MigrateCommands),

    /// Database operations
    #[command(subcommand)]
    Db(DbCommands),

    /// Route management
    #[command(subcommand)]
    Route(RouteCommands),

    /// Cache management
    #[command(subcommand)]
    Cache(CacheCommands),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Queue management
    #[command(subcommand)]
    Queue(QueueCommands),

    /// Start the development server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },

    /// Interactive REPL for the application
    Tinker,

    /// Optimize the application for better performance
    Optimize,

    /// Display an inspiring quote
    Inspire,

    /// Show RustForge version and info
    About,

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for (bash, zsh, fish, powershell)
        shell: String,
    },

    /// Show command aliases
    Aliases,

    /// Show enhanced help for a command
    Help {
        #[command(subcommand)]
        command: Option<HelpCommands>,
    },
}

#[derive(Subcommand)]
enum HelpCommands {
    /// Show help for make:model
    MakeModel,
    /// Show help for make:controller
    MakeController,
    /// Show help for migrate
    Migrate,
}

#[derive(Subcommand)]
enum MakeCommands {
    /// Generate a new model
    Model {
        /// Name of the model (e.g., User, Post)
        name: String,

        /// Also create a migration
        #[arg(short, long)]
        migration: bool,
    },

    /// Generate a new controller
    Controller {
        /// Name of the controller (e.g., UserController)
        name: String,

        /// Generate an API controller
        #[arg(long)]
        api: bool,
    },

    /// Generate a new migration
    Migration {
        /// Name of the migration (e.g., create_users_table)
        name: String,
    },

    /// Generate a new command
    Command {
        /// Name of the command (e.g., SendEmails)
        name: String,
    },

    /// Generate a new factory
    Factory {
        /// Name of the factory (e.g., UserFactory)
        name: String,

        /// Model name (if different from factory name)
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Generate a new seeder
    Seeder {
        /// Name of the seeder (e.g., UserSeeder)
        name: String,
    },

    /// Generate a new form request
    Request {
        /// Name of the request (e.g., StorePostRequest)
        name: String,
    },

    /// Generate a new policy
    Policy {
        /// Name of the policy (e.g., PostPolicy)
        name: String,

        /// Model name
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Generate a new event
    Event {
        /// Name of the event (e.g., PostCreated)
        name: String,
    },

    /// Generate a new event listener
    Listener {
        /// Name of the listener (e.g., SendPostNotification)
        name: String,

        /// Event to listen to
        #[arg(short, long)]
        event: Option<String>,
    },

    /// Generate a new job
    Job {
        /// Name of the job (e.g., ProcessPost)
        name: String,

        /// Queue name
        #[arg(short, long)]
        queue: Option<String>,
    },

    /// Generate a new mailable
    Mail {
        /// Name of the mailable (e.g., PostPublished)
        name: String,
    },

    /// Generate a new notification
    Notification {
        /// Name of the notification (e.g., PostPublished)
        name: String,
    },

    /// Generate a new API resource
    Resource {
        /// Name of the resource (e.g., PostResource)
        name: String,

        /// Also generate a resource collection
        #[arg(short, long)]
        collection: bool,
    },

    /// Generate a new test
    Test {
        /// Name of the test (e.g., PostTest)
        name: String,

        /// Generate unit test instead of integration test
        #[arg(short, long)]
        unit: bool,
    },

    /// Generate a new middleware
    Middleware {
        /// Name of the middleware (e.g., AuthMiddleware)
        name: String,
    },
}

#[derive(Subcommand)]
enum MigrateCommands {
    /// Run pending migrations
    Run,

    /// Rollback the last batch of migrations
    Rollback {
        /// Number of batches to rollback (defaults to 1)
        #[arg(long)]
        step: Option<usize>,
    },

    /// Rollback a specific batch
    RollbackBatch {
        /// Batch number to rollback
        batch: usize,
    },

    /// Drop all tables and re-run all migrations
    Fresh {
        /// Also seed the database after migrating
        #[arg(long)]
        seed: bool,
    },

    /// Reset and re-run all migrations
    Reset,

    /// Show the status of each migration
    Status,
}

#[derive(Subcommand)]
enum DbCommands {
    /// Seed the database with records
    Seed {
        /// Run a specific seeder
        #[arg(long)]
        class: Option<String>,

        /// Force seed in production
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum RouteCommands {
    /// List all registered routes
    List {
        /// Filter by method (GET, POST, etc.)
        #[arg(long)]
        method: Option<String>,

        /// Filter by path pattern
        #[arg(long)]
        path: Option<String>,
    },

    /// Cache routes for faster registration
    Cache,

    /// Clear the route cache
    Clear,
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Clear all cache
    Clear {
        /// Clear specific store
        #[arg(long)]
        store: Option<String>,
    },

    /// Forget a specific cache key
    Forget {
        /// Key to forget
        key: String,

        /// Store to use
        #[arg(long)]
        store: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Cache the configuration files
    Cache,

    /// Clear the config cache
    Clear,
}

#[derive(Subcommand)]
enum QueueCommands {
    /// Start processing jobs on the queue
    Work {
        /// Queue to process
        #[arg(long)]
        queue: Option<String>,

        /// Number of times to try a job
        #[arg(long)]
        tries: Option<u32>,

        /// Maximum seconds a job may run
        #[arg(long)]
        timeout: Option<u64>,

        /// Number of jobs to process before stopping
        #[arg(long)]
        max_jobs: Option<u32>,

        /// Memory limit in MB
        #[arg(long)]
        memory: Option<u32>,
    },

    /// Listen to the queue and process jobs
    Listen {
        /// Queue to listen to
        #[arg(long)]
        queue: Option<String>,
    },

    /// Retry a failed job
    Retry {
        /// ID of the job to retry (use 'all' to retry all)
        id: String,

        /// Queue name
        #[arg(long)]
        queue: Option<String>,
    },

    /// List failed jobs
    Failed {
        /// Queue name
        #[arg(long)]
        queue: Option<String>,
    },

    /// Flush all failed jobs
    Flush {
        /// Hours ago to flush
        #[arg(long)]
        hours: Option<u32>,

        /// Queue name
        #[arg(long)]
        queue: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = config::ForgeConfig::load().unwrap_or_default();

    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            commands::new::run(&name).await?;
        }
        Commands::Make(make_cmd) => match make_cmd {
            MakeCommands::Model { name, migration } => {
                if name.is_empty() && config.cli.interactive {
                    // Interactive mode
                    commands::make::model_interactive().await?;
                } else {
                    commands::make::model(&name, migration).await?;
                }
            }
            MakeCommands::Controller { name, api } => {
                if name.is_empty() && config.cli.interactive {
                    // Interactive mode
                    commands::make::controller_interactive().await?;
                } else {
                    commands::make::controller(&name, api).await?;
                }
            }
            MakeCommands::Migration { name } => {
                commands::make::migration(&name).await?;
            }
            MakeCommands::Command { name } => {
                commands::make::command(&name).await?;
            }
            MakeCommands::Factory { name, model } => {
                commands::make::factory(&name, model.as_deref()).await?;
            }
            MakeCommands::Seeder { name } => {
                commands::make::seeder(&name).await?;
            }
            MakeCommands::Request { name } => {
                commands::make::request(&name).await?;
            }
            MakeCommands::Policy { name, model } => {
                commands::make::policy(&name, model.as_deref()).await?;
            }
            MakeCommands::Event { name } => {
                commands::make::event(&name).await?;
            }
            MakeCommands::Listener { name, event } => {
                commands::make::listener(&name, event.as_deref()).await?;
            }
            MakeCommands::Job { name, queue } => {
                commands::make::job(&name, queue.as_deref()).await?;
            }
            MakeCommands::Mail { name } => {
                commands::make::mail(&name).await?;
            }
            MakeCommands::Notification { name } => {
                commands::make::notification(&name).await?;
            }
            MakeCommands::Resource { name, collection } => {
                commands::make::resource(&name, collection).await?;
            }
            MakeCommands::Test { name, unit } => {
                commands::make::test(&name, unit).await?;
            }
            MakeCommands::Middleware { name } => {
                commands::make::middleware(&name).await?;
            }
        },
        Commands::Migrate(migrate_cmd) => match migrate_cmd {
            MigrateCommands::Run => {
                commands::migrate::run().await?;
            }
            MigrateCommands::Rollback { step } => {
                commands::migrate::rollback(step).await?;
            }
            MigrateCommands::RollbackBatch { batch } => {
                commands::migrate::rollback_batch(batch).await?;
            }
            MigrateCommands::Fresh { seed } => {
                commands::migrate::fresh(seed).await?;
            }
            MigrateCommands::Reset => {
                commands::migrate::reset().await?;
            }
            MigrateCommands::Status => {
                commands::migrate::status().await?;
            }
        },
        Commands::Db(db_cmd) => match db_cmd {
            DbCommands::Seed { class, force } => {
                commands::make::seed(class.as_deref(), force).await?;
            }
        },
        Commands::Route(route_cmd) => match route_cmd {
            RouteCommands::List { method, path } => {
                commands::route::list(method.as_deref(), path.as_deref()).await?;
            }
            RouteCommands::Cache => {
                commands::route::cache().await?;
            }
            RouteCommands::Clear => {
                commands::route::clear().await?;
            }
        },
        Commands::Cache(cache_cmd) => match cache_cmd {
            CacheCommands::Clear { store } => {
                commands::cache::clear(store.as_deref()).await?;
            }
            CacheCommands::Forget { key, store } => {
                commands::cache::forget(&key, store.as_deref()).await?;
            }
        },
        Commands::Config(config_cmd) => match config_cmd {
            ConfigCommands::Cache => {
                commands::config::cache().await?;
            }
            ConfigCommands::Clear => {
                commands::config::clear().await?;
            }
        },
        Commands::Queue(queue_cmd) => match queue_cmd {
            QueueCommands::Work {
                queue,
                tries,
                timeout,
                max_jobs,
                memory,
            } => {
                commands::queue::work(queue.as_deref(), tries, timeout, max_jobs, memory).await?;
            }
            QueueCommands::Listen { queue } => {
                commands::queue::listen(queue.as_deref()).await?;
            }
            QueueCommands::Retry { id, queue } => {
                commands::queue::retry(&id, queue.as_deref()).await?;
            }
            QueueCommands::Failed { queue } => {
                commands::queue::failed(queue.as_deref()).await?;
            }
            QueueCommands::Flush { hours, queue } => {
                commands::queue::flush(hours, queue.as_deref()).await?;
            }
        },
        Commands::Serve { port, host } => {
            commands::serve::run(&host, port).await?;
        }
        Commands::Tinker => {
            commands::tinker::run().await?;
        }
        Commands::Optimize => {
            commands::optimize::run().await?;
        }
        Commands::Inspire => {
            commands::inspire::run();
        }
        Commands::About => {
            commands::about::run();
        }
        Commands::Completion { shell } => {
            if let Some(shell) = completion::parse_shell(&shell) {
                let mut cmd = Cli::command();
                completion::generate_for_shell(shell, &mut cmd);
                println!();
                completion::print_install_instructions(shell);
            } else {
                errors::print_error(&format!(
                    "Unknown shell: {}. Supported shells: bash, zsh, fish, powershell",
                    shell
                ));
                std::process::exit(1);
            }
        }
        Commands::Aliases => {
            aliases::display_aliases(&config.aliases);
        }
        Commands::Help { command } => match command {
            Some(HelpCommands::MakeModel) => {
                help::make_model_help().display();
            }
            Some(HelpCommands::MakeController) => {
                help::make_controller_help().display();
            }
            Some(HelpCommands::Migrate) => {
                help::migrate_help().display();
            }
            None => {
                help::display_main_help();
            }
        },
    }

    Ok(())
}
