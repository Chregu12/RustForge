use anyhow::Result;
use chrono::{DateTime, Utc};
use foundry_plugins::{CommandResult, CommandStatus, ExecutionOptions, ResponseFormat};
use serde::Serialize;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AuditOutcome {
    Success {
        status: CommandStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
    },
    Error {
        error: String,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditRecord {
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub args: Vec<String>,
    pub format: ResponseFormat,
    pub options: ExecutionOptions,
    #[serde(flatten)]
    pub outcome: AuditOutcome,
}

impl AuditRecord {
    pub fn from_success(
        command: impl Into<String>,
        args: Vec<String>,
        format: ResponseFormat,
        options: ExecutionOptions,
        result: &CommandResult,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            command: command.into(),
            args,
            format,
            options,
            outcome: AuditOutcome::Success {
                status: result.status.clone(),
                message: result.message.clone(),
                data: result.data.clone(),
            },
        }
    }

    pub fn from_error(
        command: impl Into<String>,
        args: Vec<String>,
        format: ResponseFormat,
        options: ExecutionOptions,
        error: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            command: command.into(),
            args,
            format,
            options,
            outcome: AuditOutcome::Error {
                error: error.into(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct JsonlAuditLogger {
    path: PathBuf,
}

impl JsonlAuditLogger {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn log(&self, record: &AuditRecord) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let serialized = serde_json::to_string(record)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{serialized}")?;
        Ok(())
    }
}
