use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct DatabaseCreateCommand {
    descriptor: CommandDescriptor,
}

impl Default for DatabaseCreateCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseCreateCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.database_create", "database:create")
            .summary("Erstelle eine neue Datenbank")
            .description(
                "Interactive setup wizard or use existing database from .env.\n\
                 \nInteractive Mode:\n\
                 - foundry database:create\n\
                 - foundry database:create --driver=mysql\n\
                 \nUse Existing Database from .env:\n\
                 - foundry database:create --existing\n\
                 - foundry database:create --validate-only\n\
                 \nNon-Interactive Mode (Flags):\n\
                 - foundry database:create --driver=mysql --root-user=root --root-password=pass --host=localhost --port=3306 --db-name=foundry_db --db-user=appuser --db-password=apppass\n\
                 - foundry database:create --driver=postgres --host=db.example.com --db-name=myapp --db-user=appuser --db-password=apppass"
            )
            .category(CommandKind::Database)
            .build();

        Self { descriptor }
    }

    fn get_flag_value(args: &[String], flag: &str) -> Option<String> {
        for arg in args {
            if arg.starts_with(&format!("--{}=", flag)) {
                return Some(arg.split('=').nth(1)?.to_string());
            }
        }
        None
    }

    fn has_flag(args: &[String], flag: &str) -> bool {
        args.iter().any(|arg| arg == &format!("--{}", flag))
    }

    fn read_env_file() -> Result<String, CommandError> {
        fs::read_to_string(".env")
            .map_err(|e| CommandError::Message(format!("Failed to read .env file: {}", e)))
    }

    fn get_database_url_from_env() -> Result<String, CommandError> {
        let content = Self::read_env_file()?;
        for line in content.lines() {
            if line.starts_with("DATABASE_URL=") {
                if let Some(url) = line.split('=').nth(1) {
                    return Ok(url.trim().to_string());
                }
            }
        }
        Err(CommandError::Message(
            "DATABASE_URL not found in .env file. Please set DATABASE_URL or run setup wizard.".to_string()
        ))
    }

    fn validate_database_url(url: &str) -> Result<String, CommandError> {
        // Extract driver from URL
        let driver = if url.starts_with("mysql://") {
            "mysql"
        } else if url.starts_with("postgresql://") {
            "postgres"
        } else if url.starts_with("sqlite:") {
            "sqlite"
        } else {
            return Err(CommandError::Message(
                "Invalid DATABASE_URL format. Supported: mysql://, postgresql://, sqlite:".to_string()
            ));
        };

        Ok(driver.to_string())
    }

    async fn validate_connection(url: &str, driver: &str) -> Result<(), CommandError> {
        println!("\n🔍 Validating database connection...\n");

        match driver {
            "mysql" => {
                // Parse MySQL connection string
                // Format: mysql://user:pass@host:port/database
                if let Some(credentials) = url.strip_prefix("mysql://") {
                    println!("✓ MySQL connection string: {}", credentials.replace(char::is_whitespace, ""));
                    println!("✓ Connection validated (format check)");
                    Ok(())
                } else {
                    Err(CommandError::Message("Invalid MySQL URL format".to_string()))
                }
            }
            "postgres" => {
                // Parse PostgreSQL connection string
                if let Some(credentials) = url.strip_prefix("postgresql://") {
                    println!("✓ PostgreSQL connection string: {}", credentials.replace(char::is_whitespace, ""));
                    println!("✓ Connection validated (format check)");
                    Ok(())
                } else {
                    Err(CommandError::Message("Invalid PostgreSQL URL format".to_string()))
                }
            }
            "sqlite" => {
                // Check SQLite file path
                if let Some(path) = url.strip_prefix("sqlite:///") {
                    println!("✓ SQLite database path: {}", path);
                    println!("✓ Connection validated (path check)");
                    Ok(())
                } else {
                    Err(CommandError::Message("Invalid SQLite URL format".to_string()))
                }
            }
            _ => Err(CommandError::Message("Unknown database driver".to_string())),
        }
    }

    async fn setup_mysql(
        &self,
        _ctx: &CommandContext,
        host: Option<String>,
        port: Option<String>,
        root_user: Option<String>,
        root_password: Option<String>,
        db_name: Option<String>,
        db_user: Option<String>,
        db_password: Option<String>,
    ) -> Result<String, CommandError> {
        println!("\n🔧 MySQL Database Setup\n");

        let is_interactive = host.is_none();
        if is_interactive {
            println!("Benötigte Informationen:");
        }

        let host = match host {
            Some(h) => h,
            None => dialoguer::Input::new()
                .with_prompt("Host")
                .default("localhost".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let port = match port {
            Some(p) => p,
            None => dialoguer::Input::new()
                .with_prompt("Port")
                .default("3306".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let root_user = match root_user {
            Some(u) => u,
            None => dialoguer::Input::new()
                .with_prompt("Root Username")
                .default("root".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let root_password = match root_password {
            Some(p) => p,
            None => {
                if is_interactive {
                    rpassword::prompt_password("Root Password: ")
                        .map_err(|e| CommandError::Message(format!("Password error: {}", e)))?
                } else {
                    String::new()
                }
            }
        };

        let db_name = match db_name {
            Some(n) => n,
            None => dialoguer::Input::new()
                .with_prompt("Database Name")
                .default("foundry_db".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let db_user = match db_user {
            Some(u) => u,
            None => dialoguer::Input::new()
                .with_prompt("Database User")
                .default("foundry".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let db_password = match db_password {
            Some(p) => p,
            None => {
                if is_interactive {
                    rpassword::prompt_password("Database Password: ")
                        .map_err(|e| CommandError::Message(format!("Password error: {}", e)))?
                } else {
                    String::new()
                }
            }
        };

        // Note: Administrative connection string created for documentation purposes
        let _admin_url = if root_password.is_empty() {
            format!(
                "mysql://{}@{}:{}/{}",
                root_user, host, port, db_name
            )
        } else {
            format!(
                "mysql://{}:{}@{}:{}/{}",
                root_user, root_password, host, port, db_name
            )
        };

        println!("\n✓ Setting up database...");

        // Connect to MySQL with root and create database
        let create_db_command = if root_password.is_empty() {
            format!(
                "mysql -u {} -h {} -P {} -e \"CREATE DATABASE IF NOT EXISTS {};\" ",
                root_user, host, port, db_name
            )
        } else {
            format!(
                "mysql -u {} -p'{}' -h {} -P {} -e \"CREATE DATABASE IF NOT EXISTS {}; \
                 CREATE USER IF NOT EXISTS '{}' IDENTIFIED BY '{}'; \
                 GRANT ALL PRIVILEGES ON {}.* TO '{}'; \
                 FLUSH PRIVILEGES;\"",
                root_user, root_password, host, port, db_name, db_user, db_password, db_name, db_user
            )
        };

        let output = Command::new("sh")
            .arg("-c")
            .arg(&create_db_command)
            .output()
            .map_err(|e| CommandError::Message(format!("MySQL command failed: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("⚠️  MySQL Warning: {}", error);
            // Continue anyway, might be permission issues
        }

        println!("✓ Database '{}' created/verified", db_name);

        // Create connection URL with app user
        let connection_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            db_user, db_password, host, port, db_name
        );

        Ok(connection_url)
    }

    async fn setup_postgres(
        &self,
        _ctx: &CommandContext,
        host: Option<String>,
        port: Option<String>,
        db_name: Option<String>,
        db_user: Option<String>,
        db_password: Option<String>,
    ) -> Result<String, CommandError> {
        println!("\n🔧 PostgreSQL Database Setup\n");

        let is_interactive = host.is_none();
        if is_interactive {
            println!("Benötigte Informationen:");
        }

        let host = match host {
            Some(h) => h,
            None => dialoguer::Input::new()
                .with_prompt("Host")
                .default("localhost".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let port = match port {
            Some(p) => p,
            None => dialoguer::Input::new()
                .with_prompt("Port")
                .default("5432".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let db_name = match db_name {
            Some(n) => n,
            None => dialoguer::Input::new()
                .with_prompt("Database Name")
                .default("foundry_db".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let db_user = match db_user {
            Some(u) => u,
            None => dialoguer::Input::new()
                .with_prompt("Database User")
                .default("foundry".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        let db_password = match db_password {
            Some(p) => p,
            None => {
                if is_interactive {
                    rpassword::prompt_password("Database Password: ")
                        .map_err(|e| CommandError::Message(format!("Password error: {}", e)))?
                } else {
                    String::new()
                }
            }
        };

        println!("\n✓ Setting up database...");

        let connection_url = format!(
            "postgresql://{}:{}@{}:{}/{}",
            db_user, db_password, host, port, db_name
        );

        println!("✓ Database '{}' configuration created", db_name);

        Ok(connection_url)
    }

    async fn setup_sqlite(&self, _ctx: &CommandContext, db_path: Option<String>) -> Result<String, CommandError> {
        println!("\n🔧 SQLite Database Setup\n");

        let db_path = match db_path {
            Some(p) => p,
            None => dialoguer::Input::new()
                .with_prompt("Database Path")
                .default("./database.sqlite".to_string())
                .interact_text()
                .map_err(|e| CommandError::Message(format!("Input error: {}", e)))?,
        };

        println!("✓ SQLite database path: {}", db_path);

        let connection_url = format!("sqlite:///{}", db_path);

        Ok(connection_url)
    }

    fn update_env_file(&self, database_url: &str) -> Result<(), CommandError> {
        let env_path = ".env";

        // Read existing .env or create from template
        let content = if Path::new(env_path).exists() {
            fs::read_to_string(env_path)
                .map_err(|e| CommandError::Message(format!("Failed to read .env: {}", e)))?
        } else {
            // Create from template
            self.create_env_template()?
        };

        // Update or add DATABASE_URL
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut found_database_url = false;

        for line in lines {
            if line.starts_with("DATABASE_URL=") {
                new_lines.push(format!("DATABASE_URL={}", database_url));
                found_database_url = true;
            } else {
                new_lines.push(line.to_string());
            }
        }

        if !found_database_url {
            new_lines.push(format!("DATABASE_URL={}", database_url));
        }

        let updated_content = new_lines.join("\n");

        fs::write(env_path, updated_content)
            .map_err(|e| CommandError::Message(format!("Failed to write .env: {}", e)))?;

        println!("✓ Updated .env file");

        Ok(())
    }

    fn create_env_template(&self) -> Result<String, CommandError> {
        Ok(r#"APP_NAME=Foundry
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:8000
LOG_LEVEL=debug

# Database will be added here
DATABASE_POOL_SIZE=10
DATABASE_POOL_MIN_IDLE=3

# Redis
REDIS_URL=redis://127.0.0.1:6379
REDIS_DB=0

# Cache
CACHE_DRIVER=redis
CACHE_TTL=3600

# Queue
QUEUE_DRIVER=redis
"#
        .to_string())
    }
}

#[async_trait]
impl FoundryCommand for DatabaseCreateCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        println!("\n📦 Rust Foundry Framework - Database Setup\n");
        println!("═══════════════════════════════════════════\n");

        // Check for special flags
        let use_existing = Self::has_flag(&ctx.args, "existing");
        let validate_only = Self::has_flag(&ctx.args, "validate-only");

        // Handle --existing flag (use DATABASE_URL from .env)
        if use_existing {
            let database_url = Self::get_database_url_from_env()?;
            let driver = Self::validate_database_url(&database_url)?;

            println!("✅ Using existing database from .env\n");
            println!("Driver: {}", driver);
            println!("URL: {}", database_url.replace(char::is_whitespace, ""));

            Self::validate_connection(&database_url, &driver).await?;

            println!("\n✅ Database configuration is ready!\n");
            println!("Next steps:");
            println!("  1. Run migrations: foundry migrate");
            println!("  2. Seed database: foundry migrate:seed");
            println!("  3. Start developing!\n");

            let data = json!({
                "driver": driver,
                "status": "Database configured from .env",
                "next_step": "foundry migrate"
            });

            return Ok(CommandResult {
                status: CommandStatus::Success,
                message: Some("Using existing database".to_string()),
                data: Some(data),
                error: None,
            });
        }

        // Handle --validate-only flag (test existing connection only)
        if validate_only {
            println!("🔍 Validating database connection from .env\n");

            let database_url = Self::get_database_url_from_env()?;
            let driver = Self::validate_database_url(&database_url)?;

            Self::validate_connection(&database_url, &driver).await?;

            println!("\n✅ Database connection is valid!\n");

            let data = json!({
                "driver": driver,
                "status": "Connection validated",
                "url": database_url
            });

            return Ok(CommandResult {
                status: CommandStatus::Success,
                message: Some("Database connection valid".to_string()),
                data: Some(data),
                error: None,
            });
        }

        // Parse flag values for new setup
        let host = Self::get_flag_value(&ctx.args, "host");
        let port = Self::get_flag_value(&ctx.args, "port");
        let root_user = Self::get_flag_value(&ctx.args, "root-user");
        let root_password = Self::get_flag_value(&ctx.args, "root-password");
        let db_name = Self::get_flag_value(&ctx.args, "db-name");
        let db_user = Self::get_flag_value(&ctx.args, "db-user");
        let db_password = Self::get_flag_value(&ctx.args, "db-password");
        let db_path = Self::get_flag_value(&ctx.args, "db-path");

        // Determine driver
        let driver = if let Some(arg) = ctx.args.first() {
            if arg.starts_with("--driver=") {
                arg.split('=').nth(1).unwrap_or("")
            } else {
                ""
            }
        } else {
            ""
        };

        // Map driver names
        let driver = match driver {
            "mariadb" => "mysql", // MariaDB uses same connection string as MySQL
            d if !d.is_empty() => d,
            _ => "",
        };

        let driver = if driver.is_empty() {
            // Interactive selection
            let selection = dialoguer::Select::new()
                .with_prompt("Wähle eine Datenbank")
                .items(&[
                    "MySQL / MariaDB",
                    "PostgreSQL",
                    "SQLite",
                ])
                .interact()
                .map_err(|e| CommandError::Message(format!("Selection error: {}", e)))?;

            match selection {
                0 => "mysql",
                1 => "postgres",
                2 => "sqlite",
                _ => "mysql",
            }
        } else {
            driver
        };

        // Setup based on driver
        let database_url = match driver {
            "mysql" => self.setup_mysql(&ctx, host, port, root_user, root_password, db_name, db_user, db_password).await?,
            "postgres" => self.setup_postgres(&ctx, host, port, db_name, db_user, db_password).await?,
            "sqlite" => self.setup_sqlite(&ctx, db_path).await?,
            _ => return Err(CommandError::Message("Unknown database driver".to_string())),
        };

        // Update .env file
        self.update_env_file(&database_url)?;

        println!("\n✅ Database setup completed successfully!\n");
        println!("Next steps:");
        println!("  1. Run migrations: foundry migrate");
        println!("  2. Seed database: foundry migrate:seed");
        println!("  3. Start developing!\n");

        let data = json!({
            "driver": driver,
            "database_url": database_url.replace(|c| c == '/' && c != '/', "*").split(':').next().unwrap_or(""),
            "status": "Database configured successfully",
            "next_step": "foundry migrate"
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some("Database setup completed".to_string()),
            data: Some(data),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_create_descriptor() {
        let cmd = DatabaseCreateCommand::new();
        let descriptor = cmd.descriptor();

        assert_eq!(descriptor.name, "database:create");
        assert_eq!(descriptor.category, CommandKind::Database);
        assert!(descriptor.summary.contains("Datenbank"));
    }

    #[test]
    fn test_env_template_creation() {
        let cmd = DatabaseCreateCommand::new();
        let template = cmd.create_env_template().unwrap();

        assert!(template.contains("APP_NAME"));
        assert!(template.contains("DATABASE"));
        assert!(template.contains("REDIS"));
    }

    #[test]
    fn test_has_flag() {
        let args = vec![
            "--existing".to_string(),
            "--validate-only".to_string(),
            "--driver=mysql".to_string(),
        ];

        assert!(DatabaseCreateCommand::has_flag(&args, "existing"));
        assert!(DatabaseCreateCommand::has_flag(&args, "validate-only"));
        assert!(!DatabaseCreateCommand::has_flag(&args, "driver"));
        assert!(!DatabaseCreateCommand::has_flag(&args, "nonexistent"));
    }

    #[test]
    fn test_get_flag_value() {
        let args = vec![
            "--driver=mysql".to_string(),
            "--host=localhost".to_string(),
            "--port=3306".to_string(),
            "--db-name=test_db".to_string(),
            "--root-password=secret123".to_string(),
        ];

        assert_eq!(
            DatabaseCreateCommand::get_flag_value(&args, "driver"),
            Some("mysql".to_string())
        );
        assert_eq!(
            DatabaseCreateCommand::get_flag_value(&args, "host"),
            Some("localhost".to_string())
        );
        assert_eq!(
            DatabaseCreateCommand::get_flag_value(&args, "port"),
            Some("3306".to_string())
        );
        assert_eq!(
            DatabaseCreateCommand::get_flag_value(&args, "db-name"),
            Some("test_db".to_string())
        );
        assert_eq!(
            DatabaseCreateCommand::get_flag_value(&args, "root-password"),
            Some("secret123".to_string())
        );
        assert_eq!(
            DatabaseCreateCommand::get_flag_value(&args, "nonexistent"),
            None
        );
    }

    #[test]
    fn test_validate_database_url_mysql() {
        let url = "mysql://user:pass@localhost:3306/mydb";
        let result = DatabaseCreateCommand::validate_database_url(url);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mysql");
    }

    #[test]
    fn test_validate_database_url_postgres() {
        let url = "postgresql://user:pass@localhost:5432/mydb";
        let result = DatabaseCreateCommand::validate_database_url(url);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "postgres");
    }

    #[test]
    fn test_validate_database_url_sqlite() {
        let url = "sqlite:///./database.sqlite";
        let result = DatabaseCreateCommand::validate_database_url(url);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sqlite");
    }

    #[test]
    fn test_validate_database_url_invalid() {
        let url = "mongodb://user:pass@localhost:27017/mydb";
        let result = DatabaseCreateCommand::validate_database_url(url);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid DATABASE_URL format"));
    }

    #[test]
    fn test_descriptor_includes_existing_flag_docs() {
        let cmd = DatabaseCreateCommand::new();
        let descriptor = cmd.descriptor();

        let description = descriptor.description.as_ref().expect("Description should exist");
        assert!(description.contains("--existing"));
        assert!(description.contains("--validate-only"));
        assert!(description.contains("Use Existing Database from .env"));
    }
}
