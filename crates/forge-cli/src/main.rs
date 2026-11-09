use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use forge_cli::{aliases, commands, completion, config, errors, help};

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
        /// Use a local RustForge checkout in the generated Cargo manifest
        #[arg(long)]
        framework_path: Option<std::path::PathBuf>,
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
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to
        #[arg(long)]
        host: Option<String>,
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

    /// Generate deployment artifacts (Dockerfile, docker-compose.yml, K8s manifests)
    #[command(subcommand)]
    Deploy(DeployCommands),

    /// Show command aliases
    Aliases,

    /// Show enhanced documentation for a command
    #[command(name = "docs")]
    Docs {
        #[command(subcommand)]
        command: Option<DocsCommands>,
    },
}

#[derive(Subcommand)]
enum DocsCommands {
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
    Run {
        #[arg(long)]
        force: bool,
        /// Put each migration in its own batch
        #[arg(long)]
        step: bool,
    },

    /// Rollback the last batch of migrations
    Rollback {
        /// Number of individual migrations to roll back (defaults to the last batch)
        #[arg(long)]
        step: Option<usize>,
        /// Roll back a specific batch
        #[arg(long, conflicts_with = "step")]
        batch: Option<usize>,
        #[arg(long)]
        force: bool,
    },

    /// Rollback a specific batch
    RollbackBatch {
        /// Batch number to rollback
        batch: usize,
        #[arg(long)]
        force: bool,
    },

    /// Drop all tables and re-run all migrations
    Fresh {
        /// Also seed the database after migrating
        #[arg(long)]
        seed: bool,
        /// Confirm dropping every table in the target database
        #[arg(long)]
        force: bool,
    },

    /// Roll back all migrations
    Reset {
        #[arg(long)]
        force: bool,
    },

    /// Roll back and reapply migrations
    Refresh {
        /// Roll back only the last N migrations
        #[arg(long)]
        step: Option<usize>,
        #[arg(long)]
        seed: bool,
        #[arg(long)]
        force: bool,
    },

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

#[derive(Subcommand)]
enum DeployCommands {
    /// Generate deployment artifact files (Dockerfile, docker-compose.yml, and
    /// optionally Kubernetes manifests) in the current project directory.
    ///
    /// Kubernetes probes default to /up and /ready for a Forge starter and to
    /// /health/live and /health/ready for other applications.
    Generate {
        /// Application name used in service names and image tags
        #[arg(long, default_value = "app")]
        name: String,

        /// Port exposed by the application
        #[arg(short, long, default_value = "8000")]
        port: u16,

        /// Add a PostgreSQL service (provide version, e.g. "16")
        #[arg(long)]
        postgres: Option<String>,

        /// Add a Redis service
        #[arg(long)]
        redis: bool,

        /// Also generate Kubernetes deployment + service manifests in k8s/
        #[arg(long)]
        kubernetes: bool,

        /// Container image name for Kubernetes manifests (default: <name>:latest)
        #[arg(long)]
        image: Option<String>,

        /// Override the liveness probe HTTP path
        #[arg(long)]
        liveness_path: Option<String>,

        /// Override the readiness probe HTTP path
        #[arg(long)]
        readiness_path: Option<String>,
    },
}

// Accept Artisan's namespace:command syntax alongside Clap's nested commands.
fn artisan_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Vec<std::ffi::OsString> {
    let mut args: Vec<_> = args.into_iter().collect();
    if let Some((group, command)) = args
        .get(1)
        .and_then(|arg| arg.to_str())
        .and_then(|arg| arg.split_once(':'))
    {
        if matches!(
            group,
            "make" | "migrate" | "db" | "route" | "cache" | "config" | "queue"
        ) {
            let command = std::ffi::OsString::from(command);
            args[1] = group.into();
            args.insert(2, command);
        }
    } else if args.get(1).is_some_and(|arg| arg == "migrate")
        && (args.len() == 2
            || args
                .get(2)
                .is_some_and(|arg| arg == "--force" || arg == "--step"))
    {
        args.insert(2, "run".into());
    }
    args
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = config::ForgeConfig::load().unwrap_or_default();

    let cli = Cli::parse_from(artisan_args(std::env::args_os()));

    match cli.command {
        Commands::New {
            name,
            framework_path,
        } => {
            commands::new::run_with_framework(&name, framework_path.as_deref()).await?;
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
            MigrateCommands::Run { force, step } => {
                commands::migrate::run(force, step).await?;
            }
            MigrateCommands::Rollback { step, batch, force } => match batch {
                Some(batch) => commands::migrate::rollback_batch(batch, force).await?,
                None => commands::migrate::rollback(step, force).await?,
            },
            MigrateCommands::RollbackBatch { batch, force } => {
                commands::migrate::rollback_batch(batch, force).await?;
            }
            MigrateCommands::Fresh { seed, force } => {
                commands::migrate::fresh(seed, force).await?;
            }
            MigrateCommands::Reset { force } => {
                commands::migrate::reset(force).await?;
            }
            MigrateCommands::Refresh { step, seed, force } => {
                commands::migrate::refresh(step, seed, force).await?;
            }
            MigrateCommands::Status => {
                commands::migrate::status().await?;
            }
        },
        Commands::Db(db_cmd) => match db_cmd {
            DbCommands::Seed { class, force } => {
                commands::migrate::seed(class.as_deref(), force).await?;
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
        Commands::Deploy(deploy_cmd) => match deploy_cmd {
            DeployCommands::Generate {
                name,
                port,
                postgres,
                redis,
                kubernetes,
                image,
                liveness_path,
                readiness_path,
            } => {
                commands::deploy::generate(
                    &name,
                    port,
                    postgres.as_deref(),
                    redis,
                    kubernetes,
                    liveness_path.as_deref(),
                    readiness_path.as_deref(),
                    image.as_deref(),
                )
                .await?;
            }
        },
        Commands::Serve { port, host } => {
            commands::serve::run_configured(host.as_deref(), port).await?;
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
        Commands::Docs { command } => match command {
            Some(DocsCommands::MakeModel) => {
                help::make_model_help().display();
            }
            Some(DocsCommands::MakeController) => {
                help::make_controller_help().display();
            }
            Some(DocsCommands::Migrate) => {
                help::migrate_help().display();
            }
            None => {
                help::display_main_help();
            }
        },
    }

    Ok(())
}
