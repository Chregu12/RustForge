//! Factory and seeder commands

use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};

/// make:factory <Model>
pub struct MakeFactoryCommand {
    descriptor: CommandDescriptor,
}

impl MakeFactoryCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("make:factory", "make:factory")
                .summary("Generate a model factory")
                .description("Generate a model factory for test data creation")
                .category(CommandKind::Generator)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for MakeFactoryCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let model_name = ctx
            .args
            .first()
            .ok_or_else(|| CommandError::Message("Model name required".to_string()))?;

        let factory_path = format!("tests/factories/{}_factory.rs", model_name.to_lowercase());
        let content = format!(
            r#"//! {} factory

use async_trait::async_trait;
use foundry_testing::{{Factory, FactoryHelper}};
use serde::{{Deserialize, Serialize}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
    pub id: String,
    pub name: String,
    pub created_at: String,
    // Add more fields as needed
}}

pub struct {}Factory {{
    // Add configuration fields
}}

impl {}Factory {{
    pub fn new() -> Self {{
        Self {{}}
    }}
}}

#[async_trait]
impl Factory<{}> for {}Factory {{
    async fn create(&self) -> anyhow::Result<{}> {{
        let instance = self.build()?;
        // TODO: Save to database
        Ok(instance)
    }}

    fn build(&self) -> anyhow::Result<{}> {{
        Ok({} {{
            id: FactoryHelper::uuid(),
            name: FactoryHelper::name(),
            created_at: FactoryHelper::timestamp(),
        }})
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_factory_build() {{
        let factory = {}Factory::new();
        let instance = factory.build().unwrap();
        assert!(!instance.id.is_empty());
        assert!(!instance.name.is_empty());
    }}

    #[tokio::test]
    async fn test_factory_create_many() {{
        let factory = {}Factory::new();
        let instances = factory.create_many(5).await.unwrap();
        assert_eq!(instances.len(), 5);
    }}
}}
"#,
            model_name, model_name, model_name, model_name, model_name, model_name, model_name,
            model_name, model_name, model_name, model_name
        );

        ctx.artifacts.write_file(&factory_path, &content, ctx.options.force)?;

        Ok(CommandResult::success(format!(
            "Factory created: {}",
            factory_path
        )))
    }
}

/// make:seeder <Name>
pub struct MakeSeederCommand {
    descriptor: CommandDescriptor,
}

impl MakeSeederCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("make:seeder", "make:seeder")
                .summary("Generate a database seeder")
                .description("Generate a database seeder for test data population")
                .category(CommandKind::Generator)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for MakeSeederCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let seeder_name = ctx
            .args
            .first()
            .ok_or_else(|| CommandError::Message("Seeder name required".to_string()))?;

        let seeder_path = format!("tests/seeders/{}.rs", seeder_name.to_lowercase());
        let content = format!(
            r#"//! {} seeder

use async_trait::async_trait;
use foundry_testing::Seeder;

pub struct {} {{
    count: usize,
}}

impl {} {{
    pub fn new(count: usize) -> Self {{
        Self {{ count }}
    }}
}}

#[async_trait]
impl Seeder for {} {{
    async fn run(&self) -> anyhow::Result<()> {{
        println!("Running {} with count: {{}}", self.count);

        // TODO: Implement seeding logic
        // Example:
        // let factory = UserFactory::new();
        // factory.create_many(self.count).await?;

        println!("Seeded {{}} records", self.count);
        Ok(())
    }}

    fn name(&self) -> &str {{
        "{}"
    }}

    fn dependencies(&self) -> Vec<String> {{
        // Return names of seeders that must run first
        vec![]
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_seeder_run() {{
        let seeder = {}::new(10);
        seeder.run().await.unwrap();
    }}
}}
"#,
            seeder_name, seeder_name, seeder_name, seeder_name, seeder_name,
            seeder_name.to_lowercase(), seeder_name
        );

        ctx.artifacts.write_file(&seeder_path, &content, ctx.options.force)?;

        Ok(CommandResult::success(format!(
            "Seeder created: {}",
            seeder_path
        )))
    }
}
