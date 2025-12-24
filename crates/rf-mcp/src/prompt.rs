//! MCP Prompt templates

use crate::errors::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

static PROMPT_REGISTRY: OnceLock<Arc<PromptRegistry>> = OnceLock::new();

/// Prompt message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    User,
    Assistant,
    System,
}

/// A prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: PromptContent,
}

/// Prompt content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PromptContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { uri: String },
}

impl PromptMessage {
    /// Create a user message
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: PromptRole::User,
            content: PromptContent::Text { text: text.into() },
        }
    }

    /// Create an assistant message
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: PromptRole::Assistant,
            content: PromptContent::Text { text: text.into() },
        }
    }

    /// Create a system message
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: PromptRole::System,
            content: PromptContent::Text { text: text.into() },
        }
    }
}

/// A registered prompt
#[derive(Clone)]
pub struct Prompt {
    /// Prompt name
    pub name: String,
    /// Prompt description
    pub description: Option<String>,
    /// Template arguments
    pub arguments: Vec<PromptArgument>,
    /// Message templates
    messages: Vec<PromptMessageTemplate>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

/// Prompt message template
#[derive(Debug, Clone)]
struct PromptMessageTemplate {
    role: PromptRole,
    template: String,
}

impl Prompt {
    /// Render the prompt with arguments
    pub fn render(&self, args: &HashMap<String, String>) -> McpResult<Vec<PromptMessage>> {
        // Validate required arguments
        for arg in &self.arguments {
            if arg.required && !args.contains_key(&arg.name) {
                return Err(McpError::InvalidInput(format!(
                    "Missing required argument: {}",
                    arg.name
                )));
            }
        }

        // Render messages
        let messages = self
            .messages
            .iter()
            .map(|m| {
                let mut text = m.template.clone();
                for (key, value) in args {
                    text = text.replace(&format!("{{{{{}}}}}", key), value);
                }
                PromptMessage {
                    role: m.role.clone(),
                    content: PromptContent::Text { text },
                }
            })
            .collect();

        Ok(messages)
    }
}

/// Prompt builder
pub struct PromptBuilder {
    name: String,
    description: Option<String>,
    arguments: Vec<PromptArgument>,
    messages: Vec<PromptMessageTemplate>,
}

impl PromptBuilder {
    /// Create a new prompt builder
    pub fn new(name: &str, template: &str) -> Self {
        // Extract arguments from template (e.g., {{content}})
        let re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
        let arguments: Vec<PromptArgument> = re
            .captures_iter(template)
            .map(|cap| PromptArgument {
                name: cap[1].to_string(),
                description: None,
                required: true,
            })
            .collect();

        // Deduplicate arguments
        let mut seen = std::collections::HashSet::new();
        let arguments: Vec<_> = arguments
            .into_iter()
            .filter(|a| seen.insert(a.name.clone()))
            .collect();

        Self {
            name: name.to_string(),
            description: None,
            arguments,
            messages: vec![PromptMessageTemplate {
                role: PromptRole::User,
                template: template.to_string(),
            }],
        }
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Add an argument description
    pub fn arg_description(mut self, name: &str, description: &str) -> Self {
        if let Some(arg) = self.arguments.iter_mut().find(|a| a.name == name) {
            arg.description = Some(description.to_string());
        }
        self
    }

    /// Make an argument optional
    pub fn optional(mut self, name: &str) -> Self {
        if let Some(arg) = self.arguments.iter_mut().find(|a| a.name == name) {
            arg.required = false;
        }
        self
    }

    /// Add a system message
    pub fn with_system(mut self, template: &str) -> Self {
        self.messages.insert(
            0,
            PromptMessageTemplate {
                role: PromptRole::System,
                template: template.to_string(),
            },
        );
        self
    }

    /// Add an assistant message
    pub fn with_assistant(mut self, template: &str) -> Self {
        self.messages.push(PromptMessageTemplate {
            role: PromptRole::Assistant,
            template: template.to_string(),
        });
        self
    }

    /// Register the prompt
    pub fn register(self) {
        let prompt = Prompt {
            name: self.name.clone(),
            description: self.description,
            arguments: self.arguments,
            messages: self.messages,
        };

        PromptRegistry::global().register(prompt);
    }
}

/// Prompt registry
pub struct PromptRegistry {
    prompts: RwLock<HashMap<String, Prompt>>,
}

impl PromptRegistry {
    /// Create a new prompt registry
    pub fn new() -> Self {
        Self {
            prompts: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global prompt registry
    pub fn global() -> Arc<Self> {
        PROMPT_REGISTRY
            .get_or_init(|| Arc::new(Self::new()))
            .clone()
    }

    /// Register a prompt
    pub fn register(&self, prompt: Prompt) {
        let mut prompts = self.prompts.write().unwrap();
        prompts.insert(prompt.name.clone(), prompt);
    }

    /// Get a prompt by name
    pub fn get(&self, name: &str) -> Option<Prompt> {
        let prompts = self.prompts.read().unwrap();
        prompts.get(name).cloned()
    }

    /// List all prompts
    pub fn list(&self) -> Vec<Prompt> {
        let prompts = self.prompts.read().unwrap();
        prompts.values().cloned().collect()
    }

    /// Render a prompt
    pub fn render(&self, name: &str, args: &HashMap<String, String>) -> McpResult<Vec<PromptMessage>> {
        let prompt = self
            .get(name)
            .ok_or_else(|| McpError::PromptNotFound(name.to_string()))?;
        prompt.render(args)
    }
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}
