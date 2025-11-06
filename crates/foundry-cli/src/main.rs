mod commands;

use clap::{Parser, ValueEnum};
use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use foundry_api::http::HttpServer;
use foundry_api::invocation::FoundryInvoker;
use foundry_api::mcp::McpServer;
use foundry_application::FoundryApp;
use foundry_infra::{
    AuditRecord, ConfigError, ConfigProvider, DotenvProvider, JsonlAuditLogger, LocalArtifactPort,
    SeaOrmMigrationService, SeaOrmSeedService,
};
use foundry_plugins::{CommandResult, CommandStatus, ExecutionOptions, ResponseFormat};
use foundry_interactive::{ask_with_default, choice, confirm, SelectOption};
use foundry_console::{success, error, info as console_info, warning};
use serde_json::Value;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug)]
#[command(
    name = "foundry",
    about = "Foundry Core CLI – Developer Experience Toolkit für Rust",
    version,
    author
)]
struct Cli {
    #[arg(long, value_enum, default_value_t = OutputFormat::Human, global = true)]
    format: OutputFormat,
    #[arg(short, long, help = "Aktiviere detailliertes Logging")]
    verbose: bool,
    #[arg(
        long,
        help = "Simuliert den Kommandoablauf ohne Schreiboperationen",
        global = true
    )]
    dry_run: bool,
    #[arg(
        long,
        help = "Erzwingt Überschreiben bestehender Artefakte",
        global = true
    )]
    force: bool,
    #[arg(value_name = "COMMAND")]
    command: Option<String>,
    #[arg(
        value_name = "ARGS",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    args: Vec<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

impl From<OutputFormat> for ResponseFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Human => ResponseFormat::Human,
            OutputFormat::Json => ResponseFormat::Json,
        }
    }
}

fn init_tracing(verbose: bool) -> Result<()> {
    let default_level = if verbose {
        "foundry=debug"
    } else {
        "foundry=info"
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
        .map_err(|err| color_eyre::eyre::eyre!(err))?;
    Ok(())
}

fn render_result(result: &CommandResult, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => {
            if let Some(message) = &result.message {
                println!("{message}");
            }
            if let Some(error) = &result.error {
                eprintln!("Fehler [{}]: {}", error.code, error.message);
                if !error.context().is_empty() {
                    for field in error.context() {
                        eprintln!("  {}: {}", field.key, field.value);
                    }
                }
            }
            Ok(())
        }
        OutputFormat::Json => {
            let payload = serde_json::to_string_pretty(result)?;
            println!("{payload}");
            Ok(())
        }
    }
}

fn load_config() -> Result<Value> {
    let provider = DotenvProvider::default();
    let path = provider.path.clone();
    match provider.load() {
        Ok(value) => Ok(value),
        Err(ConfigError::NotFound) => {
            warn!(".env not found at {}", path.display());
            Ok(Value::Object(Default::default()))
        }
        Err(err) => bail!("failed to load environment from {}: {err}", path.display()),
    }
}

fn resolve_audit_path(config: &Value) -> PathBuf {
    config
        .as_object()
        .and_then(|map| map.get("FOUNDRY_AUDIT_LOG"))
        .and_then(|value| value.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".foundry/audit.log"))
}

fn run_serve(app: FoundryApp, args: Vec<String>) -> Result<()> {
    match parse_serve_args(args)? {
        ServeMode::Help => {
            println!("Usage: foundry serve [--addr <ADDR>] [--mcp-stdio]");
            println!("  --addr <ADDR>     Bind-Adresse, z. B. 127.0.0.1:8080");
            println!("  --mcp-stdio       Aktiviert MCP STDIO Gateway");
            Ok(())
        }
        ServeMode::Run { addr, mcp_stdio } => {
            info!(%addr, "Starte Foundry HTTP Server");
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(async move {
                let invoker = FoundryInvoker::new(app);
                if mcp_stdio {
                    tokio::try_join!(
                        HttpServer::new(invoker.clone()).serve(addr),
                        McpServer::new(invoker).run_stdio()
                    )
                    .map_err(|err| eyre!(err))?;
                } else {
                    HttpServer::new(invoker)
                        .serve(addr)
                        .await
                        .map_err(|err| eyre!(err))?;
                }
                Ok::<(), color_eyre::eyre::Report>(())
            })?;
            Ok(())
        }
    }
}

enum ServeMode {
    Help,
    Run { addr: SocketAddr, mcp_stdio: bool },
}

fn parse_serve_args(args: Vec<String>) -> Result<ServeMode> {
    let mut addr: Option<SocketAddr> = None;
    let mut mcp_stdio = false;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--addr" => {
                let value = iter
                    .next()
                    .ok_or_else(|| eyre!("--addr benötigt einen Wert"))?;
                let parsed = value
                    .parse()
                    .wrap_err("Konnte --addr nicht als SocketAddr interpretieren")?;
                addr = Some(parsed);
            }
            "--mcp-stdio" => {
                mcp_stdio = true;
            }
            "--help" | "-h" => return Ok(ServeMode::Help),
            other if other.starts_with('-') => {
                return Err(eyre!(format!("Unbekannte Option `{other}`")));
            }
            other => {
                if addr.is_none() {
                    let parsed = other.parse().wrap_err("Konnte die Adresse nicht parsen")?;
                    addr = Some(parsed);
                } else {
                    return Err(eyre!(format!("Unerwartetes Argument `{other}`")));
                }
            }
        }
    }

    let addr = addr.unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080)));
    Ok(ServeMode::Run { addr, mcp_stdio })
}

fn run_init() -> Result<()> {
    console_info("Initializing new Foundry project...");

    if PathBuf::from(".env").exists() {
        let overwrite = confirm(".env file already exists. Overwrite?", false)
            .map_err(|e| eyre!("Failed to get confirmation: {}", e))?;
        if !overwrite {
            warning("Initialization cancelled.");
            return Ok(());
        }
    }

    let options = vec![
        SelectOption::new("SQLite", "Lightweight embedded database"),
        SelectOption::new("PostgreSQL", "Production-ready relational database"),
    ];

    let driver = choice("Which database driver would you like to use?", &options, 0)
        .map_err(|e| eyre!("Failed to get database choice: {}", e))?;

    let mut env_content = String::new();

    match driver.as_str() {
        "SQLite" => {
            let db_path = ask_with_default("Path to SQLite database", "foundry.db")
                .map_err(|e| eyre!("Failed to get database path: {}", e))?;
            env_content.push_str("DB_CONNECTION=sqlite\n");
            env_content.push_str(&format!("DATABASE_URL=sqlite:{}", db_path));
        }
        "PostgreSQL" => {
            let host = ask_with_default("Database host", "localhost")
                .map_err(|e| eyre!("Failed to get host: {}", e))?;
            let port = ask_with_default("Database port", "5432")
                .map_err(|e| eyre!("Failed to get port: {}", e))?;
            let username = ask_with_default("Username", "postgres")
                .map_err(|e| eyre!("Failed to get username: {}", e))?;
            let password = foundry_interactive::password("Password")
                .map_err(|e| eyre!("Failed to get password: {}", e))?;
            let db_name = ask_with_default("Database name", "foundry")
                .map_err(|e| eyre!("Failed to get database name: {}", e))?;

            env_content.push_str("DB_CONNECTION=postgres\n");
            env_content.push_str(&format!(
                "DATABASE_URL=postgres://{}:{}@{}:{}/{}",
                username, password, host, port, db_name
            ));
        }
        _ => unreachable!(),
    }

    fs::write(".env", env_content)?;
    success(".env file successfully created!");

    Ok(())
}

fn run_new(args: Vec<String>) -> Result<()> {
    // Parse arguments
    let mut name: Option<String> = None;
    let mut skip_wizard = false;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--skip-wizard" => {
                skip_wizard = true;
            }
            "--help" | "-h" => {
                println!("Usage: foundry new <name> [options]");
                println!();
                println!("Create a new RustForge application");
                println!();
                println!("Arguments:");
                println!("  <name>              Project name");
                println!();
                println!("Options:");
                println!("  --skip-wizard       Skip interactive wizard and use defaults");
                println!("  --help, -h          Show this help message");
                return Ok(());
            }
            other if !other.starts_with('-') => {
                if name.is_none() {
                    name = Some(other.to_string());
                } else {
                    bail!("Unexpected argument: {}", other);
                }
            }
            other => {
                bail!("Unknown option: {}", other);
            }
        }
    }

    let name = name.ok_or_else(|| eyre!("Project name is required. Usage: foundry new <name>"))?;

    // Validate project name
    if name.is_empty() {
        bail!("Project name cannot be empty");
    }

    // Execute the new command
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(commands::new::execute(name, skip_wizard))?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let Cli {
        format: output_format,
        verbose,
        dry_run,
        force,
        command,
        args,
    } = Cli::parse();
    init_tracing(verbose)?;

    if command.as_deref() == Some("init") {
        return run_init();
    }

    if command.as_deref() == Some("new") {
        return run_new(args);
    }

    if command.as_deref() == Some("test") {
        info!("Führe Cargo-Tests aus...");
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("test");
        cmd.args(&args); // Pass remaining args to cargo test

        let status = cmd.status().wrap_err("Fehler beim Ausführen von 'cargo test'")?;

        if !status.success() {
            bail!("Tests fehlgeschlagen mit Status: {}", status);
        }

        info!("Alle Tests erfolgreich bestanden.");
        return Ok(());
    }

    let command = command.unwrap_or_else(|| "list".to_string());
    let command_args = args;
    let response_format: ResponseFormat = output_format.into();
    let audit_format = response_format.clone();
    let options = ExecutionOptions { dry_run, force };

    let config = load_config()?;
    let audit_path = resolve_audit_path(&config);
    let artifacts = Arc::new(LocalArtifactPort::default());
    let migrations = Arc::new(SeaOrmMigrationService::default());
    let seeds = Arc::new(SeaOrmSeedService::default());
    let app = FoundryApp::bootstrap(config, artifacts, migrations, seeds)?;
    let audit_logger = JsonlAuditLogger::new(audit_path);

    if command == "serve" {
        let args_snapshot = command_args.clone();
        let outcome = run_serve(app, command_args);
        let record = match &outcome {
            Ok(_) => {
                let synthetic = CommandResult::success("serve command completed");
                AuditRecord::from_success(
                    command.clone(),
                    args_snapshot,
                    audit_format.clone(),
                    options,
                    &synthetic,
                )
            }
            Err(err) => AuditRecord::from_error(
                command.clone(),
                args_snapshot,
                audit_format.clone(),
                options,
                err.to_string(),
            ),
        };
        if let Err(err) = audit_logger.log(&record) {
            warn!(error = %err, "Konnte Audit-Log nicht schreiben");
        }
        return outcome;
    }

    let args_snapshot = command_args.clone();
    let runtime = tokio::runtime::Runtime::new()?;
    let outcome = runtime.block_on(app.dispatch(&command, command_args, response_format, options));

    match outcome {
        Ok(result) => {
            let record = AuditRecord::from_success(
                command.clone(),
                args_snapshot.clone(),
                audit_format.clone(),
                options,
                &result,
            );
            if let Err(err) = audit_logger.log(&record) {
                warn!(error = %err, "Konnte Audit-Log nicht schreiben");
            }

            if result.status == CommandStatus::Failure {
                if let Some(message) = &result.message {
                    bail!(message.clone());
                } else {
                    bail!("command failed without message");
                }
            }

            render_result(&result, output_format)?;
            Ok(())
        }
        Err(err) => {
            let record = AuditRecord::from_error(
                command.clone(),
                args_snapshot,
                audit_format,
                options,
                err.to_string(),
            );
            if let Err(log_err) = audit_logger.log(&record) {
                warn!(error = %log_err, "Konnte Audit-Log nicht schreiben");
            }
            Err(eyre!(err))
        }
    }
}
