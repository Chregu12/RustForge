//! Application Layer für Foundry Core.

pub mod auth;
pub mod lazy_config;
mod commands;
mod error;
mod registry;

pub use commands::{ListCommand, TestCommand};
pub use error::ApplicationError;
pub use registry::CommandRegistry;

// Re-export service container
pub use foundry_service_container::{
    Container, ProviderRegistry, ServiceProvider, ApplicationServiceProvider,
    AuthServiceProvider, CacheServiceProvider, DatabaseServiceProvider,
    MailServiceProvider,
};

use commands::BootstrapCommands;
use foundry_infra::{
    FileStorageAdapter, InMemoryCacheStore, InMemoryEventBus, InMemoryQueue,
    SimpleValidationService,
};
use foundry_plugins::{
    ArtifactPort, CachePort, CommandContext, CommandResult, EventPort, ExecutionOptions,
    MigrationPort, QueuePort, ResponseFormat, SeedPort, StoragePort, ValidationPort,
};
use serde_json::Value;
use std::sync::Arc;

use foundry_storage::config::StorageConfig;
use foundry_storage::manager::StorageManager;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct FoundryApp {
    registry: CommandRegistry,
    config: Value,
    artifacts: Arc<dyn ArtifactPort>,
    migrations: Arc<dyn MigrationPort>,
    seeds: Arc<dyn SeedPort>,
    storage_manager: Arc<StorageManager>,
    validation: Arc<dyn ValidationPort>,
    storage: Arc<dyn StoragePort>,
    cache: Arc<dyn CachePort>,
    queue: Arc<dyn QueuePort>,
    events: Arc<dyn EventPort>,
    container: Container,
    providers: ProviderRegistry,
}

impl FoundryApp {
    pub fn bootstrap(
        config: Value,
        artifacts: Arc<dyn ArtifactPort>,
        migrations: Arc<dyn MigrationPort>,
        seeds: Arc<dyn SeedPort>,
    ) -> Result<Self, ApplicationError> {
        FoundryAppBuilder::new(config, artifacts, migrations, seeds).build()
    }

    pub fn builder(
        config: Value,
        artifacts: Arc<dyn ArtifactPort>,
        migrations: Arc<dyn MigrationPort>,
        seeds: Arc<dyn SeedPort>,
    ) -> FoundryAppBuilder {
        FoundryAppBuilder::new(config, artifacts, migrations, seeds)
    }

    pub fn registry(&self) -> CommandRegistry {
        self.registry.clone()
    }

    pub fn config(&self) -> &Value {
        &self.config
    }

    pub fn storage_manager(&self) -> Arc<StorageManager> {
        self.storage_manager.clone()
    }

    pub fn validation(&self) -> Arc<dyn ValidationPort> {
        self.validation.clone()
    }

    pub fn container(&self) -> Container {
        self.container.clone()
    }

    pub fn providers(&self) -> ProviderRegistry {
        self.providers.clone()
    }

    #[instrument(skip(self, args), fields(command, num_args = args.len()))]
    pub async fn dispatch(
        &self,
        command: &str,
        args: Vec<String>,
        format: ResponseFormat,
        options: ExecutionOptions,
    ) -> Result<CommandResult, ApplicationError> {
        info!("Dispatching command: {}", command);

        let handle = self
            .registry
            .resolve(command)?
            .ok_or_else(|| ApplicationError::CommandNotFound(command.to_string()))?;

        let catalog = self.registry.descriptors()?;
        let args_snapshot = args.clone();
        let metadata = serde_json::json!({
            "invocation": {
                "command": command,
                "args": args_snapshot,
                "format": format,
                "options": options,
            },
            "catalog": catalog,
        });

        let ctx = CommandContext {
            args,
            format,
            metadata,
            config: self.config.clone(),
            options,
            artifacts: self.artifacts.clone(),
            migrations: self.migrations.clone(),
            seeds: self.seeds.clone(),
            validation: self.validation.clone(),
            storage: self.storage.clone(),
            cache: self.cache.clone(),
            queue: self.queue.clone(),
            events: self.events.clone(),
        };

        let result = handle.execute(ctx).await?;
        Ok(result)
    }
}

pub struct FoundryAppBuilder {
    config: Value,
    artifacts: Arc<dyn ArtifactPort>,
    migrations: Arc<dyn MigrationPort>,
    seeds: Arc<dyn SeedPort>,
    validation: Option<Arc<dyn ValidationPort>>,
    storage: Option<Arc<dyn StoragePort>>,
    cache: Option<Arc<dyn CachePort>>,
    queue: Option<Arc<dyn QueuePort>>,
    events: Option<Arc<dyn EventPort>>,
    container: Option<Container>,
    providers: Option<ProviderRegistry>,
}

impl FoundryAppBuilder {
    pub fn new(
        config: Value,
        artifacts: Arc<dyn ArtifactPort>,
        migrations: Arc<dyn MigrationPort>,
        seeds: Arc<dyn SeedPort>,
    ) -> Self {
        Self {
            config,
            artifacts,
            migrations,
            seeds,
            validation: None,
            storage: None,
            cache: None,
            queue: None,
            events: None,
            container: None,
            providers: None,
        }
    }

    pub fn with_validation_port(mut self, port: Arc<dyn ValidationPort>) -> Self {
        self.validation = Some(port);
        self
    }

    pub fn with_storage_port(mut self, port: Arc<dyn StoragePort>) -> Self {
        self.storage = Some(port);
        self
    }

    pub fn with_cache_port(mut self, port: Arc<dyn CachePort>) -> Self {
        self.cache = Some(port);
        self
    }

    pub fn with_queue_port(mut self, port: Arc<dyn QueuePort>) -> Self {
        self.queue = Some(port);
        self
    }

    pub fn with_event_port(mut self, port: Arc<dyn EventPort>) -> Self {
        self.events = Some(port);
        self
    }

    pub fn with_container(mut self, container: Container) -> Self {
        self.container = Some(container);
        self
    }

    pub fn with_providers(mut self, providers: ProviderRegistry) -> Self {
        self.providers = Some(providers);
        self
    }

    pub fn build(self) -> Result<FoundryApp, ApplicationError> {
        let FoundryAppBuilder {
            config,
            artifacts,
            migrations,
            seeds,
            validation,
            storage,
            cache,
            queue,
            events,
            container,
            providers,
        } = self;

        let registry = CommandRegistry::default();
        BootstrapCommands::register_all(&registry)?;

        let storage_config = StorageConfig::from_env();
        let storage_manager = Arc::new(
            StorageManager::new(storage_config)
                .map_err(|e| ApplicationError::StorageError(e.to_string()))?,
        );

        // Initialize service container and providers
        let container = container.unwrap_or_else(Container::new);
        let providers = providers.unwrap_or_else(ProviderRegistry::new);

        Ok(FoundryApp {
            registry,
            config,
            artifacts,
            migrations,
            seeds,
            storage_manager: storage_manager.clone(),
            validation: validation.unwrap_or_else(|| Arc::new(SimpleValidationService)),
            storage: storage
                .unwrap_or_else(|| Arc::new(FileStorageAdapter::new(storage_manager.clone()))),
            cache: cache.unwrap_or_else(|| Arc::new(InMemoryCacheStore::default())),
            queue: queue.unwrap_or_else(|| Arc::new(InMemoryQueue::default())),
            events: events.unwrap_or_else(|| Arc::new(InMemoryEventBus::default())),
            container,
            providers,
        })
    }
}
