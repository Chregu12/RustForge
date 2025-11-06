//! Plug-in Contracts für Foundry Core.

pub mod error;
pub use error::{AppError, AppResult, ErrorContextField};

use async_trait::async_trait;
// Re-export CommandDescriptor for convenience
pub use foundry_domain::CommandDescriptor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct CommandContext {
    pub args: Vec<String>,
    pub format: ResponseFormat,
    pub metadata: Value,
    pub config: Value,
    pub options: ExecutionOptions,
    pub artifacts: Arc<dyn ArtifactPort>,
    pub migrations: Arc<dyn MigrationPort>,
    pub seeds: Arc<dyn SeedPort>,
    pub validation: Arc<dyn ValidationPort>,
    pub storage: Arc<dyn StoragePort>,
    pub cache: Arc<dyn CachePort>,
    pub queue: Arc<dyn QueuePort>,
    pub events: Arc<dyn EventPort>,
}

impl fmt::Debug for CommandContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandContext")
            .field("args", &self.args)
            .field("format", &self.format)
            .field("metadata", &self.metadata)
            .field("config", &self.config)
            .field("options", &self.options)
            .finish()
    }
}

impl CommandContext {
    pub async fn validate(
        &self,
        payload: Value,
        rules: ValidationRules,
    ) -> Result<ValidationReport, CommandError> {
        self.validation.validate(payload, rules).await
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ExecutionOptions {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub force: bool,
}

pub trait ArtifactPort: Send + Sync {
    fn write_file(&self, path: &str, contents: &str, force: bool) -> Result<(), CommandError>;
}

#[async_trait]
pub trait MigrationPort: Send + Sync {
    async fn apply(&self, config: &Value, dry_run: bool) -> Result<MigrationRun, CommandError>;

    async fn rollback(&self, config: &Value, dry_run: bool) -> Result<MigrationRun, CommandError>;
}

#[async_trait]
pub trait SeedPort: Send + Sync {
    async fn run(&self, config: &Value, dry_run: bool) -> Result<SeedRun, CommandError>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationRun {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled_back: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SeedRun {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub executed: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Human,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Success,
    Failure,
    Skipped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub status: CommandStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

impl CommandResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            status: CommandStatus::Success,
            message: Some(message.into()),
            data: None,
            error: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn failure(error: AppError) -> Self {
        Self {
            status: CommandStatus::Failure,
            message: Some(error.message.clone()),
            data: None,
            error: Some(error),
        }
    }

    pub fn skipped(message: impl Into<String>) -> Self {
        Self {
            status: CommandStatus::Skipped,
            message: Some(message.into()),
            data: None,
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedArtifact {
    pub path: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeneratorPlan {
    pub artifacts: Vec<GeneratedArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationStep {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub steps: Vec<MigrationStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Message(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type DynCommand = Arc<dyn FoundryCommand>;

#[async_trait]
pub trait FoundryCommand: Send + Sync {
    fn descriptor(&self) -> &CommandDescriptor;
    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError>;
}

#[async_trait]
pub trait FoundryGenerator: FoundryCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError>;
}

#[async_trait]
pub trait FoundryMigration: FoundryCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<MigrationPlan, CommandError>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidationRules {
    pub rules: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationViolation {
    pub field: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ValidationViolation>,
}

impl ValidationReport {
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn with_errors(errors: Vec<ValidationViolation>) -> Self {
        Self {
            valid: errors.is_empty(),
            errors,
        }
    }
}

#[async_trait]
pub trait ValidationPort: Send + Sync {
    async fn validate(
        &self,
        payload: Value,
        rules: ValidationRules,
    ) -> Result<ValidationReport, CommandError>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredFile {
    pub disk: String,
    pub path: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn put(
        &self,
        disk: &str,
        path: &str,
        contents: Vec<u8>,
    ) -> Result<StoredFile, CommandError>;

    async fn get(&self, disk: &str, path: &str) -> Result<Vec<u8>, CommandError>;

    async fn delete(&self, disk: &str, path: &str) -> Result<(), CommandError>;

    async fn exists(&self, disk: &str, path: &str) -> Result<bool, CommandError>;

    async fn url(&self, disk: &str, path: &str) -> Result<String, CommandError>;
}

#[async_trait]
pub trait CachePort: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Value>, CommandError>;

    async fn put(&self, key: &str, value: Value, ttl: Option<Duration>)
        -> Result<(), CommandError>;

    async fn forget(&self, key: &str) -> Result<(), CommandError>;

    async fn clear(&self, prefix: Option<&str>) -> Result<(), CommandError>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueJob {
    pub name: String,
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_seconds: Option<u64>,
}

#[async_trait]
pub trait QueuePort: Send + Sync {
    async fn dispatch(&self, job: QueueJob) -> Result<(), CommandError>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    pub name: String,
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[async_trait]
pub trait EventPort: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<(), CommandError>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomRule {
    pub name: String,
    #[serde(default)]
    pub args: Value,
}
