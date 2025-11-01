use foundry_plugins::CommandError;

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("command '{0}' ist bereits registriert")]
    CommandAlreadyRegistered(String),
    #[error("command '{0}' nicht gefunden")]
    CommandNotFound(String),
    #[error("command execution failed")]
    CommandExecution(#[source] CommandError),
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl From<CommandError> for ApplicationError {
    fn from(err: CommandError) -> Self {
        ApplicationError::CommandExecution(err)
    }
}
