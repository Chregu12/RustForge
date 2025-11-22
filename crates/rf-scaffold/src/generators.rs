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
pub struct MigrationGenerator<'a> {
    engine: &'a ScaffoldEngine,
}

impl<'a> MigrationGenerator<'a> {
    pub fn new(engine: &'a ScaffoldEngine) -> Self {
        Self { engine }
    }

    pub async fn generate(&self, name: &str) -> ScaffoldResult<PathBuf> {
        let naming = self.engine.naming();
        let snake_name = naming.to_snake_case(name);
        let table_name = naming.to_pascal_case(&snake_name);

        let data = MigrationData {
            name: name.to_string(),
            snake_name: snake_name.clone(),
            table_name: table_name.clone(),
            fields: vec![],
        };

        // Render template
        let content = self.engine.render("migration", &data).await?;

        // Generate migration filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let file_name = format!("{}_{}.rs", timestamp, snake_name);

        // Write to file
        let file_path = self.engine.base_path().join("migrations").join(file_name);

        self.engine.write_file(&file_path, &content, false).await?;

        Ok(file_path)
    }

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
            .map(|(name, field_type)| MigrationField {
                name: name.to_string(),
                pascal_name: naming.to_pascal_case(name),
                column_def: map_type_to_column_def(field_type),
            })
            .collect();

        let data = MigrationData {
            name: migration_name.clone(),
            snake_name: naming.to_snake_case(&migration_name),
            table_name: naming.to_pascal_case(table_name),
            fields: migration_fields,
        };

        // Render template
        let content = self.engine.render("migration", &data).await?;

        // Generate migration filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let file_name = format!("{}_{}.rs", timestamp, naming.to_snake_case(&migration_name));

        // Write to file
        let file_path = self.engine.base_path().join("migrations").join(file_name);

        self.engine.write_file(&file_path, &content, false).await?;

        Ok(file_path)
    }
}

#[derive(Serialize)]
struct MigrationData {
    name: String,
    snake_name: String,
    table_name: String,
    fields: Vec<MigrationField>,
}

#[derive(Serialize)]
struct MigrationField {
    name: String,
    pascal_name: String,
    column_def: String,
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

/// Map Rust type to SeaORM column definition
fn map_type_to_column_def(rust_type: &str) -> String {
    match rust_type {
        "String" => ".string().not_null()".to_string(),
        "i32" => ".integer().not_null()".to_string(),
        "i64" => ".big_integer().not_null()".to_string(),
        "f32" => ".float().not_null()".to_string(),
        "f64" => ".double().not_null()".to_string(),
        "bool" => ".boolean().not_null()".to_string(),
        "DateTime<Utc>" => ".timestamp().not_null()".to_string(),
        _ => ".string()".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_map_type_to_column_def() {
        assert_eq!(map_type_to_column_def("String"), ".string().not_null()");
        assert_eq!(map_type_to_column_def("i32"), ".integer().not_null()");
        assert_eq!(map_type_to_column_def("i64"), ".big_integer().not_null()");
        assert_eq!(map_type_to_column_def("bool"), ".boolean().not_null()");
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

    #[tokio::test]
    async fn test_migration_generator() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();

        let result = engine.generate_migration("create_users_table").await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("create_users_table"));
    }
}
