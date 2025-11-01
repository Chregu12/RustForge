use crate::invocation::{CommandInvoker, FoundryInvoker};
use crate::InvocationRequest;
use anyhow::Result;
use foundry_application::ApplicationError;
use foundry_plugins::{CommandResult, CommandStatus, ExecutionOptions, ResponseFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::pending;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpCommandCall {
    pub command: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default = "default_format")]
    pub format: ResponseFormat,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub options: ExecutionOptions,
}

impl McpCommandCall {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            arguments: Vec::new(),
            format: ResponseFormat::Json,
            correlation_id: None,
            options: ExecutionOptions::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpCommandResponse {
    pub command: String,
    pub status: CommandStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub correlation_id: Option<String>,
}

impl McpCommandResponse {
    pub fn from_result(call: &McpCommandCall, result: CommandResult) -> Self {
        Self {
            command: call.command.clone(),
            status: result.status,
            message: result.message,
            data: result.data.unwrap_or(Value::Null),
            correlation_id: call.correlation_id.clone(),
        }
    }

    pub fn error(call: &McpCommandCall, message: impl Into<String>) -> Self {
        Self {
            command: call.command.clone(),
            status: CommandStatus::Failure,
            message: Some(message.into()),
            data: Value::Null,
            correlation_id: call.correlation_id.clone(),
        }
    }
}

fn default_format() -> ResponseFormat {
    ResponseFormat::Json
}

pub struct McpServer {
    invoker: FoundryInvoker,
}

impl McpServer {
    pub fn new(invoker: FoundryInvoker) -> Self {
        Self { invoker }
    }

    pub async fn run_stdio(self) -> Result<()> {
        let mut reader = BufReader::new(io::stdin());
        let mut stdout = io::stdout();
        let mut buffer = String::new();

        info!("MCP STDIO Server gestartet");

        loop {
            buffer.clear();
            tokio::select! {
                read = reader.read_line(&mut buffer) => {
                    let read = read?;
                    if read == 0 {
                        break;
                    }
                }
                _ = ctrl_c_or_pending() => {
                    info!("MCP STDIO Shutdown-Signal empfangen");
                    break;
                }
            }

            let line = buffer.trim();
            if line.is_empty() {
                continue;
            }

            let call: McpCommandCall = match serde_json::from_str(line) {
                Ok(call) => call,
                Err(err) => {
                    warn!(error = %err, "Konnte MCP Anfrage nicht parsen");
                    continue;
                }
            };

            let mut request = InvocationRequest::new(call.command.clone())
                .with_args(call.arguments.clone())
                .with_format(call.format.clone())
                .with_options(call.options);

            if let Some(id) = &call.correlation_id {
                request = request.with_correlation_id(id.clone());
            }

            let response = match self.invoker.invoke(request).await {
                Ok(result) => McpCommandResponse::from_result(&call, result),
                Err(err) => McpCommandResponse::error(&call, format_application_error(err)),
            };

            let payload = serde_json::to_string(&response)?;
            stdout.write_all(payload.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        info!("MCP STDIO Server gestoppt");
        Ok(())
    }
}

fn format_application_error(err: ApplicationError) -> String {
    match err {
        ApplicationError::CommandNotFound(command) => {
            format!("Command `{command}` wurde nicht gefunden")
        }
        ApplicationError::CommandAlreadyRegistered(command) => {
            format!("Command `{command}` ist bereits registriert")
        }
        ApplicationError::CommandExecution(inner) => inner.to_string(),
        ApplicationError::StorageError(message) => format!("Storage Error: {message}"),
    }
}

async fn ctrl_c_or_pending() {
    if tokio::signal::ctrl_c().await.is_err() {
        pending::<()>().await;
    }
}
