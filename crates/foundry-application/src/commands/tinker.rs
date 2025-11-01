use async_trait::async_trait;
use rustyline::DefaultEditor;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::{json, Value};
use std::collections::HashMap;

use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{
    CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand, ResponseFormat,
};

/// Query-based Tinker REPL
#[derive(Debug)]
pub struct TinkerCommand {
    descriptor: CommandDescriptor,
}

impl TinkerCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("dev.tinker", "tinker")
                .summary("Interaktive REPL-Konsole für Datenbank-Abfragen")
                .description(
                    "Öffnet eine interaktive REPL-Konsole mit Query-basierten Befehlen:\n\
                     - find <Model> <id>              # Find by ID\n\
                     - list <Model>                   # List first 10 records\n\
                     - list <Model> --limit 50        # With limit\n\
                     - count <Model>                  # Count total records\n\
                     - create <Model> {\"key\":\"val\"}  # Create new record\n\
                     - update <Model> <id> {...}     # Update record\n\
                     - delete <Model> <id>           # Delete record\n\
                     - sql <query>                    # Execute raw SQL\n\
                     - help                           # Show this help\n\
                     - exit                           # Exit tinker"
                )
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

impl Default for TinkerCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FoundryCommand for TinkerCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        match ctx.format {
            ResponseFormat::Human => {
                // Human mode: Start interactive REPL
                self.run_repl(ctx).await
            }
            ResponseFormat::Json => {
                // JSON mode: Not supported for REPL
                Ok(CommandResult {
                    status: CommandStatus::Success,
                    message: Some("tinker command works best in human (interactive) mode".to_string()),
                    data: Some(json!({
                        "info": "Use 'foundry tinker' without --json flag for interactive mode"
                    })),
                    error: None,
                })
            }
        }
    }
}

impl TinkerCommand {
    async fn run_repl(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Get DATABASE_URL from config
        let db_url = ctx
            .config
            .as_object()
            .and_then(|map| map.get("DATABASE_URL"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| CommandError::Message("DATABASE_URL not found in configuration".into()))?
            .to_string();

        // Connect to database
        let db = Database::connect(&db_url)
            .await
            .map_err(|e| CommandError::Message(format!("Failed to connect to database: {}", e)))?;

        // Get database type from config
        let db_type = ctx
            .config
            .as_object()
            .and_then(|map| map.get("DB_CONNECTION"))
            .and_then(|value| value.as_str())
            .unwrap_or("sqlite")
            .to_string();

        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║         RustForge Tinker - Interactive REPL Console             ║");
        println!("║                  Type 'help' for available commands              ║");
        println!("╚════════════════════════════════════════════════════════════════╝\n");

        let mut rl = DefaultEditor::new().map_err(|e| {
            CommandError::Message(format!("Failed to initialize REPL: {}", e))
        })?;

        let mut session = TinkerSession {
            db,
            db_type,
            table_cache: HashMap::new(),
        };

        loop {
            let readline = rl.readline("tinker> ");

            match readline {
                Ok(line) => {
                    if line.is_empty() {
                        continue;
                    }

                    let _ = rl.add_history_entry(&line);

                    match self.process_command(&line, &mut session).await {
                        Ok(output) => {
                            println!("{}", output);
                        }
                        Err(e) => {
                            println!("❌ Error: {}", e);
                        }
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("\n👋 Goodbye!");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("\n👋 Goodbye!");
                    break;
                }
                Err(e) => {
                    eprintln!("❌ Readline error: {}", e);
                    break;
                }
            }
        }

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some("Tinker session ended".to_string()),
            data: None,
            error: None,
        })
    }

    async fn process_command(&self, input: &str, session: &mut TinkerSession) -> Result<String, String> {
        let input = input.trim();

        // Check for exit commands
        if input == "exit" || input == "quit" || input == ":q" {
            return Err("Exit tinker with Ctrl+C or Ctrl+D".to_string());
        }

        // Check for help
        if input == "help" || input == "?" {
            return Ok(self.get_help());
        }

        // Parse and execute query
        let query = Query::parse(input)?;
        self.execute_query(query, session).await
    }

    async fn execute_query(&self, query: Query, session: &mut TinkerSession) -> Result<String, String> {
        match query {
            Query::Find { model, id } => {
                session.find_by_id(&model, &id).await
            }
            Query::List { model, limit } => {
                session.list_records(&model, limit).await
            }
            Query::Count { model } => {
                session.count_records(&model).await
            }
            Query::Create { model, data } => {
                session.create_record(&model, data).await
            }
            Query::Update { model, id, data } => {
                session.update_record(&model, &id, data).await
            }
            Query::Delete { model, id } => {
                session.delete_record(&model, &id).await
            }
            Query::Sql { query: sql } => {
                session.execute_raw_sql(&sql).await
            }
        }
    }

    fn get_help(&self) -> String {
        r#"
╔════════════════════════════════════════════════════════════════╗
║                   Tinker Commands Reference                    ║
╚════════════════════════════════════════════════════════════════╝

📚 QUERY COMMANDS:
  find <Model> <id>              Find a record by ID
  list <Model>                   List first 10 records
  list <Model> --limit <N>       List with custom limit
  count <Model>                  Count total records
  all <Model>                    Get all records (no limit)

✍️  MODIFICATION COMMANDS:
  create <Model> {"key":"val"}   Create new record with data
  update <Model> <id> {...}      Update record by ID
  delete <Model> <id>            Delete record by ID

🔧 ADVANCED:
  sql <query>                    Execute raw SQL query

🆘 SYSTEM:
  help, ?                        Show this help
  exit, quit, :q                 Exit tinker

📝 EXAMPLES:
  tinker> find User 1
  tinker> list Post --limit 20
  tinker> create User {"name": "John", "email": "john@example.com"}
  tinker> update User 1 {"name": "Jane"}
  tinker> delete User 1
  tinker> sql SELECT * FROM users WHERE created_at > '2024-01-01';
"#
            .to_string()
    }
}

/// TinkerSession manages database connection and execution
struct TinkerSession {
    db: DatabaseConnection,
    db_type: String,
    table_cache: HashMap<String, Vec<String>>,
}

impl TinkerSession {
    async fn find_by_id(&mut self, table: &str, id: &str) -> Result<String, String> {
        // Validate table exists
        self.validate_table(table).await?;

        let backend = self.db.get_database_backend();
        let stmt = match self.db_type.as_str() {
            "postgres" => {
                sea_orm::Statement::from_string(
                    backend,
                    format!("SELECT * FROM \"{}\" WHERE id = '{}' LIMIT 1;", table, escape_sql(id)),
                )
            }
            "sqlite" => {
                sea_orm::Statement::from_string(
                    backend,
                    format!("SELECT * FROM \"{}\" WHERE id = '{}' LIMIT 1;", table, escape_sql(id)),
                )
            }
            _ => return Err(format!("Unsupported database type: {}", self.db_type)),
        };

        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| format!("Query failed: {}", e))?;

        if rows.is_empty() {
            return Ok(format!("❌ No record found in '{}' with id: {}", table, id));
        }

        self.format_rows(&rows)
    }

    async fn list_records(&mut self, table: &str, limit: Option<usize>) -> Result<String, String> {
        // Validate table exists
        self.validate_table(table).await?;

        let limit_val = limit.unwrap_or(10);
        let backend = self.db.get_database_backend();
        let stmt = sea_orm::Statement::from_string(
            backend,
            format!("SELECT * FROM \"{}\" LIMIT {};", table, limit_val),
        );

        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| format!("Query failed: {}", e))?;

        if rows.is_empty() {
            return Ok(format!("📭 No records found in '{}'", table));
        }

        let mut output = format!("📋 {} records from '{}' (showing {})\n", rows.len(), table, limit_val);
        output.push_str(&self.format_rows(&rows)?);
        Ok(output)
    }

    async fn count_records(&mut self, table: &str) -> Result<String, String> {
        // Validate table exists
        self.validate_table(table).await?;

        let backend = self.db.get_database_backend();
        let stmt = sea_orm::Statement::from_string(
            backend,
            format!("SELECT COUNT(*) as count FROM \"{}\";", table),
        );

        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| format!("Query failed: {}", e))?;

        if let Some(row) = rows.first() {
            let count: i64 = row.try_get("", "count").unwrap_or(0);
            return Ok(format!("📊 Total records in '{}': {}", table, count));
        }

        Ok(format!("⚠️  Could not count records in '{}'", table))
    }

    async fn create_record(&mut self, table: &str, data: Value) -> Result<String, String> {
        // Validate table exists
        self.validate_table(table).await?;

        let obj = data.as_object().ok_or("Data must be a JSON object")?;

        let columns: Vec<String> = obj.keys().map(|k| k.clone()).collect();
        let values: Vec<String> = obj
            .values()
            .map(|v| match v {
                Value::Null => "NULL".to_string(),
                Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => format!("'{}'", escape_sql(s)),
                _ => format!("'{}'", escape_sql(&v.to_string())),
            })
            .collect();

        let backend = self.db.get_database_backend();
        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({});",
            table,
            columns.join(", "),
            values.join(", ")
        );

        let stmt = sea_orm::Statement::from_string(backend, sql);

        self.db
            .execute(stmt)
            .await
            .map_err(|e| format!("Insert failed: {}", e))?;

        Ok(format!(
            "✨ Successfully created record in '{}' with {} columns",
            table,
            columns.len()
        ))
    }

    async fn update_record(&mut self, table: &str, id: &str, data: Value) -> Result<String, String> {
        // Validate table exists
        self.validate_table(table).await?;

        let obj = data.as_object().ok_or("Data must be a JSON object")?;

        let updates: Vec<String> = obj
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    Value::Null => "NULL".to_string(),
                    Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => format!("'{}'", escape_sql(s)),
                    _ => format!("'{}'", escape_sql(&v.to_string())),
                };
                format!("\"{}\" = {}", k, val)
            })
            .collect();

        let backend = self.db.get_database_backend();
        let sql = format!(
            "UPDATE \"{}\" SET {} WHERE id = '{}';",
            table,
            updates.join(", "),
            escape_sql(id)
        );

        let stmt = sea_orm::Statement::from_string(backend, sql);

        self.db
            .execute(stmt)
            .await
            .map_err(|e| format!("Update failed: {}", e))?;

        Ok(format!(
            "🔄 Successfully updated record {} in '{}' with {} columns",
            id,
            table,
            updates.len()
        ))
    }

    async fn delete_record(&mut self, table: &str, id: &str) -> Result<String, String> {
        // Validate table exists
        self.validate_table(table).await?;

        let backend = self.db.get_database_backend();
        let sql = format!("DELETE FROM \"{}\" WHERE id = '{}';", table, escape_sql(id));

        let stmt = sea_orm::Statement::from_string(backend, sql);

        self.db
            .execute(stmt)
            .await
            .map_err(|e| format!("Delete failed: {}", e))?;

        Ok(format!("🗑️  Successfully deleted record {} from '{}'", id, table))
    }

    async fn execute_raw_sql(&mut self, sql: &str) -> Result<String, String> {
        let backend = self.db.get_database_backend();
        let stmt = sea_orm::Statement::from_string(backend, sql.to_string());

        // Try as SELECT first
        if sql.trim().to_uppercase().starts_with("SELECT") {
            let rows = self
                .db
                .query_all(stmt)
                .await
                .map_err(|e| format!("Query failed: {}", e))?;

            if rows.is_empty() {
                return Ok("🔍 Query returned no results".to_string());
            }

            return self.format_rows(&rows);
        }

        // Otherwise execute as modification
        self.db
            .execute(stmt)
            .await
            .map_err(|e| format!("Execution failed: {}", e))?;

        Ok("✅ Raw SQL executed successfully".to_string())
    }

    async fn validate_table(&mut self, table: &str) -> Result<(), String> {
        let backend = self.db.get_database_backend();
        let query_sql = match self.db_type.as_str() {
            "postgres" => {
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename = $1;"
                    .to_string()
            }
            "sqlite" => {
                format!(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name = '{}';",
                    escape_sql(table)
                )
            }
            _ => return Err(format!("Unsupported database type: {}", self.db_type)),
        };

        let stmt = sea_orm::Statement::from_string(backend, query_sql);
        let rows = self
            .db
            .query_all(stmt)
            .await
            .map_err(|e| format!("Failed to check table: {}", e))?;

        if rows.is_empty() {
            return Err(format!("❌ Table '{}' not found", table));
        }

        Ok(())
    }

    fn format_rows(&self, rows: &[sea_orm::QueryResult]) -> Result<String, String> {
        if rows.is_empty() {
            return Ok("No rows to display".to_string());
        }

        let mut output = String::new();
        for (idx, row) in rows.iter().enumerate() {
            output.push_str(&format!("\n[Record {}]\n", idx + 1));
            output.push_str(&format!("{:-^50}\n", ""));

            // Try to extract common fields from the row
            // Note: SeaORM QueryResult has limited introspection, so we'll try common fields
            let common_fields = vec!["id", "name", "title", "email", "created_at", "updated_at"];

            for field in common_fields {
                if let Ok(value) = row.try_get::<String>("", field) {
                    output.push_str(&format!("  {:20} : {}\n", field, value));
                }
            }
        }

        Ok(output)
    }
}

/// Parsed query command
#[derive(Debug)]
pub enum Query {
    Find {
        model: String,
        id: String,
    },
    List {
        model: String,
        limit: Option<usize>,
    },
    Count {
        model: String,
    },
    Create {
        model: String,
        data: Value,
    },
    Update {
        model: String,
        id: String,
        data: Value,
    },
    Delete {
        model: String,
        id: String,
    },
    Sql {
        query: String,
    },
}

/// Simple SQL injection prevention
fn escape_sql(input: &str) -> String {
    input.replace("'", "''")
}

impl Query {
    fn parse(input: &str) -> Result<Self, String> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0] {
            "find" => {
                if parts.len() < 3 {
                    return Err("find requires: find <Model> <id>".to_string());
                }
                Ok(Query::Find {
                    model: parts[1].to_string(),
                    id: parts[2].to_string(),
                })
            }
            "list" => {
                if parts.len() < 2 {
                    return Err("list requires: list <Model> [--limit N]".to_string());
                }

                let mut limit = None;
                if parts.len() >= 4 && parts[2] == "--limit" {
                    limit = Some(parts[3].parse().map_err(|_| "limit must be a number".to_string())?);
                }

                Ok(Query::List {
                    model: parts[1].to_string(),
                    limit,
                })
            }
            "count" => {
                if parts.len() < 2 {
                    return Err("count requires: count <Model>".to_string());
                }
                Ok(Query::Count {
                    model: parts[1].to_string(),
                })
            }
            "all" => {
                if parts.len() < 2 {
                    return Err("all requires: all <Model>".to_string());
                }
                Ok(Query::List {
                    model: parts[1].to_string(),
                    limit: None, // No limit for 'all'
                })
            }
            "create" => {
                if parts.len() < 3 {
                    return Err("create requires: create <Model> {...json...}".to_string());
                }

                let json_str = input
                    .split_once('{')
                    .ok_or("create requires JSON data starting with {".to_string())?
                    .1;
                let json_str = format!("{{{}", json_str);

                let data: Value = serde_json::from_str(&json_str)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                Ok(Query::Create {
                    model: parts[1].to_string(),
                    data,
                })
            }
            "update" => {
                if parts.len() < 4 {
                    return Err("update requires: update <Model> <id> {...json...}".to_string());
                }

                let json_str = input
                    .split_once('{')
                    .ok_or("update requires JSON data starting with {".to_string())?
                    .1;
                let json_str = format!("{{{}", json_str);

                let data: Value = serde_json::from_str(&json_str)
                    .map_err(|e| format!("Invalid JSON: {}", e))?;

                Ok(Query::Update {
                    model: parts[1].to_string(),
                    id: parts[2].to_string(),
                    data,
                })
            }
            "delete" => {
                if parts.len() < 3 {
                    return Err("delete requires: delete <Model> <id>".to_string());
                }
                Ok(Query::Delete {
                    model: parts[1].to_string(),
                    id: parts[2].to_string(),
                })
            }
            "sql" => {
                if parts.len() < 2 {
                    return Err("sql requires: sql <query>".to_string());
                }
                let sql = input.strip_prefix("sql").unwrap_or("").trim().to_string();
                if sql.is_empty() {
                    return Err("sql requires a query".to_string());
                }
                Ok(Query::Sql { query: sql })
            }
            _ => Err(format!(
                "Unknown command: '{}'. Type 'help' for available commands.",
                parts[0]
            )),
        }
    }
}
