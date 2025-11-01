//! Domain Layer für Foundry Core: zentrale Value Objects & Policies.

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        // Placeholder für Validierung (Slug, keine Leerzeichen etc.).
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    #[default]
    Core,
    Generator,
    Database,
    Runtime,
    Utility,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub id: CommandId,
    pub name: String,
    pub summary: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: CommandKind,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl CommandDescriptor {
    pub fn builder(id: impl Into<String>, name: impl Into<String>) -> CommandDescriptorBuilder {
        CommandDescriptorBuilder::new(id, name)
    }
}

pub struct CommandDescriptorBuilder {
    descriptor: CommandDescriptor,
}

impl CommandDescriptorBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            descriptor: CommandDescriptor {
                id: CommandId::new(id),
                name: name.into(),
                summary: String::new(),
                description: None,
                category: CommandKind::default(),
                aliases: Vec::new(),
            },
        }
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.descriptor.summary = summary.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.descriptor.description = Some(description.into());
        self
    }

    pub fn category(mut self, category: CommandKind) -> Self {
        self.descriptor.category = category;
        self
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.descriptor.aliases.push(alias.into());
        self
    }

    pub fn build(self) -> CommandDescriptor {
        self.descriptor
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("invalid command id: {0}")]
    InvalidCommandId(String),
    #[error("command not found: {0}")]
    CommandNotFound(String),
}
