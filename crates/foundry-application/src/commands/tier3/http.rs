//! HTTP client commands

use async_trait::async_trait;
use foundry_domain::CommandDescriptor;
use foundry_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};

/// http:request <method> <url>
pub struct HttpRequestCommand;

#[async_trait]
impl FoundryCommand for HttpRequestCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &CommandDescriptor {
            name: "http:request".to_string(),
            description: "Make an HTTP request (GET, POST, etc.)".to_string(),
            usage: "http:request <METHOD> <URL> [--header KEY=VALUE] [--body JSON]".to_string(),
            examples: vec![
                "http:request GET https://api.example.com/users".to_string(),
                "http:request POST https://api.example.com/users --body '{\"name\":\"John\"}'".to_string(),
            ],
        }
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
