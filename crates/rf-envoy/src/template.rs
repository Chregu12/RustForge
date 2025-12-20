//! Template engine for variable substitution

use crate::{EnvoyError, EnvoyResult};
use regex::Regex;
use std::collections::HashMap;

/// Template engine for variable substitution in commands
pub struct TemplateEngine {
    variables: HashMap<String, String>,
}

impl TemplateEngine {
    pub fn new(variables: HashMap<String, String>) -> Self {
        Self { variables }
    }

    /// Render a template string with variable substitution
    pub fn render(&self, template: &str) -> EnvoyResult<String> {
        let mut result = template.to_string();

        // Replace {{ $variable }} style (Blade-like)
        let blade_regex = Regex::new(r"\{\{\s*\$(\w+)\s*\}\}").unwrap();
        result = blade_regex
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = &caps[1];
                self.variables
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| format!("{{{{ ${} }}}}", var_name))
            })
            .to_string();

        // Replace ${variable} style (shell-like)
        let shell_regex = Regex::new(r"\$\{(\w+)\}").unwrap();
        result = shell_regex
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = &caps[1];
                self.variables
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| format!("${{{}}}", var_name))
            })
            .to_string();

        // Replace $variable style (simple)
        let simple_regex = Regex::new(r"\$(\w+)").unwrap();
        result = simple_regex
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = &caps[1];
                self.variables
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| format!("${}", var_name))
            })
            .to_string();

        Ok(result)
    }

    /// Add a variable
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(name.into(), value.into());
    }

    /// Get a variable
    pub fn get(&self, name: &str) -> Option<&String> {
        self.variables.get(name)
    }

    /// Check if a variable exists
    pub fn has(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }

    /// Load environment variables into the template engine
    pub fn load_env(&mut self) {
        for (key, value) in std::env::vars() {
            self.variables.insert(key, value);
        }
    }

    /// Load variables from a .env file
    pub fn load_dotenv(&mut self, path: &str) -> EnvoyResult<()> {
        let content = std::fs::read_to_string(path)?;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                self.variables.insert(key.to_string(), value.to_string());
            }
        }

        Ok(())
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

/// Template builder for complex command generation
pub struct CommandTemplate {
    parts: Vec<String>,
    variables: HashMap<String, String>,
}

impl CommandTemplate {
    pub fn new() -> Self {
        Self {
            parts: Vec::new(),
            variables: HashMap::new(),
        }
    }

    /// Add a literal string
    pub fn literal(mut self, s: impl Into<String>) -> Self {
        self.parts.push(s.into());
        self
    }

    /// Add a variable reference
    pub fn var(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.parts.push(format!("${{{}}}", name));
        self
    }

    /// Set a variable value
    pub fn set(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Build the command string
    pub fn build(&self) -> EnvoyResult<String> {
        let template = self.parts.join("");
        let engine = TemplateEngine::new(self.variables.clone());
        engine.render(&template)
    }
}

impl Default for CommandTemplate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blade_style_variables() {
        let mut vars = HashMap::new();
        vars.insert("branch".to_string(), "main".to_string());
        vars.insert("app_dir".to_string(), "/var/www".to_string());

        let engine = TemplateEngine::new(vars);

        let result = engine
            .render("cd {{ $app_dir }} && git pull origin {{ $branch }}")
            .unwrap();

        assert_eq!(result, "cd /var/www && git pull origin main");
    }

    #[test]
    fn test_shell_style_variables() {
        let mut vars = HashMap::new();
        vars.insert("branch".to_string(), "develop".to_string());

        let engine = TemplateEngine::new(vars);

        let result = engine.render("git checkout ${branch}").unwrap();

        assert_eq!(result, "git checkout develop");
    }

    #[test]
    fn test_simple_variables() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "myapp".to_string());

        let engine = TemplateEngine::new(vars);

        let result = engine.render("systemctl restart $name").unwrap();

        assert_eq!(result, "systemctl restart myapp");
    }

    #[test]
    fn test_undefined_variable() {
        let engine = TemplateEngine::new(HashMap::new());

        let result = engine.render("echo {{ $undefined }}").unwrap();

        assert!(result.contains("$undefined"));
    }

    #[test]
    fn test_command_template() {
        let cmd = CommandTemplate::new()
            .literal("cd ")
            .var("app_dir")
            .literal(" && git pull origin ")
            .var("branch")
            .set("app_dir", "/var/www")
            .set("branch", "main")
            .build()
            .unwrap();

        assert_eq!(cmd, "cd /var/www && git pull origin main");
    }
}
