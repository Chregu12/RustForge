use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RouteEntry {
    method: String,
    path: String,
    name: String,
    handler: String,
}

pub struct RouteListCommand {
    descriptor: CommandDescriptor,
}

impl Default for RouteListCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteListCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.route_list", "route:list")
            .summary("Zeigt alle registrierten HTTP-Routes")
            .description("Listet alle HTTP-Routen mit Methode, Path, Controller und Name auf.")
            .category(CommandKind::Utility)
            .build();

        Self { descriptor }
    }

    fn get_routes() -> Vec<RouteEntry> {
        // Core routes from HttpServer
        vec![
            RouteEntry {
                method: "GET".to_string(),
                path: "/health".to_string(),
                name: "health".to_string(),
                handler: "foundry_api::http::health".to_string(),
            },
            RouteEntry {
                method: "GET".to_string(),
                path: "/commands".to_string(),
                name: "commands".to_string(),
                handler: "foundry_api::http::commands".to_string(),
            },
            RouteEntry {
                method: "POST".to_string(),
                path: "/invoke".to_string(),
                name: "invoke".to_string(),
                handler: "foundry_api::http::invoke".to_string(),
            },
            RouteEntry {
                method: "POST".to_string(),
                path: "/upload".to_string(),
                name: "upload".to_string(),
                handler: "foundry_api::upload::upload_file".to_string(),
            },
        ]
    }

    fn format_as_table(routes: &[RouteEntry]) -> String {
        let mut lines = vec![
            String::from("╭────────┬──────────────────┬──────────────────────────────────┬───────────╮"),
            String::from("│ METHOD │ PATH             │ HANDLER                          │ NAME      │"),
            String::from("├────────┼──────────────────┼──────────────────────────────────┼───────────┤"),
        ];

        for route in routes {
            let line = format!(
                "│ {:<6} │ {:<16} │ {:<32} │ {:<9} │",
                route.method, route.path, route.handler, route.name
            );
            lines.push(line);
        }

        lines.push(String::from("╰────────┴──────────────────┴──────────────────────────────────┴───────────╯"));
        lines.join("\n")
    }
}

#[async_trait]
impl FoundryCommand for RouteListCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Check for --format flag
        let use_json = ctx.args.iter().any(|arg| {
            arg == "--format=json" || arg == "--json"
        });

        let routes = Self::get_routes();
        let total = routes.len();

        let message = if use_json {
            format!("{} routes registered", total)
        } else {
            let table = Self::format_as_table(&routes);
            format!("{}\n\n{} routes registered", table, total)
        };

        let data = json!({
            "total": total,
            "routes": routes,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::ResponseFormat;
    use serde_json::Value;

    #[tokio::test]
    async fn test_route_list_table_format() {
        let command = RouteListCommand::new();
        let ctx = CommandContext {
            args: vec![],
            format: ResponseFormat::Human,
            metadata: Value::Null,
            config: Value::Null,
            options: Default::default(),
            artifacts: std::sync::Arc::new(foundry_infra::LocalArtifactPort::default()),
            migrations: std::sync::Arc::new(foundry_infra::SeaOrmMigrationService::default()),
            seeds: std::sync::Arc::new(foundry_infra::SeaOrmSeedService::default()),
            validation: std::sync::Arc::new(foundry_infra::SimpleValidationService::default()),
            storage: std::sync::Arc::new(foundry_infra::FileStorageAdapter::new(
                std::sync::Arc::new(foundry_storage::manager::StorageManager::new(
                    foundry_storage::config::StorageConfig::from_env()
                ).unwrap())
            )),
            cache: std::sync::Arc::new(foundry_infra::InMemoryCacheStore::default()),
            queue: std::sync::Arc::new(foundry_infra::InMemoryQueue::default()),
            events: std::sync::Arc::new(foundry_infra::InMemoryEventBus::default()),
        };

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
        assert!(result.message.is_some());
        assert!(result.data.is_some());

        let message = result.message.unwrap();
        assert!(message.contains("METHOD"));
        assert!(message.contains("PATH"));
        assert!(message.contains("routes registered"));
    }

    #[tokio::test]
    async fn test_route_list_json_format() {
        let command = RouteListCommand::new();
        let ctx = CommandContext {
            args: vec!["--format=json".to_string()],
            format: ResponseFormat::Json,
            metadata: Value::Null,
            config: Value::Null,
            options: Default::default(),
            artifacts: std::sync::Arc::new(foundry_infra::LocalArtifactPort::default()),
            migrations: std::sync::Arc::new(foundry_infra::SeaOrmMigrationService::default()),
            seeds: std::sync::Arc::new(foundry_infra::SeaOrmSeedService::default()),
            validation: std::sync::Arc::new(foundry_infra::SimpleValidationService::default()),
            storage: std::sync::Arc::new(foundry_infra::FileStorageAdapter::new(
                std::sync::Arc::new(foundry_storage::manager::StorageManager::new(
                    foundry_storage::config::StorageConfig::from_env()
                ).unwrap())
            )),
            cache: std::sync::Arc::new(foundry_infra::InMemoryCacheStore::default()),
            queue: std::sync::Arc::new(foundry_infra::InMemoryQueue::default()),
            events: std::sync::Arc::new(foundry_infra::InMemoryEventBus::default()),
        };

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        let total = data["total"].as_u64().unwrap();
        assert!(total > 0);
        assert!(data["routes"].is_array());
    }
}
