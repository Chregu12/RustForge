use async_trait::async_trait;
use foundry_application::{ApplicationError, FoundryApp};
use foundry_domain::CommandDescriptor;
use foundry_plugins::{CommandResult, ExecutionOptions, ResponseFormat};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvocationRequest {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_format")]
    pub format: ResponseFormat,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub options: ExecutionOptions,
}

impl InvocationRequest {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            format: ResponseFormat::Human,
            correlation_id: None,
            options: ExecutionOptions::default(),
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_format(mut self, format: ResponseFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_options(mut self, options: ExecutionOptions) -> Self {
        self.options = options;
        self
    }
}

fn default_format() -> ResponseFormat {
    ResponseFormat::Human
}

#[async_trait]
pub trait CommandInvoker: Send + Sync {
    async fn invoke(&self, request: InvocationRequest) -> Result<CommandResult, ApplicationError>;
}

#[derive(Clone)]
pub struct FoundryInvoker {
    app: FoundryApp,
}

impl FoundryInvoker {
    pub fn new(app: FoundryApp) -> Self {
        Self { app }
    }

    pub fn descriptors(&self) -> Vec<CommandDescriptor> {
        self.app.registry().descriptors()
    }

    pub fn app(&self) -> &FoundryApp {
        &self.app
    }
}

#[async_trait]
impl CommandInvoker for FoundryInvoker {
    #[instrument(name = "foundry.invoke", skip(self), fields(command = %request.command))]
    async fn invoke(&self, request: InvocationRequest) -> Result<CommandResult, ApplicationError> {
        self.app
            .dispatch(
                &request.command,
                request.args.clone(),
                request.format.clone(),
                request.options,
            )
            .await
    }
}
