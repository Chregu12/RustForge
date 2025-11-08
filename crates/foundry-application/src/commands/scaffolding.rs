use async_trait::async_trait;
use chrono::Utc;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{
    AppError, CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand,
    FoundryGenerator, GeneratedArtifact, GeneratorPlan, ResponseFormat, ValidationRules,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use crate::commands::TinkerCommand;

pub struct MakeModelCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeMigrationCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeControllerCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeMiddlewareCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeSeederCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeRequestCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeJobCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeFactoryCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeEventCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeListenerCommand {
    descriptor: CommandDescriptor,
}

pub struct MakeCommandCommand {
    descriptor: CommandDescriptor,
}

impl MakeModelCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_model", "make:model")
                .summary("Plant die Erzeugung eines Domain-Modells inklusive Migration")
                .description(
                    "Generiert (dry-run) ein Domain-Modell mit passender Migration und zeigt die geplanten Artefakte.",
                )
                .category(CommandKind::Generator)
                .alias("make model")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:model")?;
        let slug = slugify(&name);
        let timestamp = current_timestamp();
        let domain_root = config_path(ctx, "FOUNDRY_DOMAIN_MODELS", "domain/models");
        let migration_root = config_path(ctx, "FOUNDRY_MIGRATIONS", "migrations");
        let migration_dir = format!("{migration_root}/{timestamp}_create_{slug}_table");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: format!("{domain_root}/{slug}.rs"),
                    description: format!("Domain-Modell `{name}` mit serde-Strukturen"),
                    preview: Some(model_template(&name)),
                },
                GeneratedArtifact {
                    path: format!("{migration_dir}/up.sql"),
                    description: format!("Migration: Tabelle `{slug}` erstellen"),
                    preview: Some(migration_up_sql(&slug)),
                },
                GeneratedArtifact {
                    path: format!("{migration_dir}/down.sql"),
                    description: format!("Rollback: Tabelle `{slug}` entfernen"),
                    preview: Some(migration_down_sql(&slug)),
                },
            ],
            summary: Some(format!(
                "Erzeugt Domain Model `{name}` inklusive Migration (dry-run)"
            )),
        };

        Ok((name, plan))
    }
}

impl Default for MakeModelCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeMigrationCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_migration", "make:migration")
                .summary("Plant die Erzeugung einer Datenbank-Migration")
                .description("Erzeugt (dry-run) eine neue Migration mit Template-Skeleton.")
                .category(CommandKind::Generator)
                .alias("make migration")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:migration")?;
        let slug = slugify(&name);
        let timestamp = current_timestamp();
        let migration_root = config_path(ctx, "FOUNDRY_MIGRATIONS", "migrations");
        let migration_dir = format!("{migration_root}/{timestamp}_{slug}");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: format!("{migration_dir}/up.sql"),
                    description: format!("Migration `{name}`"),
                    preview: Some(migration_up_sql(&slug)),
                },
                GeneratedArtifact {
                    path: format!("{migration_dir}/down.sql"),
                    description: format!("Rollback für `{name}`"),
                    preview: Some(migration_down_sql(&slug)),
                },
            ],
            summary: Some(format!("Legt Migration `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeMigrationCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeControllerCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_controller", "make:controller")
                .summary("Plant einen Axum REST-Controller inklusive Routing-Stubs")
                .description(
                    "Generiert (dry-run) Controller und Route-Stubs für einen Resource-Controller.",
                )
                .category(CommandKind::Generator)
                .alias("make controller")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:controller")?;
        let slug = slugify(&name);
        let controller_root = config_path(ctx, "FOUNDRY_HTTP_CONTROLLERS", "app/http/controllers");
        let routes_root = config_path(ctx, "FOUNDRY_HTTP_ROUTES", "app/http/routes");

        let controller_source = controller_template(&name);
        let routes_source = routes_template(&name, &slug);

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: format!("{controller_root}/{slug}_controller.rs"),
                    description: format!("Axum Controller `{name}` mit CRUD-Stubs"),
                    preview: Some(controller_source),
                },
                GeneratedArtifact {
                    path: format!("{routes_root}/{slug}.rs"),
                    description: "Route-Registrierung für den Controller".to_string(),
                    preview: Some(routes_source),
                },
            ],
            summary: Some(format!("Erzeugt Controller + Routing für `{name}`")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeControllerCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeMiddlewareCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_middleware", "make:middleware")
                .summary("Plant die Erzeugung eines HTTP-Middleware-Handlers")
                .description("Generiert (dry-run) eine Middleware-Funktion, die mit axum::middleware::from_fn kompatibel ist.")
                .category(CommandKind::Generator)
                .alias("make middleware")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:middleware")?;
        let slug = slugify(&name);
        let middleware_root = config_path(ctx, "FOUNDRY_HTTP_MIDDLEWARE", "app/http/middleware");

        let plan = GeneratorPlan {
            artifacts: vec![GeneratedArtifact {
                path: format!("{middleware_root}/{slug}.rs"),
                description: format!("HTTP Middleware `{name}`"),
                preview: Some(middleware_template(&name, &slug)),
            }],
            summary: Some(format!("Erzeugt Middleware `{name}`")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeMiddlewareCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeSeederCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_seeder", "make:seeder")
                .summary("Plant die Erzeugung eines Datenbank-Seeders")
                .description("Erzeugt (dry-run) einen neuen Seeder mit einer SQL-Vorlage.")
                .category(CommandKind::Generator)
                .alias("make seeder")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:seeder")?;
        let slug = slugify(&name);
        let timestamp = current_timestamp();
        let seeder_root = config_path(ctx, "FOUNDRY_SEEDS", "seeds");
        let seeder_path = format!("{seeder_root}/{timestamp}_{slug}.sql");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: seeder_path,
                    description: format!("Datenbank-Seeder `{name}`"),
                    preview: Some(seeder_template(&slug)),
                },
            ],
            summary: Some(format!("Legt Seeder `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeSeederCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeRequestCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_request", "make:request")
                .summary("Plant die Erzeugung einer HTTP-Anfrage-Klasse mit Validierung")
                .description("Erzeugt (dry-run) eine neue Anfrage-Klasse mit Validierungsregeln.")
                .category(CommandKind::Generator)
                .alias("make request")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:request")?;
        let slug = slugify(&name);
        let request_root = config_path(ctx, "FOUNDRY_HTTP_REQUESTS", "app/http/requests");
        let request_path = format!("{request_root}/{slug}_request.rs");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: request_path,
                    description: format!("Anfrage-Klasse `{name}`"),
                    preview: Some(request_template(&name)),
                },
            ],
            summary: Some(format!("Legt Anfrage-Klasse `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeRequestCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeJobCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_job", "make:job")
                .summary("Plant die Erzeugung einer asynchronen Hintergrund-Aufgabe")
                .description("Erzeugt (dry-run) einen neuen Job mit einer Vorlage für asynchrone Hintergrundaufgaben.")
                .category(CommandKind::Generator)
                .alias("make job")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:job")?;
        let slug = slugify(&name);
        let jobs_root = config_path(ctx, "FOUNDRY_JOBS", "app/jobs");
        let job_path = format!("{jobs_root}/{slug}_job.rs");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: job_path,
                    description: format!("Job `{name}` für asynchrone Hintergrundaufgaben"),
                    preview: Some(job_template(&name)),
                },
            ],
            summary: Some(format!("Legt Job `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeJobCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeFactoryCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_factory", "make:factory")
                .summary("Plant die Erzeugung einer Test-Daten-Factory")
                .description("Erzeugt (dry-run) eine neue Factory mit Faker-ähnlicher Struktur für Test-Daten.")
                .category(CommandKind::Generator)
                .alias("make factory")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:factory")?;
        let slug = slugify(&name);
        let factory_root = config_path(ctx, "FOUNDRY_FACTORIES", "app/factories");
        let factory_path = format!("{factory_root}/{slug}_factory.rs");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: factory_path,
                    description: format!("Factory `{name}` für Test-Daten-Generierung"),
                    preview: Some(factory_template(&name)),
                },
            ],
            summary: Some(format!("Legt Factory `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeFactoryCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeEventCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_event", "make:event")
                .summary("Plant die Erzeugung einer Event-Klasse")
                .description("Erzeugt (dry-run) eine neue Event-Klasse mit serialisierbarer Struktur und Payload-Feldern.")
                .category(CommandKind::Generator)
                .alias("make event")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:event")?;
        let slug = slugify(&name);
        let events_root = config_path(ctx, "FOUNDRY_EVENTS", "app/events");
        let event_path = format!("{events_root}/{slug}_event.rs");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: event_path,
                    description: format!("Event `{name}` mit serialisierbarer Payload"),
                    preview: Some(event_template(&name)),
                },
            ],
            summary: Some(format!("Legt Event `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeEventCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeListenerCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_listener", "make:listener")
                .summary("Plant die Erzeugung eines Event-Listeners")
                .description("Erzeugt (dry-run) einen neuen Event-Listener mit async handle() Methode zur Event-Verarbeitung.")
                .category(CommandKind::Generator)
                .alias("make listener")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:listener")?;
        let slug = slugify(&name);
        let listeners_root = config_path(ctx, "FOUNDRY_LISTENERS", "app/listeners");
        let listener_path = format!("{listeners_root}/{slug}_listener.rs");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: listener_path,
                    description: format!("Listener `{name}` mit async handle() Methode"),
                    preview: Some(listener_template(&name)),
                },
            ],
            summary: Some(format!("Legt Listener `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeListenerCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl MakeCommandCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_command", "make:command")
                .summary("Erstellt einen Custom Artisan-ähnlichen Command")
                .description("Erzeugt (dry-run) einen neuen Command mit async execute() Methode. Zeigt wie man eigene Commands baut.")
                .category(CommandKind::Generator)
                .alias("make command")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<(String, GeneratorPlan), CommandError> {
        let name = extract_primary_argument(ctx, "make:command")?;
        let slug = slugify(&name);
        let commands_root = config_path(ctx, "FOUNDRY_COMMANDS", "app/commands");
        let command_path = format!("{commands_root}/{slug}_command.rs");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: command_path,
                    description: format!("Command `{name}` mit async execute() Methode"),
                    preview: Some(command_template(&name)),
                },
            ],
            summary: Some(format!("Legt Command `{name}` an")),
        };

        Ok((name, plan))
    }
}

impl Default for MakeCommandCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FoundryCommand for MakeModelCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let input = serde_json::json!({
            "name": ctx.args.first().cloned().unwrap_or_default(),
        });
        let validation_rules = ValidationRules {
            rules: serde_json::json!({
                "required": ["name"],
                "fields": {
                    "name": {
                        "regex": "^[A-Za-z][A-Za-z0-9_]*$",
                        "min_length": 3,
                        "max_length": 40
                    }
                }
            }),
        };
        let validation = ctx.validate(input, validation_rules).await?;
        if !validation.valid {
            let mut error = AppError::new("VALIDATION_FAILED", "Model creation validation failed")
                .with_status(422);
            for violation in validation.errors {
                error = error.with_context(violation.field, violation.message);
            }
            return Ok(CommandResult::failure(error));
        }

        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_domain_model(&ctx, &slug)?;
        }
        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:model → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:model → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:model for {name}")
                } else {
                    format!("generated make:model for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeModelCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeMigrationCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:migration → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:migration → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:migration for {name}")
                } else {
                    format!("generated make:migration for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeMigrationCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeControllerCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_controller_modules(&ctx, &slug)?;
        }
        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:controller → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:controller → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:controller for {name}")
                } else {
                    format!("generated make:controller for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeControllerCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeMiddlewareCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_middleware_modules(&ctx, &slug)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:middleware → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:middleware → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:middleware for {name}")
                } else {
                    format!("generated make:middleware for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeMiddlewareCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeSeederCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:seeder → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:seeder → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:seeder for {name}")
                } else {
                    format!("generated make:seeder for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeSeederCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeRequestCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_request_modules(&ctx, &slug)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:request → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:request → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:request for {name}")
                } else {
                    format!("generated make:request for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeRequestCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeJobCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_job_modules(&ctx, &slug)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:job → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:job → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:job for {name}")
                } else {
                    format!("generated make:job for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeJobCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeFactoryCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_factory_modules(&ctx, &slug)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:factory → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:factory → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:factory for {name}")
                } else {
                    format!("generated make:factory for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeFactoryCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeEventCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_event_modules(&ctx, &slug)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:event → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:event → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:event for {name}")
                } else {
                    format!("generated make:event for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeEventCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeListenerCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_listener_modules(&ctx, &slug)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:listener → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:listener → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:listener for {name}")
                } else {
                    format!("generated make:listener for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeListenerCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

#[async_trait]
impl FoundryCommand for MakeCommandCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let (name, plan) = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;
        let slug = slugify(&name);
        if !ctx.options.dry_run {
            register_command_modules(&ctx, &slug)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:command → {total} Artefakte für `{name}` geplant (dry-run).")
                } else {
                    format!("make:command → {total} Artefakte für `{name}` erzeugt.")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    format!("planned make:command for {name}")
                } else {
                    format!("generated make:command for {name}")
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "name": name,
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeCommandCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let (_, plan) = self.compute_plan(ctx)?;
        Ok(plan)
    }
}

fn extract_primary_argument(ctx: &CommandContext, command: &str) -> Result<String, CommandError> {
    ctx.args
        .first()
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            CommandError::Message(format!(
                "`{command}` benötigt einen Namen als erstes Argument"
            ))
        })
}

fn config_path(ctx: &CommandContext, key: &str, fallback: &str) -> String {
    ctx.config
        .as_object()
        .and_then(|map| map.get(key))
        .and_then(|value| value.as_str())
        .map(|value| value.trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;

    for (idx, ch) in value.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if idx > 0 && !last_was_separator && !slug.ends_with('_') {
                    slug.push('_');
                }
                slug.push(ch.to_ascii_lowercase());
            } else {
                slug.push(ch);
            }
            last_was_separator = false;
        } else if matches!(ch, '-' | ' ' | '/' | ':')
            && !last_was_separator && !slug.is_empty() {
                slug.push('_');
                last_was_separator = true;
            }
    }

    if slug.is_empty() {
        value.to_lowercase()
    } else {
        slug.trim_matches('_').to_string()
    }
}

fn current_timestamp() -> String {
    Utc::now().format("%Y%m%d%H%M%S").to_string()
}

fn model_template(struct_name: &str) -> String {
    format!(
        "use serde::{{Deserialize, Serialize}};\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {struct_name} {{\n    pub id: i64,\n    // TODO: weitere Felder hinzufügen\n}}\n"
    )
}

fn migration_up_sql(table: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {table} (\n    id INTEGER PRIMARY KEY AUTOINCREMENT,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP\n);\n"
    )
}

fn migration_down_sql(table: &str) -> String {
    format!("DROP TABLE IF EXISTS {table};\n")
}

fn seeder_template(table: &str) -> String {
    format!("-- Seeder für Tabelle: {table}\n-- INSERT INTO {table} (column) VALUES (\'value\');\n")
}

fn request_template(name: &str) -> String {
    format!(
        "use foundry_plugins::ValidationRules;\nuse serde::Deserialize;\n\n#[derive(Debug, Deserialize)]\npub struct {name} {{\n    // TODO: Anfrage-Payload-Felder hier definieren\n}}\n\nimpl {name} {{\n    pub fn rules() -> ValidationRules {{\n        ValidationRules {{\n            rules: serde_json::json!({{\n                \"required\": [\"field_name\"],\n                \"fields\": {{\n                    \"field_name\": {{ \"min_length\": 3 }}\n                }}\n            }}),\n        }}\n    }}\n}}\n"
    )
}

fn job_template(name: &str) -> String {
    format!(
        "use foundry_plugins::CommandContext;\nuse serde::{{Deserialize, Serialize}};\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {name}Job {{\n    // TODO: Job-Payload-Felder hier definieren\n}}\n\nimpl {name}Job {{\n    pub async fn handle(&self, _ctx: &CommandContext) -> Result<(), String> {{\n        // TODO: Implementiere die asynchrone Job-Logik hier\n        // Beispiel: E-Mail versenden, Datei verarbeiten, API-Anfrage, etc.\n        println!(\"Executing {name}Job...\");\n        Ok(())\n    }}\n}}\n"
    )
}

fn event_template(name: &str) -> String {
    format!(
        "use serde::{{Deserialize, Serialize}};\nuse chrono::{{DateTime, Utc}};\n\n/// Event: {name}\n///\n/// Dieses Event wird ausgelöst, wenn ein bestimmtes Ereignis im System eintritt.\n/// Es kann von mehreren Listenern verarbeitet werden, um verschiedene\n/// Nebeneffekte (z.B. E-Mail versenden, Logging, Benachrichtigungen) auszulösen.\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {name}Event {{\n    /// Eindeutige Event-ID\n    pub event_id: String,\n    \n    /// Zeitpunkt des Event-Auftretens\n    pub occurred_at: DateTime<Utc>,\n    \n    /// Event-spezifische Payload-Daten\n    pub payload: {name}Payload,\n}}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {name}Payload {{\n    // TODO: Füge hier die Event-spezifischen Felder hinzu\n    // Beispiel:\n    // pub user_id: i64,\n    // pub action: String,\n    // pub metadata: serde_json::Value,\n}}\n\nimpl {name}Event {{\n    /// Erstellt eine neue Event-Instanz\n    pub fn new(payload: {name}Payload) -> Self {{\n        Self {{\n            event_id: uuid::Uuid::new_v4().to_string(),\n            occurred_at: Utc::now(),\n            payload,\n        }}\n    }}\n    \n    /// Gibt den Event-Namen zurück (für Event-Bus-Routing)\n    pub fn event_name() -> &'static str {{\n        \"{name}Event\"\n    }}\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n    #[test]\n    fn creates_event_with_unique_id() {{\n        let payload = {name}Payload {{}};\n        let event1 = {name}Event::new(payload.clone());\n        let event2 = {name}Event::new(payload);\n        \n        assert_ne!(event1.event_id, event2.event_id);\n    }}\n    \n    #[test]\n    fn event_name_is_correct() {{\n        assert_eq!({name}Event::event_name(), \"{name}Event\");\n    }}\n}}\n"
    )
}

fn listener_template(name: &str) -> String {
    format!(
        "use async_trait::async_trait;\nuse serde::{{Deserialize, Serialize}};\n\n/// Listener: {name}\n///\n/// Dieser Listener reagiert auf Events und führt spezifische Aktionen aus.\n/// Implementiere die `handle()` Methode, um die Event-Verarbeitungslogik zu definieren.\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {name}Listener;\n\n#[async_trait]\npub trait EventListener<E> {{\n    async fn handle(&self, event: &E) -> Result<(), ListenerError>;\n}}\n\n#[derive(Debug, thiserror::Error)]\npub enum ListenerError {{\n    #[error(\"Listener execution failed: {{0}}\")]\n    ExecutionFailed(String),\n    \n    #[error(\"Event processing error: {{0}}\")]\n    ProcessingError(String),\n}}\n\nimpl {name}Listener {{\n    pub fn new() -> Self {{\n        Self\n    }}\n}}\n\nimpl Default for {name}Listener {{\n    fn default() -> Self {{\n        Self::new()\n    }}\n}}\n\n// TODO: Implementiere EventListener für dein spezifisches Event\n// Beispiel:\n// #[async_trait]\n// impl EventListener<OrderPlacedEvent> for {name}Listener {{\n//     async fn handle(&self, event: &OrderPlacedEvent) -> Result<(), ListenerError> {{\n//         // Verarbeite das Event\n//         println!(\"Handling event: {{:?}}\", event);\n//         \n//         // Führe Aktionen aus (z.B. E-Mail versenden)\n//         // self.send_email(&event.payload).await?;\n//         \n//         Ok(())\n//     }}\n// }}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n    #[test]\n    fn creates_listener_instance() {{\n        let listener = {name}Listener::new();\n        assert!(true); // Placeholder test\n    }}\n    \n    // TODO: Füge hier weitere Tests hinzu\n    // #[tokio::test]\n    // async fn handles_event_successfully() {{\n    //     let listener = {name}Listener::new();\n    //     let event = OrderPlacedEvent::new(...);\n    //     let result = listener.handle(&event).await;\n    //     assert!(result.is_ok());\n    // }}\n}}\n"
    )
}

fn command_template(name: &str) -> String {
    let slug = slugify(name);
    format!(
        "use async_trait::async_trait;\nuse foundry_domain::{{CommandDescriptor, CommandKind}};\nuse foundry_plugins::{{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand}};\nuse serde_json::json;\n\n/// Custom Command: {name}\n///\n/// Dieser Command kann über `foundry {slug}` ausgeführt werden.\n/// Implementiere die `execute()` Methode, um die Command-Logik zu definieren.\npub struct {name}Command {{\n    descriptor: CommandDescriptor,\n}}\n\nimpl {name}Command {{\n    pub fn new() -> Self {{\n        Self {{\n            descriptor: CommandDescriptor::builder(\"custom.{slug}\", \"{slug}\")\n                .summary(\"Custom Command: {name}\")\n                .description(\"Führt eine benutzerdefinierte Aktion aus.\")\n                .category(CommandKind::Custom)\n                .build(),\n        }}\n    }}\n}}\n\nimpl Default for {name}Command {{\n    fn default() -> Self {{\n        Self::new()\n    }}\n}}\n\n#[async_trait]\nimpl FoundryCommand for {name}Command {{\n    fn descriptor(&self) -> &CommandDescriptor {{\n        &self.descriptor\n    }}\n\n    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {{\n        // TODO: Implementiere hier die Command-Logik\n        println!(\"Executing {name}Command...\");\n        \n        // Beispiel: Zugriff auf Command-Argumente\n        // let arg = ctx.args.get(0).ok_or_else(|| CommandError::Message(\"Argument fehlt\".into()))?;\n        \n        // Beispiel: Zugriff auf Config\n        // let config_value = ctx.config.get(\"SOME_KEY\");\n        \n        let message = format!(\"Command {name} erfolgreich ausgeführt\");\n        let data = json!({{\n            \"executed\": true,\n            \"timestamp\": chrono::Utc::now().to_rfc3339(),\n        }});\n\n        Ok(CommandResult {{\n            status: CommandStatus::Success,\n            message: Some(message),\n            data: Some(data),\n            error: None,\n        }})\n    }}\n}}\n"
    )
}

fn factory_template(name: &str) -> String {
    format!(
        "use serde::{{Deserialize, Serialize}};\nuse std::sync::atomic::{{AtomicU64, Ordering}};\n\n/// Factory für Test-Daten-Generierung von `{name}`\n///\n/// Diese Factory erstellt Test-Instanzen mit realistischen, aber deterministischen Daten.\n/// Nutze `build()` für einzelne Instanzen oder `build_many(n)` für mehrere.\n#[derive(Debug, Clone)]\npub struct {name}Factory {{\n    sequence: &'static AtomicU64,\n}}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {name} {{\n    pub id: i64,\n    pub name: String,\n    pub description: String,\n    pub created_at: String,\n}}\n\nimpl Default for {name}Factory {{\n    fn default() -> Self {{\n        Self::new()\n    }}\n}}\n\nimpl {name}Factory {{\n    /// Erstellt eine neue Factory-Instanz\n    pub fn new() -> Self {{\n        static SEQUENCE: AtomicU64 = AtomicU64::new(1);\n        Self {{\n            sequence: &SEQUENCE,\n        }}\n    }}\n\n    /// Generiert eine einzelne Test-Instanz\n    pub fn build(&self) -> {name} {{\n        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);\n        {name} {{\n            id: seq as i64,\n            name: self.fake_name(seq),\n            description: self.fake_description(seq),\n            created_at: self.fake_timestamp(seq),\n        }}\n    }}\n\n    /// Generiert mehrere Test-Instanzen\n    pub fn build_many(&self, count: usize) -> Vec<{name}> {{\n        (0..count).map(|_| self.build()).collect()\n    }}\n\n    // Faker-ähnliche Helper-Methoden\n\n    fn fake_name(&self, seq: u64) -> String {{\n        let prefixes = [\"Test\", \"Demo\", \"Sample\", \"Example\", \"Mock\"];\n        let suffixes = [\"Item\", \"Entity\", \"Object\", \"Record\", \"Entry\"];\n        let prefix = prefixes[(seq as usize) % prefixes.len()];\n        let suffix = suffixes[(seq as usize / prefixes.len()) % suffixes.len()];\n        format!(\"{{}} {{}} #{{}}\", prefix, suffix, seq)\n    }}\n\n    fn fake_description(&self, seq: u64) -> String {{\n        let templates = [\n            \"A comprehensive description for testing purposes\",\n            \"Sample data entry for integration tests\",\n            \"Mock object with realistic attributes\",\n            \"Generated test fixture for validation\",\n            \"Automated test data with unique identifier\",\n        ];\n        let template = templates[(seq as usize) % templates.len()];\n        format!(\"{{}}.  ID: {{}}\", template, seq)\n    }}\n\n    fn fake_timestamp(&self, seq: u64) -> String {{\n        // Generiert deterministische Timestamps basierend auf Sequenz\n        let base_year = 2024;\n        let month = ((seq % 12) + 1).max(1).min(12);\n        let day = ((seq % 28) + 1).max(1).min(28);\n        let hour = (seq % 24).max(0).min(23);\n        let minute = ((seq * 17) % 60).max(0).min(59);\n        let second = ((seq * 37) % 60).max(0).min(59);\n        format!(\n            \"{{:04}}-{{:02}}-{{:02}}T{{:02}}:{{:02}}:{{:02}}Z\",\n            base_year, month, day, hour, minute, second\n        )\n    }}\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n    #[test]\n    fn factory_builds_unique_instances() {{\n        let factory = {name}Factory::new();\n        let item1 = factory.build();\n        let item2 = factory.build();\n\n        assert_ne!(item1.id, item2.id, \"IDs should be unique\");\n        assert_ne!(item1.name, item2.name, \"Names should be unique\");\n    }}\n\n    #[test]\n    fn factory_builds_multiple_instances() {{\n        let factory = {name}Factory::new();\n        let items = factory.build_many(5);\n\n        assert_eq!(items.len(), 5);\n        // Überprüfe dass alle IDs unique sind\n        let ids: std::collections::HashSet<_> = items.iter().map(|i| i.id).collect();\n        assert_eq!(ids.len(), 5);\n    }}\n\n    #[test]\n    fn factory_generates_realistic_data() {{\n        let factory = {name}Factory::new();\n        let item = factory.build();\n\n        assert!(!item.name.is_empty());\n        assert!(!item.description.is_empty());\n        assert!(item.created_at.contains('T')); // ISO8601 format\n    }}\n}}\n"
    )
}

fn controller_template(name: &str) -> String {
    format!(
        "use axum::extract::State;\nuse foundry_api::{{AppJson, ApiResult, AppState, JsonResponse}};\nuse foundry_plugins::ValidationRules;\nuse serde::{{Deserialize, Serialize}};\n\n#[derive(Clone)]\npub struct {name}Controller;\n\n#[derive(Serialize)]\npub struct {name}Resource {{\n    pub id: i64,\n}}\n\n#[derive(Serialize, Deserialize)]\npub struct Create{name}Payload {{\n    pub name: String,\n}}\n\npub async fn index(State(_state): State<AppState>) -> ApiResult<Vec<{name}Resource>> {{\n    Ok(JsonResponse::ok(vec![]))\n}}\n\npub async fn store(\n    State(state): State<AppState>,\n    payload: AppJson<Create{name}Payload>,\n) -> ApiResult<{name}Resource> {{\n    let rules = ValidationRules {{\n        rules: serde_json::json!({{\n            \"required\": [\"name\"],\n            \"fields\": {{\n                \"name\": {{ \"min_length\": 3 }}\n            }}\n        }}),\n    }};\n    payload.validate(&state, rules).await?;\n    let _input = payload.into_inner();\n\n    Ok(JsonResponse::created({name}Resource {{ id: 1 }}))\n}}\n"
    )
}

fn routes_template(_name: &str, slug: &str) -> String {
    format!(
        "use axum::routing::{{get, post, Route}};\nuse foundry_api::{{app_router, AppRouter, AppState}};\n\npub fn routes() -> AppRouter {{\n    app_router()\n        .route(\n            \"/{slug}\",\n            with_optional_middleware(get(crate::app::http::controllers::{slug}_controller::index)),\n        )\n        .route(\n            \"/{slug}\",\n            with_optional_middleware(post(crate::app::http::controllers::{slug}_controller::store)),\n        )\n}}\n\nfn with_optional_middleware(route: Route<AppState>) -> Route<AppState> {{\n    route\n    // .layer(axum::middleware::from_fn(\n    //     crate::app::http::middleware::ensure_token::ensure_token,\n    // ))\n}}\n"
    )
}

fn middleware_template(name: &str, slug: &str) -> String {
    format!(
        "// Middleware `{name}`\nuse axum::{{body::Body, http::StatusCode, middleware::Next}};\nuse axum::http::Request;\nuse axum::response::IntoResponse;\n\npub async fn {slug}(mut request: Request<Body>, next: Next) -> impl IntoResponse {{\n    let token = request\n        .headers()\n        .get(\"x-api-token\")\n        .and_then(|value| value.to_str().ok());\n\n    if token != Some(\"my-secret-token\") {{\n        return StatusCode::UNAUTHORIZED;\n    }}\n\n    next.run(request).await\n}}\n"
    )
}

fn kernel_template() -> String {
    "use axum::{\n    body::Body,\n    http::Request,\n    middleware::Next,\n    response::IntoResponse,\n    Router,\n};\nuse foundry_api::{app_router, AppRouter, HttpServer};\n\npub fn build(server: HttpServer) -> Router {\n    let server = server.merge_router(app_routes());\n    let server = server.with_middleware(global_middleware);\n    server.into_router()\n}\n\nfn app_routes() -> AppRouter {\n    app_router()\n    // .merge(crate::app::http::routes::account::routes())\n}\n\nasync fn global_middleware(request: Request<Body>, next: Next) -> impl IntoResponse {\n    // Customize global guards/logging here.\n    next.run(request).await\n}\n"
        .to_string()
}

fn apply_generator_plan(
    ctx: &CommandContext,
    plan: &GeneratorPlan,
) -> Result<Vec<String>, CommandError> {
    if ctx.options.dry_run {
        return Ok(Vec::new());
    }

    let mut written = Vec::with_capacity(plan.artifacts.len());
    for artifact in &plan.artifacts {
        let contents = artifact
            .preview
            .as_deref()
            .unwrap_or("// TODO: Inhalt definieren\n");
        ctx.artifacts
            .write_file(&artifact.path, contents, ctx.options.force)?;
        written.push(artifact.path.clone());
    }

    Ok(written)
}

fn register_domain_model(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let models_root = PathBuf::from(config_path(ctx, "FOUNDRY_DOMAIN_MODELS", "domain/models"));
    ensure_module_listing(models_root.join("mod.rs"), slug)?;
    for (mod_path, child) in module_links(&models_root, 1) {
        ensure_module_listing(mod_path, &child)?;
    }
    Ok(())
}

fn register_request_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let request_root = PathBuf::from(config_path(
        ctx,
        "FOUNDRY_HTTP_REQUESTS",
        "app/http/requests",
    ));
    ensure_module_listing(request_root.join("mod.rs"), &format!("{slug}_request"))?;
    for (mod_path, child) in module_links(&request_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }
    Ok(())
}

fn register_controller_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let controller_root = PathBuf::from(config_path(
        ctx,
        "FOUNDRY_HTTP_CONTROLLERS",
        "app/http/controllers",
    ));
    ensure_module_listing(
        controller_root.join("mod.rs"),
        &format!("{slug}_controller"),
    )?;
    for (mod_path, child) in module_links(&controller_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }

    let routes_root = PathBuf::from(config_path(ctx, "FOUNDRY_HTTP_ROUTES", "app/http/routes"));
    ensure_module_listing(routes_root.join("mod.rs"), slug)?;
    for (mod_path, child) in module_links(&routes_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }

    ensure_http_kernel(ctx)?;

    Ok(())
}

fn register_middleware_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let middleware_root = PathBuf::from(config_path(
        ctx,
        "FOUNDRY_HTTP_MIDDLEWARE",
        "app/http/middleware",
    ));
    ensure_module_listing(middleware_root.join("mod.rs"), slug)?;
    for (mod_path, child) in module_links(&middleware_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }

    ensure_http_kernel(ctx)?;
    Ok(())
}

fn register_job_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let jobs_root = PathBuf::from(config_path(ctx, "FOUNDRY_JOBS", "app/jobs"));
    ensure_module_listing(jobs_root.join("mod.rs"), &format!("{slug}_job"))?;
    for (mod_path, child) in module_links(&jobs_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }
    Ok(())
}

fn register_factory_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let factory_root = PathBuf::from(config_path(ctx, "FOUNDRY_FACTORIES", "app/factories"));
    ensure_module_listing(factory_root.join("mod.rs"), &format!("{slug}_factory"))?;
    for (mod_path, child) in module_links(&factory_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }
    Ok(())
}

fn register_event_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let events_root = PathBuf::from(config_path(ctx, "FOUNDRY_EVENTS", "app/events"));
    ensure_module_listing(events_root.join("mod.rs"), &format!("{slug}_event"))?;
    for (mod_path, child) in module_links(&events_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }
    Ok(())
}

fn register_listener_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let listeners_root = PathBuf::from(config_path(ctx, "FOUNDRY_LISTENERS", "app/listeners"));
    ensure_module_listing(listeners_root.join("mod.rs"), &format!("{slug}_listener"))?;
    for (mod_path, child) in module_links(&listeners_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }
    Ok(())
}

fn register_command_modules(ctx: &CommandContext, slug: &str) -> Result<(), CommandError> {
    let commands_root = PathBuf::from(config_path(ctx, "FOUNDRY_COMMANDS", "app/commands"));
    ensure_module_listing(commands_root.join("mod.rs"), &format!("{slug}_command"))?;
    for (mod_path, child) in module_links(&commands_root, 2) {
        ensure_module_listing(mod_path, &child)?;
    }
    Ok(())
}

fn ensure_http_kernel(ctx: &CommandContext) -> Result<(), CommandError> {
    let http_root = resolve_http_root(ctx);
    let http_mod = http_root.join("mod.rs");
    if let Some(parent) = http_mod.parent() {
        fs::create_dir_all(parent).map_err(|err| CommandError::Other(err.into()))?;
    }
    let mut http_content = if http_mod.exists() {
        fs::read_to_string(&http_mod).map_err(|err| CommandError::Other(err.into()))?
    } else {
        String::from("// @generated by Foundry CLI\n\n")
    };
    if !http_content
        .lines()
        .any(|line| line.trim() == "pub mod kernel;")
    {
        if !http_content.ends_with('\n') {
            http_content.push('\n');
        }
        http_content.push_str("pub mod kernel;\n");
        fs::write(&http_mod, http_content).map_err(|err| CommandError::Other(err.into()))?;
    }

    for (mod_path, child) in module_links(&http_root, 1) {
        ensure_module_listing(mod_path, &child)?;
    }

    let kernel_path = http_root.join("kernel.rs");
    if !kernel_path.exists() {
        if let Some(parent) = kernel_path.parent() {
            fs::create_dir_all(parent).map_err(|err| CommandError::Other(err.into()))?;
        }
        fs::write(&kernel_path, kernel_template())
            .map_err(|err| CommandError::Other(err.into()))?;
    }

    Ok(())
}

fn ensure_module_listing(path: PathBuf, module: &str) -> Result<(), CommandError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| CommandError::Other(err.into()))?;
    }

    let entry = format!("pub mod {module};");
    let mut content = if path.exists() {
        fs::read_to_string(&path).map_err(|err| CommandError::Other(err.into()))?
    } else {
        String::from("// @generated by Foundry CLI\n\n")
    };

    if content.lines().any(|line| line.trim() == entry) {
        return Ok(());
    }

    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&entry);
    content.push('\n');

    fs::write(&path, content).map_err(|err| CommandError::Other(err.into()))?;
    Ok(())
}

fn module_links(path: &Path, levels: usize) -> Vec<(PathBuf, String)> {
    let mut current = path.to_path_buf();
    let mut links = Vec::new();
    let mut remaining = levels;

    while remaining > 0 {
        let module_name = match current.file_name().and_then(|value| value.to_str()) {
            Some(name) => name.to_string(),
            None => break,
        };

        let parent = match current.parent() {
            Some(parent) if parent.file_name().is_some() => parent.to_path_buf(),
            _ => break,
        };

        links.push((parent.join("mod.rs"), module_name));
        current = parent;
        remaining -= 1;
    }

    links
}

fn resolve_http_root(ctx: &CommandContext) -> PathBuf {
    let controllers_root = PathBuf::from(config_path(
        ctx,
        "FOUNDRY_HTTP_CONTROLLERS",
        "app/http/controllers",
    ));
    if let Some(parent) = controllers_root.parent() {
        return parent.to_path_buf();
    }

    let routes_root = PathBuf::from(config_path(ctx, "FOUNDRY_HTTP_ROUTES", "app/http/routes"));
    if let Some(parent) = routes_root.parent() {
        return parent.to_path_buf();
    }

    let middleware_root = PathBuf::from(config_path(
        ctx,
        "FOUNDRY_HTTP_MIDDLEWARE",
        "app/http/middleware",
    ));
    if let Some(parent) = middleware_root.parent() {
        return parent.to_path_buf();
    }

    PathBuf::from("app/http")
}

// =============================================================================
// Sprint 9: Low Priority / Optional Commands
// =============================================================================

/// MakeAuthCommand - Vollständige Auth-Scaffold Implementation
pub struct MakeAuthCommand {
    descriptor: CommandDescriptor,
}

impl MakeAuthCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("generator.make_auth", "make:auth")
                .summary("Generiert eine vollständige Authentication-Struktur")
                .description(
                    "Erstellt Auth-Controller, Login/Register-Requests und Route-Stubs für eine Basic-Auth-Flow.",
                )
                .category(CommandKind::Generator)
                .alias("make auth")
                .build(),
        }
    }

    fn compute_plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        let controller_root = config_path(ctx, "FOUNDRY_HTTP_CONTROLLERS", "app/http/controllers");
        let routes_root = config_path(ctx, "FOUNDRY_HTTP_ROUTES", "app/http/routes");
        let request_root = config_path(ctx, "FOUNDRY_HTTP_REQUESTS", "app/http/requests");

        let plan = GeneratorPlan {
            artifacts: vec![
                GeneratedArtifact {
                    path: format!("{controller_root}/auth_controller.rs"),
                    description: "Auth Controller mit Login/Register/Logout".to_string(),
                    preview: Some(auth_controller_template()),
                },
                GeneratedArtifact {
                    path: format!("{routes_root}/auth.rs"),
                    description: "Auth-Routes für /auth endpoints".to_string(),
                    preview: Some(auth_routes_template()),
                },
                GeneratedArtifact {
                    path: format!("{request_root}/login_request.rs"),
                    description: "Login Payload mit Validierung".to_string(),
                    preview: Some(login_request_template()),
                },
                GeneratedArtifact {
                    path: format!("{request_root}/register_request.rs"),
                    description: "Register Payload mit Validierung".to_string(),
                    preview: Some(register_request_template()),
                },
            ],
            summary: Some("Erzeugt vollständige Auth-Struktur (Login, Register, Controller)".to_string()),
        };

        Ok(plan)
    }
}

impl Default for MakeAuthCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FoundryCommand for MakeAuthCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let plan = self.compute_plan(&ctx)?;
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let total = plan.artifacts.len();
        let written = apply_generator_plan(&ctx, &plan)?;

        if !ctx.options.dry_run {
            register_auth_modules(&ctx)?;
        }

        let message = match format {
            ResponseFormat::Human => {
                if ctx.options.dry_run {
                    format!("make:auth → {total} Artefakte geplant (dry-run).")
                } else {
                    format!("make:auth → {total} Artefakte erzeugt. Auth-Flow bereit!")
                }
            }
            ResponseFormat::Json => {
                if ctx.options.dry_run {
                    "planned make:auth".to_string()
                } else {
                    "generated make:auth".to_string()
                }
            }
        };

        let data = json!({
            "plan": plan,
            "input": {
                "args": args_snapshot,
            },
            "dry_run": ctx.options.dry_run,
            "written": written,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[async_trait]
impl FoundryGenerator for MakeAuthCommand {
    async fn plan(&self, ctx: &CommandContext) -> Result<GeneratorPlan, CommandError> {
        self.compute_plan(ctx)
    }
}


/// InstallPackageCommand - Package-Installer Stub (Passport/Sanctum)
pub struct InstallPackageCommand {
    descriptor: CommandDescriptor,
}

impl InstallPackageCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("packages.install", "install:package")
                .summary("Installiert Auth-Packages (Passport/Sanctum) - Coming Soon")
                .description(
                    "Installiert und konfiguriert OAuth2/Token-Auth-Pakete. Diese Funktion ist derzeit nicht verfügbar.",
                )
                .category(CommandKind::Utility)
                .alias("install passport")
                .alias("install sanctum")
                .build(),
        }
    }
}

impl Default for InstallPackageCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FoundryCommand for InstallPackageCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let package = ctx.args.first().map(|s| s.as_str()).unwrap_or("package");

        let message = match ctx.format {
            ResponseFormat::Human => {
                format!(
                    "install:package → Feature nicht verfügbar.\n\n\
                     Die automatische Installation von Auth-Paketen (z.B. {}) ist derzeit nicht implementiert.\n\n\
                     Für Auth-Funktionalität empfehlen wir:\n\
                     - `foundry make:auth` für Basic-Auth-Scaffold\n\
                     - Manuelle Integration von OAuth2-Crates (oauth2, jsonwebtoken)\n\
                     - See documentation: https://docs.rs/oauth2\n\n\
                     Diese Funktion könnte in einem zukünftigen Release hinzugefügt werden.",
                    package
                )
            }
            ResponseFormat::Json => format!("install:package not available for {}", package),
        };

        let data = json!({
            "available": false,
            "requested_package": package,
            "reason": "Package installation requires Cargo.toml modification and config generation",
            "alternatives": [
                "foundry make:auth",
                "Manual oauth2 crate integration",
                "jsonwebtoken for JWT"
            ]
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

// =============================================================================
// Auth Templates
// =============================================================================

fn auth_controller_template() -> String {
    r#"use axum::extract::State;
use foundry_api::{AppJson, ApiResult, AppState, JsonResponse};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AuthController;

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: i64,
}

#[derive(Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

/// POST /auth/login
pub async fn login(
    State(state): State<AppState>,
    payload: AppJson<crate::app::http::requests::login_request::LoginRequest>,
) -> ApiResult<AuthResponse> {
    payload.validate(&state, crate::app::http::requests::login_request::LoginRequest::rules()).await?;
    let input = payload.into_inner();

    // TODO: Implementiere Login-Logik
    // 1. Finde User in Datenbank
    // 2. Verifiziere Passwort (bcrypt)
    // 3. Generiere JWT-Token oder Session

    // Placeholder response
    Ok(JsonResponse::ok(AuthResponse {
        token: "dummy-jwt-token".to_string(),
        user_id: 1,
    }))
}

/// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    payload: AppJson<crate::app::http::requests::register_request::RegisterRequest>,
) -> ApiResult<AuthResponse> {
    payload.validate(&state, crate::app::http::requests::register_request::RegisterRequest::rules()).await?;
    let input = payload.into_inner();

    // TODO: Implementiere Registration-Logik
    // 1. Validiere ob Email bereits existiert
    // 2. Hash Passwort (bcrypt)
    // 3. Erstelle User in Datenbank
    // 4. Generiere JWT-Token oder Session

    // Placeholder response
    Ok(JsonResponse::created(AuthResponse {
        token: "dummy-jwt-token".to_string(),
        user_id: 1,
    }))
}

/// POST /auth/logout
pub async fn logout(State(_state): State<AppState>) -> ApiResult<LogoutResponse> {
    // TODO: Implementiere Logout-Logik
    // 1. Invalidiere JWT-Token (Blacklist)
    // 2. Oder lösche Session aus Store

    Ok(JsonResponse::ok(LogoutResponse {
        message: "Successfully logged out".to_string(),
    }))
}
"#.to_string()
}

fn auth_routes_template() -> String {
    r#"use axum::routing::post;
use foundry_api::{app_router, AppRouter};

pub fn routes() -> AppRouter {
    app_router()
        .route(
            "/auth/login",
            post(crate::app::http::controllers::auth_controller::login),
        )
        .route(
            "/auth/register",
            post(crate::app::http::controllers::auth_controller::register),
        )
        .route(
            "/auth/logout",
            post(crate::app::http::controllers::auth_controller::logout),
        )
}
"#.to_string()
}

fn login_request_template() -> String {
    r#"use foundry_plugins::ValidationRules;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl LoginRequest {
    pub fn rules() -> ValidationRules {
        ValidationRules {
            rules: serde_json::json!({
                "required": ["email", "password"],
                "fields": {
                    "email": {
                        "format": "email",
                        "min_length": 3
                    },
                    "password": {
                        "min_length": 6
                    }
                }
            }),
        }
    }
}
"#.to_string()
}

fn register_request_template() -> String {
    r#"use foundry_plugins::ValidationRules;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
    pub name: Option<String>,
}

impl RegisterRequest {
    pub fn rules() -> ValidationRules {
        ValidationRules {
            rules: serde_json::json!({
                "required": ["email", "password", "password_confirmation"],
                "fields": {
                    "email": {
                        "format": "email",
                        "min_length": 3
                    },
                    "password": {
                        "min_length": 8,
                        "matches": "^(?=.*[A-Za-z])(?=.*\\d)[A-Za-z\\d@$!%*#?&]{8,}$"
                    },
                    "password_confirmation": {
                        "matches_field": "password"
                    },
                    "name": {
                        "min_length": 2
                    }
                }
            }),
        }
    }
}
"#.to_string()
}

fn register_auth_modules(ctx: &CommandContext) -> Result<(), CommandError> {
    let controller_root = PathBuf::from(config_path(
        ctx,
        "FOUNDRY_HTTP_CONTROLLERS",
        "app/http/controllers",
    ));
    ensure_module_listing(controller_root.join("mod.rs"), "auth_controller")?;

    let routes_root = PathBuf::from(config_path(ctx, "FOUNDRY_HTTP_ROUTES", "app/http/routes"));
    ensure_module_listing(routes_root.join("mod.rs"), "auth")?;

    let request_root = PathBuf::from(config_path(
        ctx,
        "FOUNDRY_HTTP_REQUESTS",
        "app/http/requests",
    ));
    ensure_module_listing(request_root.join("mod.rs"), "login_request")?;
    ensure_module_listing(request_root.join("mod.rs"), "register_request")?;

    for (mod_path, child) in module_links(&controller_root, 1) {
        ensure_module_listing(mod_path, &child)?;
    }
    for (mod_path, child) in module_links(&routes_root, 1) {
        ensure_module_listing(mod_path, &child)?;
    }
    for (mod_path, child) in module_links(&request_root, 1) {
        ensure_module_listing(mod_path, &child)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_infra::{
        InMemoryCacheStore, InMemoryEventBus, InMemoryQueue, InMemoryStorage,
        SimpleValidationService,
    };
    use foundry_plugins::{
        ArtifactPort, CommandError, CommandStatus, ExecutionOptions, MigrationPort, MigrationRun,
        ResponseFormat, SeedPort, SeedRun,
    };
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[derive(Default)]
    struct MemoryArtifacts {
        written: Mutex<Vec<(String, String)>>,
    }

    impl ArtifactPort for MemoryArtifacts {
        fn write_file(&self, path: &str, contents: &str, _force: bool) -> Result<(), CommandError> {
            self.written
                .lock()
                .unwrap()
                .push((path.to_string(), contents.to_string()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct NoopMigrations;

    #[async_trait]
    impl MigrationPort for NoopMigrations {
        async fn apply(
            &self,
            _config: &Value,
            _dry_run: bool,
        ) -> Result<MigrationRun, CommandError> {
            Ok(MigrationRun::default())
        }

        async fn rollback(
            &self,
            _config: &Value,
            _dry_run: bool,
        ) -> Result<MigrationRun, CommandError> {
            Ok(MigrationRun::default())
        }
    }

    #[derive(Default)]
    struct NoopSeeds;

    #[async_trait]
    impl SeedPort for NoopSeeds {
        async fn run(&self, _config: &Value, _dry_run: bool) -> Result<SeedRun, CommandError> {
            Ok(SeedRun::default())
        }
    }

    struct TestHarness {
        ctx: CommandContext,
        artifacts: Arc<MemoryArtifacts>,
        temp: TempDir,
    }

    fn base_harness() -> TestHarness {
        let temp = TempDir::new().expect("temp dir");
        let artifacts = Arc::new(MemoryArtifacts::default());
        let config = json!({
            "FOUNDRY_DOMAIN_MODELS": temp
                .path()
                .join("domain/models")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_MIGRATIONS": temp
                .path()
                .join("migrations")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_CONTROLLERS": temp
                .path()
                .join("app/http/controllers")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_ROUTES": temp
                .path()
                .join("app/http/routes")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_MIDDLEWARE": temp
                .path()
                .join("app/http/middleware")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_REQUESTS": temp
                .path()
                .join("app/http/requests")
                .to_string_lossy()
                .to_string(),
        });

        let ctx = CommandContext {
            args: vec!["Account".into()],
            format: ResponseFormat::Human,
            metadata: Value::Null,
            config,
            options: ExecutionOptions::default(),
            artifacts: artifacts.clone() as Arc<dyn ArtifactPort>,
            migrations: Arc::new(NoopMigrations::default()),
            seeds: Arc::new(NoopSeeds::default()),
            validation: Arc::new(SimpleValidationService::default()),
            storage: Arc::new(InMemoryStorage::default()),
            cache: Arc::new(InMemoryCacheStore::default()),
            queue: Arc::new(InMemoryQueue::default()),
            events: Arc::new(InMemoryEventBus::default()),
        };
        TestHarness {
            ctx,
            artifacts,
            temp,
        }
    }

    #[test]
    fn slugify_handles_mixed_case() {
        assert_eq!(slugify("UserProfile"), "user_profile");
        assert_eq!(slugify("user-profile"), "user_profile");
        assert_eq!(slugify("  Space  Name "), "space_name");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_model_dry_run_writes_nothing() {
        let command = MakeModelCommand::new();
        let mut harness = base_harness();
        harness.ctx.options.dry_run = true;

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("dry-run succeeds");
        assert_eq!(result.status, CommandStatus::Success);
        assert!(harness.artifacts.written.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_model_generates_three_artifacts() {
        let command = MakeModelCommand::new();
        let harness = base_harness();
        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let written = harness.artifacts.written.lock().unwrap();
        assert_eq!(written.len(), 3);
        assert!(written.iter().any(|(path, _)| path.ends_with(".rs")));
        assert!(
            written
                .iter()
                .filter(|(path, _)| path.ends_with(".sql"))
                .count()
                == 2
        );

        let models_mod = harness.temp.path().join("domain/models/mod.rs");
        assert!(
            models_mod.exists(),
            "expected generated models mod file at {}",
            models_mod.display()
        );
        let contents = fs::read_to_string(models_mod).expect("read models mod");
        assert!(
            contents.contains("pub mod account;"),
            "expected mod registration in {contents}"
        );

        let domain_mod = harness.temp.path().join("domain/mod.rs");
        let domain_contents = fs::read_to_string(&domain_mod).expect("read domain mod");
        assert!(
            domain_contents.contains("pub mod models;"),
            "expected domain mod to register models"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_model_rejects_invalid_name() {
        let command = MakeModelCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["1invalid".into()];

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute returns result");
        assert_eq!(result.status, CommandStatus::Failure);
        let error = result.error.expect("validation error present");
        assert_eq!(error.code, "VALIDATION_FAILED");
        assert!(error
            .context()
            .iter()
            .any(|ctx| ctx.key == "name" && ctx.value.contains("invalid")));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_controller_registers_modules() {
        let command = MakeControllerCommand::new();
        let harness = base_harness();

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let controller_mod = harness.temp.path().join("app/http/controllers/mod.rs");
        let controller_contents =
            fs::read_to_string(&controller_mod).expect("controller mod exists");
        assert!(
            controller_contents.contains("pub mod account_controller;"),
            "expected controller registration, got {controller_contents}"
        );

        let routes_mod = harness.temp.path().join("app/http/routes/mod.rs");
        let routes_contents = fs::read_to_string(&routes_mod).expect("routes mod exists");
        assert!(
            routes_contents.contains("pub mod account;"),
            "expected routes registration, got {routes_contents}"
        );

        let http_mod = harness.temp.path().join("app/http/mod.rs");
        let http_contents = fs::read_to_string(&http_mod).expect("http mod exists");
        assert!(
            http_contents.contains("pub mod controllers;"),
            "expected http mod to register controllers"
        );
        assert!(
            http_contents.contains("pub mod routes;"),
            "expected http mod to register routes"
        );
        assert!(
            http_contents.contains("pub mod kernel;"),
            "expected http mod to register kernel"
        );

        let app_mod = harness.temp.path().join("app/mod.rs");
        let app_contents = fs::read_to_string(&app_mod).expect("app mod exists");
        assert!(
            app_contents.contains("pub mod http;"),
            "expected app mod to register http"
        );

        let kernel_file = harness.temp.path().join("app/http/kernel.rs");
        assert!(kernel_file.exists(), "expected kernel file to exist");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_middleware_registers_modules() {
        let command = MakeMiddlewareCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["EnsureToken".into()];

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let middleware_mod = harness.temp.path().join("app/http/middleware/mod.rs");
        let middleware_contents =
            fs::read_to_string(&middleware_mod).expect("middleware mod exists");
        assert!(
            middleware_contents.contains("pub mod ensure_token;"),
            "expected middleware registration, got {middleware_contents}"
        );

        let http_mod = harness.temp.path().join("app/http/mod.rs");
        let http_contents = fs::read_to_string(&http_mod).expect("http mod exists");
        assert!(
            http_contents.contains("pub mod middleware;"),
            "expected http mod to register middleware"
        );
        assert!(
            http_contents.contains("pub mod kernel;"),
            "expected http mod to register kernel"
        );

        let app_mod = harness.temp.path().join("app/mod.rs");
        let app_contents = fs::read_to_string(&app_mod).expect("app mod exists");
        assert!(
            app_contents.contains("pub mod http;"),
            "expected app mod to register http"
        );

        let written = harness.artifacts.written.lock().unwrap();
        let entry = written
            .iter()
            .find(|(path, _)| path.ends_with("app/http/middleware/ensure_token.rs"))
            .expect("middleware artifact written");
        assert!(
            entry.1.contains("pub async fn ensure_token"),
            "expected middleware function in generated artifact"
        );

        let kernel_file = harness.temp.path().join("app/http/kernel.rs");
        assert!(kernel_file.exists(), "expected kernel file to exist");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_job_registers_modules() {
        let command = MakeJobCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["SendEmail".into()];
        harness.ctx.config = json!({
            "FOUNDRY_DOMAIN_MODELS": harness.temp
                .path()
                .join("domain/models")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_MIGRATIONS": harness.temp
                .path()
                .join("migrations")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_JOBS": harness.temp
                .path()
                .join("app/jobs")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_CONTROLLERS": harness.temp
                .path()
                .join("app/http/controllers")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_ROUTES": harness.temp
                .path()
                .join("app/http/routes")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_MIDDLEWARE": harness.temp
                .path()
                .join("app/http/middleware")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let jobs_mod = harness.temp.path().join("app/jobs/mod.rs");
        let jobs_contents = fs::read_to_string(&jobs_mod).expect("jobs mod exists");
        assert!(
            jobs_contents.contains("pub mod send_email_job;"),
            "expected job registration, got {jobs_contents}"
        );

        let written = harness.artifacts.written.lock().unwrap();
        let entry = written
            .iter()
            .find(|(path, _)| path.ends_with("app/jobs/send_email_job.rs"))
            .expect("job artifact written");
        assert!(
            entry.1.contains("pub struct SendEmailJob"),
            "expected job struct in generated artifact"
        );
        assert!(
            entry.1.contains("pub async fn handle"),
            "expected handle method in generated artifact"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_factory_generates_test_data_factory() {
        let command = MakeFactoryCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["Product".into()];
        harness.ctx.config = json!({
            "FOUNDRY_DOMAIN_MODELS": harness.temp
                .path()
                .join("domain/models")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_MIGRATIONS": harness.temp
                .path()
                .join("migrations")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_FACTORIES": harness.temp
                .path()
                .join("app/factories")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_CONTROLLERS": harness.temp
                .path()
                .join("app/http/controllers")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_ROUTES": harness.temp
                .path()
                .join("app/http/routes")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_MIDDLEWARE": harness.temp
                .path()
                .join("app/http/middleware")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let factories_mod = harness.temp.path().join("app/factories/mod.rs");
        let factories_contents = fs::read_to_string(&factories_mod).expect("factories mod exists");
        assert!(
            factories_contents.contains("pub mod product_factory;"),
            "expected factory registration, got {factories_contents}"
        );

        let written = harness.artifacts.written.lock().unwrap();
        let entry = written
            .iter()
            .find(|(path, _)| path.ends_with("app/factories/product_factory.rs"))
            .expect("factory artifact written");

        // Verify factory structure
        assert!(
            entry.1.contains("pub struct ProductFactory"),
            "expected factory struct in generated artifact"
        );
        assert!(
            entry.1.contains("pub fn build(&self) -> Product"),
            "expected build method in generated artifact"
        );
        assert!(
            entry.1.contains("pub fn build_many(&self, count: usize) -> Vec<Product>"),
            "expected build_many method in generated artifact"
        );

        // Verify Faker-like methods
        assert!(
            entry.1.contains("fn fake_name"),
            "expected fake_name method in generated artifact"
        );
        assert!(
            entry.1.contains("fn fake_description"),
            "expected fake_description method in generated artifact"
        );
        assert!(
            entry.1.contains("fn fake_timestamp"),
            "expected fake_timestamp method in generated artifact"
        );

        // Verify test module
        assert!(
            entry.1.contains("#[cfg(test)]"),
            "expected test module in generated artifact"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_factory_dry_run_writes_nothing() {
        let command = MakeFactoryCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["Product".into()];
        harness.ctx.options.dry_run = true;
        harness.ctx.config = json!({
            "FOUNDRY_FACTORIES": harness.temp
                .path()
                .join("app/factories")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("dry-run succeeds");
        assert_eq!(result.status, CommandStatus::Success);
        assert!(harness.artifacts.written.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_event_generates_event_class() {
        let command = MakeEventCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["OrderPlaced".into()];
        harness.ctx.config = json!({
            "FOUNDRY_EVENTS": harness.temp
                .path()
                .join("app/events")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_CONTROLLERS": harness.temp
                .path()
                .join("app/http/controllers")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_ROUTES": harness.temp
                .path()
                .join("app/http/routes")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_MIDDLEWARE": harness.temp
                .path()
                .join("app/http/middleware")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let events_mod = harness.temp.path().join("app/events/mod.rs");
        let events_contents = fs::read_to_string(&events_mod).expect("events mod exists");
        assert!(
            events_contents.contains("pub mod order_placed_event;"),
            "expected event registration, got {events_contents}"
        );

        let written = harness.artifacts.written.lock().unwrap();
        let entry = written
            .iter()
            .find(|(path, _)| path.ends_with("app/events/order_placed_event.rs"))
            .expect("event artifact written");

        // Verify event structure
        assert!(
            entry.1.contains("pub struct OrderPlacedEvent"),
            "expected event struct in generated artifact"
        );
        assert!(
            entry.1.contains("pub struct OrderPlacedPayload"),
            "expected payload struct in generated artifact"
        );
        assert!(
            entry.1.contains("pub fn new(payload: OrderPlacedPayload) -> Self"),
            "expected new method in generated artifact"
        );
        assert!(
            entry.1.contains("pub fn event_name() -> &'static str"),
            "expected event_name method in generated artifact"
        );
        assert!(
            entry.1.contains("pub event_id: String"),
            "expected event_id field in generated artifact"
        );
        assert!(
            entry.1.contains("pub occurred_at: DateTime<Utc>"),
            "expected occurred_at field in generated artifact"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_event_dry_run_writes_nothing() {
        let command = MakeEventCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["OrderPlaced".into()];
        harness.ctx.options.dry_run = true;
        harness.ctx.config = json!({
            "FOUNDRY_EVENTS": harness.temp
                .path()
                .join("app/events")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("dry-run succeeds");
        assert_eq!(result.status, CommandStatus::Success);
        assert!(harness.artifacts.written.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_listener_generates_listener_class() {
        let command = MakeListenerCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["SendOrderEmail".into()];
        harness.ctx.config = json!({
            "FOUNDRY_LISTENERS": harness.temp
                .path()
                .join("app/listeners")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_CONTROLLERS": harness.temp
                .path()
                .join("app/http/controllers")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_ROUTES": harness.temp
                .path()
                .join("app/http/routes")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_MIDDLEWARE": harness.temp
                .path()
                .join("app/http/middleware")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let listeners_mod = harness.temp.path().join("app/listeners/mod.rs");
        let listeners_contents = fs::read_to_string(&listeners_mod).expect("listeners mod exists");
        assert!(
            listeners_contents.contains("pub mod send_order_email_listener;"),
            "expected listener registration, got {listeners_contents}"
        );

        let written = harness.artifacts.written.lock().unwrap();
        let entry = written
            .iter()
            .find(|(path, _)| path.ends_with("app/listeners/send_order_email_listener.rs"))
            .expect("listener artifact written");

        // Verify listener structure
        assert!(
            entry.1.contains("pub struct SendOrderEmailListener"),
            "expected listener struct in generated artifact"
        );
        assert!(
            entry.1.contains("pub trait EventListener<E>"),
            "expected EventListener trait in generated artifact"
        );
        assert!(
            entry.1.contains("async fn handle(&self, event: &E) -> Result<(), ListenerError>"),
            "expected handle method signature in generated artifact"
        );
        assert!(
            entry.1.contains("pub enum ListenerError"),
            "expected ListenerError enum in generated artifact"
        );
        assert!(
            entry.1.contains("pub fn new() -> Self"),
            "expected new method in generated artifact"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_listener_dry_run_writes_nothing() {
        let command = MakeListenerCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["SendOrderEmail".into()];
        harness.ctx.options.dry_run = true;
        harness.ctx.config = json!({
            "FOUNDRY_LISTENERS": harness.temp
                .path()
                .join("app/listeners")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("dry-run succeeds");
        assert_eq!(result.status, CommandStatus::Success);
        assert!(harness.artifacts.written.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_command_generates_custom_command() {
        let command = MakeCommandCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["SyncExternalApi".into()];
        harness.ctx.config = json!({
            "FOUNDRY_COMMANDS": harness.temp
                .path()
                .join("app/commands")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_CONTROLLERS": harness.temp
                .path()
                .join("app/http/controllers")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_ROUTES": harness.temp
                .path()
                .join("app/http/routes")
                .to_string_lossy()
                .to_string(),
            "FOUNDRY_HTTP_MIDDLEWARE": harness.temp
                .path()
                .join("app/http/middleware")
                .to_string_lossy()
                .to_string(),
        });

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let commands_mod = harness.temp.path().join("app/commands/mod.rs");
        let commands_contents = fs::read_to_string(&commands_mod).expect("commands mod exists");
        assert!(
            commands_contents.contains("pub mod sync_external_api_command;"),
            "expected command registration, got {commands_contents}"
        );

        let written = harness.artifacts.written.lock().unwrap();
        let entry = written
            .iter()
            .find(|(path, _)| path.ends_with("app/commands/sync_external_api_command.rs"))
            .expect("command artifact written");

        // Verify command structure
        assert!(
            entry.1.contains("pub struct SyncExternalApiCommand"),
            "expected command struct in generated artifact"
        );
        assert!(
            entry.1.contains("impl FoundryCommand for SyncExternalApiCommand"),
            "expected FoundryCommand impl in generated artifact"
        );
        assert!(
            entry.1.contains("async fn execute(&self, "),
            "expected execute method signature in generated artifact"
        );
        assert!(
            entry.1.contains("CommandDescriptor"),
            "expected CommandDescriptor in generated artifact"
        );
    }

    // =============================================================================
    // Sprint 9: Tests für Low Priority / Optional Commands
    // =============================================================================

    #[tokio::test(flavor = "current_thread")]
    async fn make_auth_generates_full_auth_scaffold() {
        let command = MakeAuthCommand::new();
        let harness = base_harness();

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        // Check written artifacts in memory
        let written = harness.artifacts.written.lock().unwrap();
        assert_eq!(written.len(), 4, "should write 4 artifacts");

        // Check auth controller
        let controller_entry = written
            .iter()
            .find(|(path, _)| path.contains("auth_controller.rs"))
            .expect("auth controller should be written");
        assert!(controller_entry.1.contains("pub async fn login"));
        assert!(controller_entry.1.contains("pub async fn register"));
        assert!(controller_entry.1.contains("pub async fn logout"));

        // Check auth routes
        let routes_entry = written
            .iter()
            .find(|(path, _)| path.contains("routes/auth.rs"))
            .expect("auth routes should be written");
        assert!(routes_entry.1.contains("/auth/login"));
        assert!(routes_entry.1.contains("/auth/register"));
        assert!(routes_entry.1.contains("/auth/logout"));

        // Check login request
        let login_entry = written
            .iter()
            .find(|(path, _)| path.contains("login_request.rs"))
            .expect("login request should be written");
        assert!(login_entry.1.contains("pub struct LoginRequest"));
        assert!(login_entry.1.contains("ValidationRules"));

        // Check register request
        let register_entry = written
            .iter()
            .find(|(path, _)| path.contains("register_request.rs"))
            .expect("register request should be written");
        assert!(register_entry.1.contains("pub struct RegisterRequest"));
        assert!(register_entry.1.contains("password_confirmation"));

        // Check module registration (written to disk via ensure_module_listing)
        let controller_mod = harness.temp.path().join("app/http/controllers/mod.rs");
        let controller_mod_contents = fs::read_to_string(&controller_mod).expect("controller mod exists");
        assert!(
            controller_mod_contents.contains("pub mod auth_controller;"),
            "expected auth_controller registration"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn make_auth_dry_run_writes_nothing() {
        let command = MakeAuthCommand::new();
        let mut harness = base_harness();
        harness.ctx.options.dry_run = true;

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("dry-run succeeds");
        assert_eq!(result.status, CommandStatus::Success);
        assert!(harness.artifacts.written.lock().unwrap().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tinker_returns_unavailable_message() {
        let command = TinkerCommand::new();
        let harness = base_harness();

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let message = result.message.expect("message present");
        assert!(message.contains("nicht verfügbar"));
        assert!(message.contains("cargo test"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn install_package_returns_unavailable_message() {
        let command = InstallPackageCommand::new();
        let mut harness = base_harness();
        harness.ctx.args = vec!["passport".into()];

        let result = command
            .execute(harness.ctx.clone())
            .await
            .expect("execute succeeds");
        assert_eq!(result.status, CommandStatus::Success);

        let message = result.message.expect("message present");
        assert!(message.contains("nicht verfügbar"));
        assert!(message.contains("passport"));
        assert!(message.contains("foundry make:auth"));
    }
}
