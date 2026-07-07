#![allow(dead_code)] // utility helpers reserved for CLI commands, not all consumed yet
//! Interactive prompts for the CLI
//!
//! This module provides rich, user-friendly interactive prompts for generating
//! code scaffolding, with smart defaults and validation.

use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};

/// Configuration for creating a new model
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub create_migration: bool,
    pub create_factory: bool,
    pub create_seeder: bool,
    pub add_timestamps: bool,
    pub add_soft_deletes: bool,
}

/// Configuration for creating a new controller
#[derive(Debug, Clone)]
pub struct ControllerConfig {
    pub name: String,
    pub controller_type: ControllerType,
    pub create_routes: bool,
    pub add_to_resource_routes: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControllerType {
    Resource,
    Api,
    Invokable,
    Plain,
}

impl std::fmt::Display for ControllerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControllerType::Resource => write!(f, "Resource (CRUD)"),
            ControllerType::Api => write!(f, "API (JSON)"),
            ControllerType::Invokable => write!(f, "Invokable (Single action)"),
            ControllerType::Plain => write!(f, "Plain"),
        }
    }
}

/// Configuration for creating a migration
#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub name: String,
    pub table_name: Option<String>,
    pub migration_type: MigrationType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationType {
    Create,
    Modify,
    Drop,
    Custom,
}

impl std::fmt::Display for MigrationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationType::Create => write!(f, "Create new table"),
            MigrationType::Modify => write!(f, "Modify existing table"),
            MigrationType::Drop => write!(f, "Drop table"),
            MigrationType::Custom => write!(f, "Custom migration"),
        }
    }
}

/// Print a section header with formatting
pub fn print_section_header(title: &str) {
    let border = "─".repeat(title.len() + 4);
    println!();
    println!("┌{}┐", border);
    println!("│  {}  │", style(title).bold().cyan());
    println!("└{}┘", border);
    println!();
}

/// Print a success checkmark with message
pub fn print_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{} {}", style("→").blue().bold(), message);
}

/// Print a warning message
pub fn print_warning(message: &str) {
    println!("{} {}", style("⚠").yellow().bold(), message);
}

/// Print next steps
pub fn print_next_steps(steps: &[&str]) {
    println!();
    println!("{}", style("Next steps:").bold().cyan());
    for (i, step) in steps.iter().enumerate() {
        println!("  {}. {}", i + 1, step);
    }
    println!();
}

/// Prompt for model configuration interactively
pub fn prompt_model_config() -> Result<ModelConfig> {
    print_section_header("Create a new Eloquent Model");

    let theme = ColorfulTheme::default();

    // Model name
    let name: String = Input::with_theme(&theme)
        .with_prompt("Model name")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                Err("Model name cannot be empty")
            } else if !input.chars().next().unwrap().is_uppercase() {
                Err("Model name should start with an uppercase letter (e.g., User, BlogPost)")
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    // Migration
    let create_migration = Confirm::with_theme(&theme)
        .with_prompt("Create migration?")
        .default(true)
        .interact()?;

    // Factory
    let create_factory = Confirm::with_theme(&theme)
        .with_prompt("Create factory?")
        .default(true)
        .interact()?;

    // Seeder
    let create_seeder = Confirm::with_theme(&theme)
        .with_prompt("Create seeder?")
        .default(false)
        .interact()?;

    // Timestamps
    let add_timestamps = Confirm::with_theme(&theme)
        .with_prompt("Add timestamps?")
        .default(true)
        .interact()?;

    // Soft deletes
    let add_soft_deletes = Confirm::with_theme(&theme)
        .with_prompt("Add soft deletes?")
        .default(false)
        .interact()?;

    Ok(ModelConfig {
        name,
        create_migration,
        create_factory,
        create_seeder,
        add_timestamps,
        add_soft_deletes,
    })
}

/// Prompt for controller configuration interactively
pub fn prompt_controller_config() -> Result<ControllerConfig> {
    print_section_header("Create a new Controller");

    let theme = ColorfulTheme::default();

    // Controller name
    let name: String = Input::with_theme(&theme)
        .with_prompt("Controller name")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.trim().is_empty() {
                Err("Controller name cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    // Controller type
    let types = vec![
        ControllerType::Resource,
        ControllerType::Api,
        ControllerType::Invokable,
        ControllerType::Plain,
    ];

    let type_index = Select::with_theme(&theme)
        .with_prompt("Type")
        .items(&types)
        .default(0)
        .interact()?;

    let controller_type = types[type_index].clone();

    // Only ask about routes for Resource and API controllers
    let (create_routes, add_to_resource_routes) = match controller_type {
        ControllerType::Resource | ControllerType::Api => {
            let create = Confirm::with_theme(&theme)
                .with_prompt("Create route?")
                .default(true)
                .interact()?;

            let add_resource = if create {
                Confirm::with_theme(&theme)
                    .with_prompt("Add to resource routes?")
                    .default(true)
                    .interact()?
            } else {
                false
            };

            (create, add_resource)
        }
        _ => (false, false),
    };

    Ok(ControllerConfig {
        name,
        controller_type,
        create_routes,
        add_to_resource_routes,
    })
}

/// Prompt for migration configuration interactively
pub fn prompt_migration_config() -> Result<MigrationConfig> {
    print_section_header("Create a new Migration");

    let theme = ColorfulTheme::default();

    // Migration type
    let types = vec![
        MigrationType::Create,
        MigrationType::Modify,
        MigrationType::Drop,
        MigrationType::Custom,
    ];

    let type_index = Select::with_theme(&theme)
        .with_prompt("Migration type")
        .items(&types)
        .default(0)
        .interact()?;

    let migration_type = types[type_index].clone();

    // Table name (for create/modify/drop)
    let table_name = match migration_type {
        MigrationType::Create | MigrationType::Modify | MigrationType::Drop => {
            let name: String = Input::with_theme(&theme)
                .with_prompt("Table name (plural)")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if input.trim().is_empty() {
                        Err("Table name cannot be empty")
                    } else if input.chars().any(|c| c.is_uppercase()) {
                        Err("Table name should be lowercase (e.g., users, blog_posts)")
                    } else {
                        Ok(())
                    }
                })
                .interact_text()?;
            Some(name)
        }
        MigrationType::Custom => None,
    };

    // Migration name
    let name = match migration_type {
        MigrationType::Create => {
            format!("create_{}_table", table_name.as_ref().unwrap())
        }
        MigrationType::Modify => {
            let action: String = Input::with_theme(&theme)
                .with_prompt("What are you modifying? (e.g., add_email_to_users)")
                .interact_text()?;
            action
        }
        MigrationType::Drop => {
            format!("drop_{}_table", table_name.as_ref().unwrap())
        }
        MigrationType::Custom => Input::with_theme(&theme)
            .with_prompt("Migration name")
            .interact_text()?,
    };

    Ok(MigrationConfig {
        name,
        table_name,
        migration_type,
    })
}

/// Prompt for confirmation with a custom message
pub fn confirm(message: &str, default: bool) -> Result<bool> {
    let theme = ColorfulTheme::default();
    Confirm::with_theme(&theme)
        .with_prompt(message)
        .default(default)
        .interact()
        .map_err(Into::into)
}

/// Prompt for text input with validation
pub fn prompt_text(prompt: &str, default: Option<&str>) -> Result<String> {
    let theme = ColorfulTheme::default();
    let mut input = Input::with_theme(&theme).with_prompt(prompt);

    if let Some(def) = default {
        input = input.default(def.to_string());
    }

    input.interact_text().map_err(Into::into)
}

/// Prompt for selecting from a list of options
pub fn prompt_select<T: std::fmt::Display>(
    prompt: &str,
    items: &[T],
    default: usize,
) -> Result<usize> {
    let theme = ColorfulTheme::default();
    Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(items)
        .default(default)
        .interact()
        .map_err(Into::into)
}

/// Prompt for multiple selections
pub fn prompt_multiselect<T: std::fmt::Display>(
    prompt: &str,
    items: &[T],
    defaults: &[bool],
) -> Result<Vec<usize>> {
    let theme = ColorfulTheme::default();
    let mut multi = MultiSelect::with_theme(&theme)
        .with_prompt(prompt)
        .items(items);

    if !defaults.is_empty() {
        multi = multi.defaults(defaults);
    }

    multi.interact().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_type_display() {
        assert_eq!(ControllerType::Resource.to_string(), "Resource (CRUD)");
        assert_eq!(ControllerType::Api.to_string(), "API (JSON)");
        assert_eq!(
            ControllerType::Invokable.to_string(),
            "Invokable (Single action)"
        );
        assert_eq!(ControllerType::Plain.to_string(), "Plain");
    }

    #[test]
    fn test_migration_type_display() {
        assert_eq!(MigrationType::Create.to_string(), "Create new table");
        assert_eq!(MigrationType::Modify.to_string(), "Modify existing table");
        assert_eq!(MigrationType::Drop.to_string(), "Drop table");
        assert_eq!(MigrationType::Custom.to_string(), "Custom migration");
    }

    #[test]
    fn test_model_config_creation() {
        let config = ModelConfig {
            name: "User".to_string(),
            create_migration: true,
            create_factory: true,
            create_seeder: false,
            add_timestamps: true,
            add_soft_deletes: false,
        };

        assert_eq!(config.name, "User");
        assert!(config.create_migration);
        assert!(config.create_factory);
        assert!(!config.create_seeder);
        assert!(config.add_timestamps);
        assert!(!config.add_soft_deletes);
    }

    #[test]
    fn test_controller_config_creation() {
        let config = ControllerConfig {
            name: "UserController".to_string(),
            controller_type: ControllerType::Resource,
            create_routes: true,
            add_to_resource_routes: true,
        };

        assert_eq!(config.name, "UserController");
        assert_eq!(config.controller_type, ControllerType::Resource);
        assert!(config.create_routes);
        assert!(config.add_to_resource_routes);
    }

    #[test]
    fn test_migration_config_creation() {
        let config = MigrationConfig {
            name: "create_users_table".to_string(),
            table_name: Some("users".to_string()),
            migration_type: MigrationType::Create,
        };

        assert_eq!(config.name, "create_users_table");
        assert_eq!(config.table_name, Some("users".to_string()));
        assert_eq!(config.migration_type, MigrationType::Create);
    }
}
