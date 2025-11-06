use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandId};
use foundry_plugins::{
    FoundryCommand, CommandContext, CommandError, CommandResult, CommandStatus,
};
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::Path;

pub struct MakeGraphQLTypeCommand {
    descriptor: CommandDescriptor,
}

impl MakeGraphQLTypeCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("make:graphql-type", "make:graphql-type")
                .summary("Generate a new GraphQL type with resolvers")
                .description("Generate a GraphQL type, resolver, and optionally a Sea-ORM model with migration")
                .build(),
        }
    }
}

impl Default for MakeGraphQLTypeCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FoundryCommand for MakeGraphQLTypeCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let name = ctx
            .args
            .get(0)
            .ok_or_else(|| {
                CommandError::Message("Type name is required".to_string())
            })?;

        let with_model = ctx.args.contains(&"--model".to_string()) || ctx.args.contains(&"-m".to_string());
        let with_migration = ctx.args.contains(&"--migration".to_string()) || ctx.args.contains(&"-M".to_string());

        let type_name = capitalize(name);
        let snake_name = to_snake_case(name);

        // Create GraphQL type file
        let type_path = format!("crates/foundry-graphql/src/types/{}.rs", snake_name);
        if Path::new(&type_path).exists() {
            return Ok(CommandResult {
                status: CommandStatus::Failure,
                message: Some(format!("GraphQL type {} already exists", type_name)),
                data: None,
                error: None,
            });
        }

        let type_content = generate_type_template(&type_name, &snake_name);
        fs::create_dir_all("crates/foundry-graphql/src/types")
            .map_err(|e| CommandError::Message(format!("Failed to create types directory: {}", e)))?;
        fs::write(&type_path, type_content)
            .map_err(|e| CommandError::Message(format!("Failed to write type file: {}", e)))?;

        // Create resolver file
        let resolver_path = format!("crates/foundry-graphql/src/resolvers/{}.rs", snake_name);
        let resolver_content = generate_resolver_template(&type_name, &snake_name);
        fs::create_dir_all("crates/foundry-graphql/src/resolvers")
            .map_err(|e| CommandError::Message(format!("Failed to create resolvers directory: {}", e)))?;
        fs::write(&resolver_path, resolver_content)
            .map_err(|e| CommandError::Message(format!("Failed to write resolver file: {}", e)))?;

        // Update mod.rs files
        update_mod_file(
            "crates/foundry-graphql/src/types/mod.rs",
            &snake_name,
            &type_name,
        )?;
        update_mod_file(
            "crates/foundry-graphql/src/resolvers/mod.rs",
            &snake_name,
            &type_name,
        )?;

        let mut messages = vec![
            format!("✅ GraphQL type created: {}", type_path),
            format!("✅ Resolver created: {}", resolver_path),
        ];

        if with_model {
            messages.push(format!("📝 Remember to add Sea-ORM model for {}", type_name));
        }

        if with_migration {
            messages.push(format!(
                "📝 Remember to create migration: rustforge make:migration create_{}_table",
                snake_name
            ));
        }

        messages.push("\n📋 Next steps:".to_string());
        messages.push("1. Add your type to QueryRoot or MutationRoot in lib.rs".to_string());
        messages.push("2. Implement your resolver logic".to_string());
        messages.push("3. Test your GraphQL queries".to_string());

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(messages.join("\n")),
            data: Some(json!({
                "type_name": type_name,
                "type_path": type_path,
                "resolver_path": resolver_path,
            })),
            error: None,
        })
    }
}

fn generate_type_template(type_name: &str, snake_name: &str) -> String {
    let table_name = format!("{}s", snake_name);
    format!(
        r#"use async_graphql::{{InputObject, SimpleObject}};
use chrono::{{DateTime, Utc}};
use sea_orm::entity::prelude::*;
use serde::{{Deserialize, Serialize}};

/// {} Entity for Sea-ORM
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "{}")]
pub struct Model {{
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub active: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {{}}

impl ActiveModelBehavior for ActiveModel {{}}

/// GraphQL {} Type
#[derive(SimpleObject, Clone, Debug)]
pub struct {} {{
    pub id: i64,
    pub name: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}}

impl From<Model> for {} {{
    fn from(model: Model) -> Self {{
        Self {{
            id: model.id,
            name: model.name,
            active: model.active,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.and_utc(),
        }}
    }}
}}

/// Input for creating a new {}
#[derive(InputObject, Debug)]
pub struct {}Input {{
    pub name: String,
    pub active: Option<bool>,
}}

/// Input for updating a {}
#[derive(InputObject, Debug)]
pub struct Update{}Input {{
    pub name: Option<String>,
    pub active: Option<bool>,
}}

pub use Entity as {}Entity;
"#,
        type_name, table_name, type_name, type_name, type_name, type_name, type_name, type_name,
        type_name, type_name
    )
}

fn generate_resolver_template(type_name: &str, snake_name: &str) -> String {
    let query_name = snake_name;
    let query_name_plural = format!("{}s", snake_name);

    format!(
        r#"use crate::context::GraphQLContext;
use crate::error::{{database_error, not_found}};
use crate::types::{}::{{
    {}, {}Entity, {}Input, Update{}Input,
}};
use async_graphql::{{Context, Object, Result}};
use chrono::Utc;
use sea_orm::{{
    ActiveModelTrait, ActiveValue, EntityTrait, PaginatorTrait, QueryOrder,
}};

#[derive(Default)]
pub struct {}Query;

#[Object]
impl {}Query {{
    /// Get a {} by ID
    async fn {}(&self, ctx: &Context<'_>, id: i64) -> Result<{}> {{
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let item = {}Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("{} with id {{}} not found", id)))?;

        Ok(item.into())
    }}

    /// Get all {}s with optional pagination
    async fn {}(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 0)] offset: u64,
        #[graphql(default = 10)] limit: u64,
    ) -> Result<Vec<{}>> {{
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let items = {}Entity::find()
            .order_by_asc(crate::types::{}::Column::Id)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await
            .map_err(database_error)?;

        Ok(items.into_iter().map(|i| i.into()).collect())
    }}

    /// Count total {}s
    async fn {}_count(&self, ctx: &Context<'_>) -> Result<u64> {{
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        {}Entity::find()
            .count(db)
            .await
            .map_err(database_error)
    }}
}}

#[derive(Default)]
pub struct {}Mutation;

#[Object]
impl {}Mutation {{
    /// Create a new {}
    async fn create_{}(&self, ctx: &Context<'_>, input: {}Input) -> Result<{}> {{
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let now = Utc::now().naive_utc();
        let item = crate::types::{}::ActiveModel {{
            name: ActiveValue::Set(input.name),
            active: ActiveValue::Set(input.active.unwrap_or(true)),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        }};

        let result = item.insert(db).await.map_err(database_error)?;
        Ok(result.into())
    }}

    /// Update an existing {}
    async fn update_{}(
        &self,
        ctx: &Context<'_>,
        id: i64,
        input: Update{}Input,
    ) -> Result<{}> {{
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let item = {}Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("{} with id {{}} not found", id)))?;

        let mut active: crate::types::{}::ActiveModel = item.into();

        if let Some(name) = input.name {{
            active.name = ActiveValue::Set(name);
        }}
        if let Some(active_flag) = input.active {{
            active.active = ActiveValue::Set(active_flag);
        }}

        active.updated_at = ActiveValue::Set(Utc::now().naive_utc());

        let result = active.update(db).await.map_err(database_error)?;
        Ok(result.into())
    }}

    /// Delete a {}
    async fn delete_{}(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {{
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let item = {}Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("{} with id {{}} not found", id)))?;

        let active: crate::types::{}::ActiveModel = item.into();
        active.delete(db).await.map_err(database_error)?;

        Ok(true)
    }}
}}
"#,
        snake_name,
        type_name,
        type_name,
        type_name,
        type_name,
        type_name,
        type_name,
        type_name,
        query_name,
        type_name,
        type_name,
        type_name,
        snake_name,
        query_name_plural,
        query_name_plural,
        type_name,
        type_name,
        snake_name,
        snake_name,
        query_name_plural,
        type_name,
        type_name,
        type_name,
        type_name,
        query_name,
        type_name,
        type_name,
        snake_name,
        type_name,
        query_name,
        type_name,
        type_name,
        type_name,
        type_name,
        snake_name,
        type_name,
        query_name,
        type_name,
        type_name,
    )
}

fn update_mod_file(path: &str, snake_name: &str, _type_name: &str) -> Result<(), CommandError> {
    let content = if Path::new(path).exists() {
        fs::read_to_string(path)
            .map_err(|e| CommandError::Message(format!("Failed to read {}: {}", path, e)))?
    } else {
        String::new()
    };

    if content.contains(&format!("pub mod {};", snake_name)) {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| CommandError::Message(format!("Failed to open {}: {}", path, e)))?;

    writeln!(file, "pub mod {};", snake_name)
        .map_err(|e| CommandError::Message(format!("Failed to write to {}: {}", path, e)))?;

    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

