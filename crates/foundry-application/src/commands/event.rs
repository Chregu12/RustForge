use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EventListEntry {
    event: String,
    listener: String,
    handler: String,
}

pub struct EventListCommand {
    descriptor: CommandDescriptor,
}

impl Default for EventListCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl EventListCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.event_list", "event:list")
            .summary("Zeigt Events & Listener")
            .description("Listet alle Events und ihre zugehörigen Listener aus app/events/ und app/listeners/ auf.")
            .category(CommandKind::Utility)
            .build();

        Self { descriptor }
    }

    fn get_events_path() -> PathBuf {
        PathBuf::from("app/events")
    }

    fn get_listeners_path() -> PathBuf {
        PathBuf::from("app/listeners")
    }

    fn read_events(path: &Path) -> Result<Vec<String>, std::io::Error> {
        let mut events = Vec::new();

        if !path.exists() {
            return Ok(events);
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                if let Some(file_name) = entry_path.file_name() {
                    let name = file_name.to_string_lossy().to_string();
                    if name.ends_with(".rs") && name != "mod.rs" {
                        // Convert snake_case file name to PascalCase event name
                        let event_name = Self::snake_to_pascal(&name.replace(".rs", ""));
                        events.push(event_name);
                    }
                }
            }
        }

        events.sort();
        Ok(events)
    }

    fn read_listeners(path: &Path) -> Result<Vec<String>, std::io::Error> {
        let mut listeners = Vec::new();

        if !path.exists() {
            return Ok(listeners);
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                if let Some(file_name) = entry_path.file_name() {
                    let name = file_name.to_string_lossy().to_string();
                    if name.ends_with(".rs") && name != "mod.rs" {
                        // Convert snake_case file name to PascalCase listener name
                        let listener_name = Self::snake_to_pascal(&name.replace(".rs", ""));
                        listeners.push(listener_name);
                    }
                }
            }
        }

        listeners.sort();
        Ok(listeners)
    }

    fn snake_to_pascal(snake: &str) -> String {
        snake
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }

    fn map_events_to_listeners(events: Vec<String>, listeners: Vec<String>) -> Vec<EventListEntry> {
        let mut entries = Vec::new();

        // For each event, find matching listeners
        for event in &events {
            let event_base = event.replace("Event", "");
            let mut found_listener = false;

            for listener in &listeners {
                // Simple heuristic: listener name contains event base name
                if listener.to_lowercase().contains(&event_base.to_lowercase()) {
                    entries.push(EventListEntry {
                        event: event.clone(),
                        listener: listener.clone(),
                        handler: format!("app::listeners::{}", Self::pascal_to_snake(listener)),
                    });
                    found_listener = true;
                }
            }

            // If no matching listener found, show event without listener
            if !found_listener {
                entries.push(EventListEntry {
                    event: event.clone(),
                    listener: "-".to_string(),
                    handler: "-".to_string(),
                });
            }
        }

        // Add listeners that don't match any event
        for listener in &listeners {
            let has_match = entries.iter().any(|e| e.listener == *listener);
            if !has_match {
                entries.push(EventListEntry {
                    event: "-".to_string(),
                    listener: listener.clone(),
                    handler: format!("app::listeners::{}", Self::pascal_to_snake(listener)),
                });
            }
        }

        entries
    }

    fn pascal_to_snake(pascal: &str) -> String {
        let mut result = String::new();
        for (i, ch) in pascal.chars().enumerate() {
            if ch.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        }
        result
    }

    fn format_as_table(entries: &[EventListEntry]) -> String {
        let mut lines = vec![
            String::from("╭──────────────────────────────┬──────────────────────────────┬────────────────────────────────────────────╮"),
            String::from("│ EVENT                        │ LISTENER                     │ HANDLER                                    │"),
            String::from("├──────────────────────────────┼──────────────────────────────┼────────────────────────────────────────────┤"),
        ];

        for entry in entries {
            let line = format!(
                "│ {:<28} │ {:<28} │ {:<42} │",
                truncate(&entry.event, 28),
                truncate(&entry.listener, 28),
                truncate(&entry.handler, 42)
            );
            lines.push(line);
        }

        lines.push(String::from("╰──────────────────────────────┴──────────────────────────────┴────────────────────────────────────────────╯"));
        lines.join("\n")
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[0..max_len - 3])
    } else {
        s.to_string()
    }
}

#[async_trait]
impl FoundryCommand for EventListCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Check for --format flag
        let use_json = ctx.args.iter().any(|arg| {
            arg == "--format=json" || arg == "--json"
        });

        let events_path = Self::get_events_path();
        let listeners_path = Self::get_listeners_path();

        let events = Self::read_events(&events_path).unwrap_or_default();
        let listeners = Self::read_listeners(&listeners_path).unwrap_or_default();

        let entries = Self::map_events_to_listeners(events.clone(), listeners.clone());
        let total = entries.len();

        let message = if use_json {
            format!("{} event-listener mappings found", total)
        } else {
            let table = Self::format_as_table(&entries);
            format!("{}\n\n{} event-listener mappings found", table, total)
        };

        let data = json!({
            "total": total,
            "events": entries,
            "events_path": events_path.display().to_string(),
            "listeners_path": listeners_path.display().to_string(),
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

    fn create_test_context(args: Vec<String>) -> CommandContext {
        CommandContext {
            args,
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
        }
    }

    #[tokio::test]
    async fn test_event_list_table_format() {
        let command = EventListCommand::new();
        let ctx = create_test_context(vec![]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_event_list_json_format() {
        let command = EventListCommand::new();
        let ctx = create_test_context(vec!["--format=json".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        assert!(data["total"].is_number());
        assert!(data["events"].is_array());
    }

    #[test]
    fn test_snake_to_pascal() {
        assert_eq!(EventListCommand::snake_to_pascal("order_placed_event"), "OrderPlacedEvent");
        assert_eq!(EventListCommand::snake_to_pascal("send_order_email_listener"), "SendOrderEmailListener");
    }

    #[test]
    fn test_pascal_to_snake() {
        assert_eq!(EventListCommand::pascal_to_snake("OrderPlacedEvent"), "order_placed_event");
        assert_eq!(EventListCommand::pascal_to_snake("SendOrderEmailListener"), "send_order_email_listener");
    }

    #[tokio::test]
    async fn test_event_list_descriptor() {
        let command = EventListCommand::new();
        let descriptor = command.descriptor();

        assert_eq!(descriptor.name, "event:list");
        assert_eq!(descriptor.category, CommandKind::Utility);
    }
}
