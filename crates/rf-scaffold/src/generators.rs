//! Code generators for different components

use crate::{ModelOptions, ScaffoldEngine, ScaffoldResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;
use std::path::PathBuf;

/// Generator trait
#[async_trait]
pub trait Generator {
    /// Generate code
    async fn generate(&self) -> ScaffoldResult<PathBuf>;
}

/// Model generator
pub struct ModelGenerator<'a> {
    engine: &'a ScaffoldEngine,
}

impl<'a> ModelGenerator<'a> {
    pub fn new(engine: &'a ScaffoldEngine) -> Self {
        Self { engine }
    }

    pub async fn generate(
        &self,
        name: &str,
        options: &ModelOptions<'_>,
    ) -> ScaffoldResult<PathBuf> {
        let naming = self.engine.naming();
        let pascal_name = naming.to_pascal_case(name);
        let snake_name = naming.to_snake_case(&pascal_name);

        // Prepare template data
        let fields: Vec<FieldData> = options
            .fields
            .iter()
            .map(|(name, field_type)| FieldData {
                name: name.to_string(),
                field_type: field_type.to_string(),
            })
            .collect();

        let data = ModelData {
            name: pascal_name.clone(),
            snake_name: snake_name.clone(),
            fields,
        };

        // Render template
        let content = self.engine.render("model", &data).await?;

        // Write to file
        let file_path = self
            .engine
            .base_path()
            .join("src")
            .join("models")
            .join(format!("{}.rs", snake_name));

        self.engine.write_file(&file_path, &content, false).await?;

        // Generate migration if requested
        if options.with_migration {
            let migration_gen = MigrationGenerator::new(self.engine);
            let table_name = naming.pluralize(&snake_name);
            migration_gen
                .generate_for_model(&pascal_name, &table_name, &options.fields)
                .await?;
        }

        Ok(file_path)
    }
}

#[derive(Serialize)]
struct ModelData {
    name: String,
    snake_name: String,
    fields: Vec<FieldData>,
}

#[derive(Serialize)]
struct FieldData {
    name: String,
    field_type: String,
}

/// Controller generator
pub struct ControllerGenerator<'a> {
    engine: &'a ScaffoldEngine,
}

impl<'a> ControllerGenerator<'a> {
    pub fn new(engine: &'a ScaffoldEngine) -> Self {
        Self { engine }
    }

    pub async fn generate(&self, name: &str, resource: bool) -> ScaffoldResult<PathBuf> {
        let naming = self.engine.naming();
        let pascal_name = naming.to_pascal_case(name);
        let snake_name = naming.to_snake_case(&pascal_name);

        // Extract model name
        let model_name = naming.extract_base(&pascal_name);
        let plural_model = naming.pluralize(&naming.to_snake_case(&model_name));

        let template = if resource {
            "controller_resource"
        } else {
            "controller"
        };

        let data = ControllerData {
            name: pascal_name.clone(),
            snake_name: snake_name.clone(),
            model_name: model_name.clone(),
            plural_model,
            fields: vec![],
        };

        // Render template
        let content = self.engine.render(template, &data).await?;

        // Write to file
        let file_path = self
            .engine
            .base_path()
            .join("src")
            .join("controllers")
            .join(format!("{}.rs", snake_name));

        self.engine.write_file(&file_path, &content, false).await?;

        Ok(file_path)
    }
}

#[derive(Serialize)]
struct ControllerData {
    name: String,
    snake_name: String,
    model_name: String,
    plural_model: String,
    fields: Vec<FieldData>,
}

/// Migration generator
///
/// Canonical migration convention: each migration is a timestamped directory
/// (`migrations/{timestamp}_{name}/`) containing two plain-SQL files:
///   - `up.sql`   — forward migration (CREATE TABLE …)
///   - `down.sql` — rollback (DROP TABLE …)
///
/// All three generators (rf-scaffold, forge-cli, foundry-cli) emit this same
/// format. The canonical runner is `DB::statement(sql)` from rf-orm.
pub struct MigrationGenerator<'a> {
    engine: &'a ScaffoldEngine,
}

impl<'a> MigrationGenerator<'a> {
    pub fn new(engine: &'a ScaffoldEngine) -> Self {
        Self { engine }
    }

    /// Generate a standalone migration skeleton.
    ///
    /// Creates `migrations/{timestamp}_{snake_name}/up.sql` and
    /// `migrations/{timestamp}_{snake_name}/down.sql` with TODO placeholders.
    /// Returns the migration directory path.
    pub async fn generate(&self, name: &str) -> ScaffoldResult<PathBuf> {
        let naming = self.engine.naming();
        let snake_name = naming.to_snake_case(name);

        let data = MigrationData {
            name: name.to_string(),
            snake_name: snake_name.clone(),
            table_sql_name: snake_name.clone(),
            fields: vec![],
        };

        let up_content = self.engine.render("migration_up", &data).await?;
        let down_content = self.engine.render("migration_down", &data).await?;

        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let dir_name = format!("{}_{}", timestamp, snake_name);
        let dir_path = self.engine.base_path().join("migrations").join(&dir_name);

        self.engine
            .write_file(&dir_path.join("up.sql"), &up_content, false)
            .await?;
        self.engine
            .write_file(&dir_path.join("down.sql"), &down_content, false)
            .await?;

        Ok(dir_path)
    }

    /// Generate a model migration with a real CREATE TABLE statement.
    ///
    /// Creates `migrations/{timestamp}_create_{table_name}_table/up.sql` and `down.sql`.
    /// Returns the migration directory path.
    pub async fn generate_for_model(
        &self,
        _model_name: &str,
        table_name: &str,
        fields: &[(&str, &str)],
    ) -> ScaffoldResult<PathBuf> {
        let naming = self.engine.naming();
        let migration_name = format!("create_{}_table", table_name);

        let migration_fields: Vec<MigrationField> = fields
            .iter()
            .map(|(field_name, field_type)| MigrationField {
                name: field_name.to_string(),
                sql_type: map_type_to_sql_type(field_type),
            })
            .collect();

        let data = MigrationData {
            name: migration_name.clone(),
            snake_name: naming.to_snake_case(&migration_name),
            table_sql_name: table_name.to_string(),
            fields: migration_fields,
        };

        let up_content = self.engine.render("migration_model_up", &data).await?;
        let down_content = self.engine.render("migration_model_down", &data).await?;

        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let dir_name = format!("{}_{}", timestamp, naming.to_snake_case(&migration_name));
        let dir_path = self.engine.base_path().join("migrations").join(&dir_name);

        self.engine
            .write_file(&dir_path.join("up.sql"), &up_content, false)
            .await?;
        self.engine
            .write_file(&dir_path.join("down.sql"), &down_content, false)
            .await?;

        Ok(dir_path)
    }
}

#[derive(Serialize)]
struct MigrationData {
    name: String,
    snake_name: String,
    /// snake_case table name used directly in SQL (e.g. "users", not PascalCase)
    table_sql_name: String,
    fields: Vec<MigrationField>,
}

#[derive(Serialize)]
struct MigrationField {
    name: String,
    /// SQLite column type string, e.g. "TEXT NOT NULL"
    sql_type: String,
}

/// Service generator
pub struct ServiceGenerator<'a> {
    engine: &'a ScaffoldEngine,
}

impl<'a> ServiceGenerator<'a> {
    pub fn new(engine: &'a ScaffoldEngine) -> Self {
        Self { engine }
    }

    pub async fn generate(&self, name: &str) -> ScaffoldResult<PathBuf> {
        let naming = self.engine.naming();
        let pascal_name = naming.to_pascal_case(name);
        let snake_name = naming.to_snake_case(&pascal_name);

        let data = ServiceData {
            name: pascal_name.clone(),
            snake_name: snake_name.clone(),
        };

        // Render template
        let content = self.engine.render("service", &data).await?;

        // Write to file
        let file_path = self
            .engine
            .base_path()
            .join("src")
            .join("services")
            .join(format!("{}.rs", snake_name));

        self.engine.write_file(&file_path, &content, false).await?;

        Ok(file_path)
    }
}

#[derive(Serialize)]
struct ServiceData {
    name: String,
    snake_name: String,
}

/// Repository generator
pub struct RepositoryGenerator<'a> {
    engine: &'a ScaffoldEngine,
}

impl<'a> RepositoryGenerator<'a> {
    pub fn new(engine: &'a ScaffoldEngine) -> Self {
        Self { engine }
    }

    pub async fn generate(&self, name: &str, model_name: &str) -> ScaffoldResult<PathBuf> {
        let naming = self.engine.naming();
        let pascal_name = naming.to_pascal_case(name);
        let snake_name = naming.to_snake_case(&pascal_name);

        let data = RepositoryData {
            name: pascal_name.clone(),
            snake_name: snake_name.clone(),
            model_name: model_name.to_string(),
        };

        // Render template
        let content = self.engine.render("repository", &data).await?;

        // Write to file
        let file_path = self
            .engine
            .base_path()
            .join("src")
            .join("repositories")
            .join(format!("{}.rs", snake_name));

        self.engine.write_file(&file_path, &content, false).await?;

        Ok(file_path)
    }
}

#[derive(Serialize)]
struct RepositoryData {
    name: String,
    snake_name: String,
    model_name: String,
}

/// Map a Rust type to a SQLite column type string for use in plain-SQL migrations.
///
/// Returns the canonical SQLite DDL type fragment (e.g. `"TEXT NOT NULL"`).
/// This matches the type strings emitted by foundry-cli's `model_migration_up_sql`
/// so that all three generators produce interoperable SQLite DDL.
fn map_type_to_sql_type(rust_type: &str) -> String {
    match rust_type {
        "String" => "TEXT NOT NULL".to_string(),
        "i32" | "i64" | "usize" | "u32" | "u64" => "INTEGER NOT NULL".to_string(),
        "f32" | "f64" => "REAL NOT NULL".to_string(),
        // SQLite stores booleans as INTEGER (0/1)
        "bool" => "INTEGER NOT NULL".to_string(),
        "DateTime<Utc>" | "NaiveDateTime" => "TIMESTAMP NOT NULL".to_string(),
        // Optional variants — nullable columns
        "Option<String>" => "TEXT".to_string(),
        "Option<i32>" | "Option<i64>" | "Option<bool>" => "INTEGER".to_string(),
        "Option<f32>" | "Option<f64>" => "REAL".to_string(),
        "Option<DateTime<Utc>>" | "Option<NaiveDateTime>" => "TIMESTAMP".to_string(),
        _ => "TEXT".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_map_type_to_sql_type() {
        // Canonical SQLite column types — must match foundry-cli's model_migration_up_sql output
        assert_eq!(map_type_to_sql_type("String"), "TEXT NOT NULL");
        assert_eq!(map_type_to_sql_type("i32"), "INTEGER NOT NULL");
        assert_eq!(map_type_to_sql_type("i64"), "INTEGER NOT NULL");
        assert_eq!(map_type_to_sql_type("bool"), "INTEGER NOT NULL");
        assert_eq!(map_type_to_sql_type("f32"), "REAL NOT NULL");
        assert_eq!(map_type_to_sql_type("f64"), "REAL NOT NULL");
        assert_eq!(map_type_to_sql_type("DateTime<Utc>"), "TIMESTAMP NOT NULL");
        assert_eq!(map_type_to_sql_type("Option<String>"), "TEXT");
        assert_eq!(map_type_to_sql_type("Option<i64>"), "INTEGER");
        // Unknown types default to nullable TEXT
        assert_eq!(map_type_to_sql_type("CustomType"), "TEXT");
    }

    #[tokio::test]
    async fn test_model_generator() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();

        let options = ModelOptions {
            fields: vec![("name", "String"), ("age", "i32")],
            with_migration: false,
            with_factory: false,
        };

        let result = engine.generate_model("User", &options).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_controller_generator() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();

        let result = engine.generate_controller("UserController", false).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_service_generator() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();

        let result = engine.generate_service("UserService").await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
    }

    /// Canonical convention: generate_migration returns a directory path (not a .rs file).
    /// The directory contains up.sql and down.sql with plain SQLite DDL.
    #[tokio::test]
    async fn test_migration_generator_emits_plain_sql_dir() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();

        let result = engine.generate_migration("create_users_table").await;
        assert!(result.is_ok(), "generate_migration must succeed");

        let migration_dir = result.unwrap();
        // Returns the directory path, so file_name contains the migration name
        assert!(
            migration_dir.exists(),
            "migration directory must exist: {migration_dir:?}"
        );
        assert!(
            migration_dir
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("create_users_table"),
            "directory name must contain the migration name"
        );

        // up.sql and down.sql must exist inside the directory
        let up_sql = migration_dir.join("up.sql");
        let down_sql = migration_dir.join("down.sql");
        assert!(up_sql.exists(), "up.sql must exist");
        assert!(down_sql.exists(), "down.sql must exist");

        // up.sql is a TODO skeleton (standalone migration has no table name to fill in)
        let up_content = std::fs::read_to_string(&up_sql).unwrap();
        assert!(
            up_content.contains("-- Up migration:"),
            "up.sql must be a plain-SQL file"
        );
        assert!(
            up_content.contains("Runner: DB::statement"),
            "up.sql must document the canonical runner"
        );
    }

    /// Model migration must produce a real CREATE TABLE statement in up.sql.
    #[tokio::test]
    async fn test_model_migration_emits_create_table_sql() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();

        let options = ModelOptions {
            fields: vec![("name", "String"), ("score", "i64")],
            with_migration: true,
            with_factory: false,
        };

        engine.generate_model("Article", &options).await.unwrap();

        // Find the migration directory
        let migrations_dir = dir.path().join("migrations");
        let mut entries: Vec<_> = std::fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        assert!(!entries.is_empty(), "at least one migration directory must exist");
        let migration_dir = entries[0].path();

        let up_sql = migration_dir.join("up.sql");
        let down_sql = migration_dir.join("down.sql");
        assert!(up_sql.exists(), "up.sql must exist for model migration");
        assert!(down_sql.exists(), "down.sql must exist for model migration");

        let up_content = std::fs::read_to_string(&up_sql).unwrap();
        // Must contain a real CREATE TABLE for the pluralised table name
        assert!(
            up_content.contains("CREATE TABLE IF NOT EXISTS articles"),
            "up.sql must CREATE TABLE articles (plural of Article); got:\n{up_content}"
        );
        // Fields must appear with canonical SQLite types
        assert!(
            up_content.contains("name TEXT NOT NULL"),
            "String field must map to TEXT NOT NULL; got:\n{up_content}"
        );
        assert!(
            up_content.contains("score INTEGER NOT NULL"),
            "i64 field must map to INTEGER NOT NULL; got:\n{up_content}"
        );

        let down_content = std::fs::read_to_string(&down_sql).unwrap();
        assert!(
            down_content.contains("DROP TABLE IF EXISTS articles"),
            "down.sql must DROP TABLE articles; got:\n{down_content}"
        );
    }
}
