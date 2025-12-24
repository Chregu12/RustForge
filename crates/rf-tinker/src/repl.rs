//! Main REPL implementation

use crate::commands::CommandHandler;
use crate::completer::TinkerCompleter;
use crate::executor::{ExecutionContext, QueryExecutor};
use crate::formatter::OutputFormatter;
use crate::highlighter::TinkerHighlighter;
use colored::*;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Config, Context, Editor, Helper};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::borrow::Cow;
use std::sync::Arc;

/// Tinker configuration
#[derive(Debug, Clone)]
pub struct TinkerConfig {
    /// Database URL
    pub database_url: Option<String>,
    /// History file path
    pub history_file: Option<String>,
    /// Enable syntax highlighting
    pub highlighting: bool,
    /// Enable auto-completion
    pub completion: bool,
    /// Prompt string
    pub prompt: String,
    /// Max history entries
    pub max_history: usize,
}

impl Default for TinkerConfig {
    fn default() -> Self {
        Self {
            database_url: None,
            history_file: Some(".tinker_history".to_string()),
            highlighting: true,
            completion: true,
            prompt: "Tinker> ".to_string(),
            max_history: 1000,
        }
    }
}

/// Helper struct for rustyline
struct TinkerHelper {
    completer: TinkerCompleter,
    highlighter: TinkerHighlighter,
}

impl Helper for TinkerHelper {}

impl Completer for TinkerHelper {
    type Candidate = rustyline::completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for TinkerHelper {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        None
    }
}

impl Highlighter for TinkerHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        self.highlighter.highlight_prompt(prompt, default)
    }
}

impl Validator for TinkerHelper {}

/// The main Tinker REPL
pub struct Tinker {
    config: TinkerConfig,
    db: Option<Arc<DatabaseConnection>>,
    executor: QueryExecutor,
    formatter: OutputFormatter,
    #[allow(dead_code)]
    command_handler: CommandHandler,
}

impl Tinker {
    /// Create a new Tinker instance
    pub fn new(config: TinkerConfig) -> Self {
        Self {
            config,
            db: None,
            executor: QueryExecutor::new(),
            formatter: OutputFormatter::new(),
            command_handler: CommandHandler::new(),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(TinkerConfig::default())
    }

    /// Set database connection
    pub fn with_database(mut self, db: DatabaseConnection) -> Self {
        self.db = Some(Arc::new(db));
        self
    }

    /// Connect to database from URL
    pub async fn connect(&mut self, url: &str) -> Result<(), sea_orm::DbErr> {
        let db = sea_orm::Database::connect(url).await?;
        self.db = Some(Arc::new(db));
        Ok(())
    }

    /// Run the REPL
    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.print_banner();

        // Try to connect to database if URL provided
        if let Some(ref url) = self.config.database_url.clone() {
            match self.connect(url).await {
                Ok(_) => println!("{}", "Connected to database.".green()),
                Err(e) => println!("{}: {}", "Database connection failed".yellow(), e),
            }
        }

        // Setup rustyline
        let config = Config::builder()
            .history_ignore_space(true)
            .max_history_size(self.config.max_history)?
            .build();

        let helper = TinkerHelper {
            completer: TinkerCompleter::new(),
            highlighter: TinkerHighlighter::new(),
        };

        let mut rl: Editor<TinkerHelper, DefaultHistory> = Editor::with_config(config)?;
        rl.set_helper(Some(helper));

        // Load history
        if let Some(ref history_file) = self.config.history_file {
            let _ = rl.load_history(history_file);
        }

        // Main REPL loop
        loop {
            let readline = rl.readline(&self.config.prompt.cyan().to_string());

            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // Add to history
                    let _ = rl.add_history_entry(line);

                    // Check for meta commands (starting with .)
                    if line.starts_with('.') {
                        match self.handle_meta_command(line).await {
                            Ok(should_exit) => {
                                if should_exit {
                                    break;
                                }
                            }
                            Err(e) => println!("{}: {}", "Error".red(), e),
                        }
                        continue;
                    }

                    // Execute the expression
                    match self.execute(line).await {
                        Ok(result) => {
                            self.formatter.print(&result);
                        }
                        Err(e) => {
                            println!("{}: {}", "Error".red(), e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Use .exit or Ctrl-D to exit");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("Goodbye!");
                    break;
                }
                Err(err) => {
                    println!("{}: {:?}", "Error".red(), err);
                    break;
                }
            }
        }

        // Save history
        if let Some(ref history_file) = self.config.history_file {
            let _ = rl.save_history(history_file);
        }

        Ok(())
    }

    /// Print welcome banner
    fn print_banner(&self) {
        println!();
        println!("{}", "╔═══════════════════════════════════════════╗".cyan());
        println!("{}", "║     RustForge Tinker - Interactive REPL   ║".cyan());
        println!("{}", "╚═══════════════════════════════════════════╝".cyan());
        println!();
        println!("Type {} for available commands", ".help".yellow());
        println!("Type {} or press {} to exit", ".exit".yellow(), "Ctrl-D".yellow());
        println!();
    }

    /// Handle meta commands (starting with .)
    async fn handle_meta_command(&mut self, command: &str) -> anyhow::Result<bool> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts.first().unwrap_or(&"");
        let args: Vec<&str> = parts.iter().skip(1).copied().collect();

        match *cmd {
            ".exit" | ".quit" | ".q" => {
                println!("Goodbye!");
                return Ok(true);
            }
            ".help" | ".h" | ".?" => {
                self.print_help();
            }
            ".tables" => {
                self.list_tables().await?;
            }
            ".schema" => {
                if let Some(table) = args.first() {
                    self.show_schema(table).await?;
                } else {
                    println!("{}: .schema <table_name>", "Usage".yellow());
                }
            }
            ".databases" => {
                self.list_databases().await?;
            }
            ".clear" => {
                print!("\x1B[2J\x1B[1;1H");
            }
            ".reconnect" => {
                if let Some(ref url) = self.config.database_url.clone() {
                    self.connect(&url).await?;
                    println!("{}", "Reconnected to database.".green());
                } else {
                    println!("{}", "No database URL configured.".yellow());
                }
            }
            ".env" => {
                self.show_env();
            }
            ".history" => {
                println!("{}", "History is saved to .tinker_history".dimmed());
            }
            _ => {
                println!("{}: Unknown command '{}'. Type .help for available commands.",
                    "Error".red(), cmd);
            }
        }

        Ok(false)
    }

    /// Print help message
    fn print_help(&self) {
        println!();
        println!("{}", "Available Commands:".yellow().bold());
        println!();
        println!("  {:<20} {}", ".help, .h, .?".green(), "Show this help message");
        println!("  {:<20} {}", ".exit, .quit, .q".green(), "Exit Tinker");
        println!("  {:<20} {}", ".clear".green(), "Clear the screen");
        println!("  {:<20} {}", ".tables".green(), "List all database tables");
        println!("  {:<20} {}", ".schema <table>".green(), "Show table schema");
        println!("  {:<20} {}", ".databases".green(), "List available databases");
        println!("  {:<20} {}", ".reconnect".green(), "Reconnect to database");
        println!("  {:<20} {}", ".env".green(), "Show environment info");
        println!("  {:<20} {}", ".history".green(), "Show history info");
        println!();
        println!("{}", "Query Examples:".yellow().bold());
        println!();
        println!("  {}", r#"DB::table("users").get()"#.cyan());
        println!("  {}", r#"DB::table("users").where("id", 1).first()"#.cyan());
        println!("  {}", r#"DB::table("users").count()"#.cyan());
        println!("  {}", r#"DB::select("SELECT * FROM users LIMIT 5")"#.cyan());
        println!();
        println!("{}", "SQL Examples:".yellow().bold());
        println!();
        println!("  {}", "SELECT * FROM users LIMIT 10".cyan());
        println!("  {}", "INSERT INTO users (name, email) VALUES ('John', 'john@example.com')".cyan());
        println!("  {}", "UPDATE users SET name = 'Jane' WHERE id = 1".cyan());
        println!();
    }

    /// Execute an expression or query
    async fn execute(&self, input: &str) -> anyhow::Result<crate::executor::ExecutionResult> {
        let ctx = ExecutionContext {
            db: self.db.clone(),
            input: input.to_string(),
        };

        self.executor.execute(ctx).await
    }

    /// List all database tables
    async fn list_tables(&self) -> anyhow::Result<()> {
        let Some(ref db) = self.db else {
            println!("{}", "No database connection. Use forge tinker --database=<url>".yellow());
            return Ok(());
        };

        let backend = db.get_database_backend();
        let query = match backend {
            sea_orm::DatabaseBackend::Postgres => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
            }
            sea_orm::DatabaseBackend::MySql => {
                "SHOW TABLES"
            }
            sea_orm::DatabaseBackend::Sqlite => {
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
            }
        };

        let results = db.query_all(sea_orm::Statement::from_string(backend, query)).await?;

        println!();
        println!("{}", "Tables:".yellow().bold());
        println!();

        for row in results {
            if let Ok(name) = row.try_get::<String>("", "table_name")
                .or_else(|_| row.try_get::<String>("", "name"))
                .or_else(|_| row.try_get::<String>("", "Tables_in_database"))
            {
                println!("  - {}", name.green());
            }
        }
        println!();

        Ok(())
    }

    /// Show table schema
    async fn show_schema(&self, table: &str) -> anyhow::Result<()> {
        let Some(ref db) = self.db else {
            println!("{}", "No database connection.".yellow());
            return Ok(());
        };

        let backend = db.get_database_backend();
        let query = match backend {
            sea_orm::DatabaseBackend::Postgres => {
                format!(
                    "SELECT column_name, data_type, is_nullable, column_default
                     FROM information_schema.columns
                     WHERE table_name = '{}'
                     ORDER BY ordinal_position",
                    table
                )
            }
            sea_orm::DatabaseBackend::MySql => {
                format!("DESCRIBE {}", table)
            }
            sea_orm::DatabaseBackend::Sqlite => {
                format!("PRAGMA table_info({})", table)
            }
        };

        let results = db.query_all(sea_orm::Statement::from_string(backend, query)).await?;

        println!();
        println!("{} {}", "Schema for".yellow().bold(), table.green().bold());
        println!();
        println!("  {:<20} {:<15} {:<10} {}",
            "Column".cyan(), "Type".cyan(), "Nullable".cyan(), "Default".cyan());
        println!("  {}", "-".repeat(60));

        for row in results {
            match backend {
                sea_orm::DatabaseBackend::Postgres => {
                    let name: String = row.try_get("", "column_name").unwrap_or_default();
                    let dtype: String = row.try_get("", "data_type").unwrap_or_default();
                    let nullable: String = row.try_get("", "is_nullable").unwrap_or_default();
                    let default: Option<String> = row.try_get("", "column_default").ok();
                    println!("  {:<20} {:<15} {:<10} {}",
                        name, dtype, nullable, default.unwrap_or("-".to_string()));
                }
                sea_orm::DatabaseBackend::Sqlite => {
                    let name: String = row.try_get("", "name").unwrap_or_default();
                    let dtype: String = row.try_get("", "type").unwrap_or_default();
                    let notnull: i32 = row.try_get("", "notnull").unwrap_or(0);
                    let default: Option<String> = row.try_get("", "dflt_value").ok();
                    println!("  {:<20} {:<15} {:<10} {}",
                        name, dtype, if notnull == 0 { "YES" } else { "NO" },
                        default.unwrap_or("-".to_string()));
                }
                _ => {}
            }
        }
        println!();

        Ok(())
    }

    /// List databases
    async fn list_databases(&self) -> anyhow::Result<()> {
        let Some(ref db) = self.db else {
            println!("{}", "No database connection.".yellow());
            return Ok(());
        };

        let backend = db.get_database_backend();
        let query = match backend {
            sea_orm::DatabaseBackend::Postgres => {
                "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname"
            }
            sea_orm::DatabaseBackend::MySql => {
                "SHOW DATABASES"
            }
            sea_orm::DatabaseBackend::Sqlite => {
                "PRAGMA database_list"
            }
        };

        let results = db.query_all(sea_orm::Statement::from_string(backend, query)).await?;

        println!();
        println!("{}", "Databases:".yellow().bold());
        println!();

        for row in results {
            if let Ok(name) = row.try_get::<String>("", "datname")
                .or_else(|_| row.try_get::<String>("", "name"))
                .or_else(|_| row.try_get::<String>("", "Database"))
            {
                println!("  - {}", name.green());
            }
        }
        println!();

        Ok(())
    }

    /// Show environment info
    fn show_env(&self) {
        println!();
        println!("{}", "Environment:".yellow().bold());
        println!();

        if let Ok(val) = std::env::var("APP_ENV") {
            println!("  {:<15} {}", "APP_ENV:".cyan(), val);
        }
        if let Ok(val) = std::env::var("APP_DEBUG") {
            println!("  {:<15} {}", "APP_DEBUG:".cyan(), val);
        }
        if let Ok(val) = std::env::var("DATABASE_URL") {
            // Mask password in URL
            let masked = val.split('@').last().unwrap_or(&val);
            println!("  {:<15} ***@{}", "DATABASE:".cyan(), masked);
        }
        println!();
    }
}
