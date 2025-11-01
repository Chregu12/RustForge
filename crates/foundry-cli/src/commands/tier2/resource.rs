//! make:resource command

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "make:resource", about = "Create a new API resource")]
pub struct MakeResourceCommand {
    /// Resource name
    pub name: String,

    /// Collection resource
    #[arg(long, short = 'c')]
    pub collection: bool,
}

impl MakeResourceCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("Creating resource: {}", self.name);
        println!("Collection: {}", self.collection);

        let content = if self.collection {
            self.generate_collection_resource()
        } else {
            self.generate_resource()
        };

        let filename = format!("app/resources/{}.rs", self.name.to_lowercase());
        std::fs::create_dir_all("app/resources")?;
        std::fs::write(&filename, content)?;

        println!("✓ Resource created: {}", filename);
        Ok(())
    }

    fn generate_resource(&self) -> String {
        format!(
            r#"use foundry_resources::{{Resource, ResourceContext}};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct {}Resource {{
    pub id: i64,
    // Add your fields here
}}

impl Resource for {}Resource {{
    type Model = {}Model;

    fn from_model(model: Self::Model) -> Self {{
        Self {{
            id: model.id,
        }}
    }}
}}
"#,
            self.name, self.name, self.name
        )
    }

    fn generate_collection_resource(&self) -> String {
        format!(
            r#"use foundry_resources::{{ResourceCollection, Pagination}};
use super::{}Resource;

pub struct {}Collection;

impl {}Collection {{
    pub fn from_models(models: Vec<{}Model>) -> ResourceCollection<{}Resource> {{
        ResourceCollection::from_models(models)
    }}

    pub fn paginated(
        models: Vec<{}Model>,
        pagination: Pagination,
        total: u64,
    ) -> ResourceCollection<{}Resource> {{
        ResourceCollection::paginated(
            models.into_iter().map({}Resource::from_model).collect(),
            pagination,
            total,
        )
    }}
}}
"#,
            self.name,
            self.name,
            self.name,
            self.name,
            self.name,
            self.name,
            self.name,
            self.name
        )
    }
}
