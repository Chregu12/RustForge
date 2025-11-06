use foundry_plugins::CommandError;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ApplicationError {
    #[error("command '{0}' ist bereits registriert")]
    CommandAlreadyRegistered(String),
    #[error("command '{0}' nicht gefunden")]
    CommandNotFound(String),
    #[error("command execution failed: {0}")]
    CommandExecution(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Registry corrupted: lock poisoned")]
    RegistryCorrupted,
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
}

impl From<CommandError> for ApplicationError {
    fn from(err: CommandError) -> Self {
        ApplicationError::CommandExecution(err.to_string())
    }
}
