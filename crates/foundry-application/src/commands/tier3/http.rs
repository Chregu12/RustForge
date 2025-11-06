//! HTTP client commands

use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};

/// http:request <method> <url>
pub struct HttpRequestCommand {
    descriptor: CommandDescriptor,
}

impl HttpRequestCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("http:request", "http:request")
                .summary("Make an HTTP request")
                .description("Make an HTTP request (GET, POST, etc.) using the HTTP client")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for HttpRequestCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        if ctx.args.len() < 2 {
            return Err(CommandError::Message(
                "Method and URL required".to_string(),
            ));
        }

        let method = &ctx.args[0];
        let url = &ctx.args[1];

        // TODO: Implement actual HTTP request using foundry-http-client
        let message = format!("Would make {} request to: {}", method.to_uppercase(), url);

        Ok(CommandResult::success(message))
    }
}
