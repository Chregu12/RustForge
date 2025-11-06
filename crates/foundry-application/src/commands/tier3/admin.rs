//! Admin panel commands

use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};

/// make:admin-resource <Model>
pub struct AdminResourceCommand {
    descriptor: CommandDescriptor,
}

impl AdminResourceCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("make:admin-resource", "make:admin-resource")
                .summary("Generate an admin resource")
                .description("Generate an admin resource for CRUD operations in the admin panel")
                .category(CommandKind::Generator)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for AdminResourceCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let model_name = ctx
            .args
            .first()
            .ok_or_else(|| CommandError::Message("Model name required".to_string()))?;

        let resource_path = format!("app/Admin/{}.rs", model_name);
        let content = format!(
            r#"//! Admin resource for {}

use foundry_admin::{{AdminResource, ResourceConfig, FieldConfig, FieldType, ListQuery, ListResult}};
use async_trait::async_trait;
use serde_json::Value;

pub struct {}Resource;

impl {}Resource {{
    pub fn new() -> Self {{
        Self
    }}
}}

#[async_trait]
impl AdminResource for {}Resource {{
    fn config(&self) -> &ResourceConfig {{
        static CONFIG: ResourceConfig = ResourceConfig {{
            name: "{}",
            label: "{}",
            icon: Some("users"),
            fields: vec![
                FieldConfig {{
                    name: "id",
                    label: "ID",
                    field_type: FieldType::Text,
                    required: false,
                    readonly: true,
                    help_text: None,
                }},
                FieldConfig {{
                    name: "name",
                    label: "Name",
                    field_type: FieldType::Text,
                    required: true,
                    readonly: false,
                    help_text: None,
                }},
            ],
            searchable_fields: vec!["name".to_string()],
            filterable_fields: vec![],
            sortable_fields: vec!["id".to_string(), "name".to_string()],
        }};
        &CONFIG
    }}

    async fn list(&self, query: ListQuery) -> anyhow::Result<ListResult> {{
        // TODO: Implement database query
        Ok(ListResult {{
            data: vec![],
            total: 0,
            page: query.page,
            per_page: query.per_page,
            total_pages: 0,
        }})
    }}

    async fn get(&self, id: &str) -> anyhow::Result<Option<Value>> {{
        // TODO: Implement database lookup
        Ok(None)
    }}

    async fn create(&self, data: Value) -> anyhow::Result<Value> {{
        // TODO: Implement database insert
        Ok(data)
    }}

    async fn update(&self, id: &str, data: Value) -> anyhow::Result<Value> {{
        // TODO: Implement database update
        Ok(data)
    }}

    async fn delete(&self, id: &str) -> anyhow::Result<()> {{
        // TODO: Implement database delete
        Ok(())
    }}

    async fn validate(&self, data: &Value, is_update: bool) -> anyhow::Result<ValidationResult> {{
        // TODO: Implement validation
        Ok(ValidationResult::ok())
    }}
}}
"#,
            model_name, model_name, model_name, model_name, model_name.to_lowercase(), model_name
        );

        ctx.artifacts.write_file(&resource_path, &content, ctx.options.force)?;

        Ok(CommandResult::success(format!(
            "Admin resource created: {}",
            resource_path
        )))
    }
}

/// admin:publish
pub struct AdminPublishCommand {
    descriptor: CommandDescriptor,
}

impl AdminPublishCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("admin:publish", "admin:publish")
                .summary("Publish admin panel assets")
                .description("Publish admin panel assets and configuration files")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for AdminPublishCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let config_path = "config/admin.toml";
        let config_content = r##"# Admin Panel Configuration

[admin]
prefix = "/admin"
require_auth = true
title = "Foundry Admin"

[theme]
primary_color = "#3b82f6"
sidebar_color = "#1f2937"
dark_mode = false

[pagination]
per_page = 25
max_per_page = 100
"##;

        ctx.artifacts.write_file(config_path, config_content, ctx.options.force)?;

        Ok(CommandResult::success("Admin configuration published"))
    }
}
