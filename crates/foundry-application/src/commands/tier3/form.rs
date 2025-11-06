//! Form builder commands

use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};

/// make:form <Name>
pub struct MakeFormCommand {
    descriptor: CommandDescriptor,
}

impl MakeFormCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("make:form", "make:form")
                .summary("Generate a form builder class")
                .description("Create a new form builder class with common fields")
                .category(CommandKind::Generator)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for MakeFormCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let form_name = ctx
            .args
            .first()
            .ok_or_else(|| CommandError::Message("Form name required".to_string()))?;

        let form_path = format!("app/Forms/{}.rs", form_name);
        let content = format!(
            r#"//! {} form

use foundry_forms::{{Form, Field, Theme, FormData, FormErrors}};

pub struct {} {{
    // Add configuration fields
}}

impl {} {{
    pub fn new() -> Self {{
        Self {{}}
    }}

    pub fn build(&self) -> Form {{
        Form::new("{}")
            .action("/submit")
            .field(
                Field::text("name")
                    .label("Name")
                    .placeholder("Enter your name")
                    .required()
                    .build()
            )
            .field(
                Field::email("email")
                    .label("Email")
                    .placeholder("your@email.com")
                    .required()
                    .build()
            )
            .field(
                Field::textarea("message")
                    .label("Message")
                    .help("Enter your message here")
                    .required()
                    .build()
            )
            .submit("Submit")
            .build()
    }}

    pub fn render(&self) -> anyhow::Result<String> {{
        self.build().render(Theme::Tailwind)
    }}

    pub fn validate(&self, data: &FormData) -> Result<(), FormErrors> {{
        self.build().validate(data)
    }}
}}
"#,
            form_name, form_name, form_name, form_name.to_lowercase()
        );

        ctx.artifacts.write_file(&form_path, &content, ctx.options.force)?;

        Ok(CommandResult::success(format!("Form created: {}", form_path)))
    }
}
