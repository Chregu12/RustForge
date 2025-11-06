use crate::ApplicationError;
use foundry_domain::CommandDescriptor;
use foundry_plugins::DynCommand;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, instrument};

#[derive(Clone, Default)]
pub struct CommandRegistry {
    inner: Arc<Mutex<RegistryState>>,
}

#[derive(Default)]
struct RegistryState {
    commands: Vec<DynCommand>,
    lookup: HashMap<String, usize>,
}


impl CommandRegistry {
    #[instrument(skip(self, command), fields(command_name = %command.descriptor().name))]
    pub fn register(&self, command: DynCommand) -> Result<(), ApplicationError> {
        let descriptor = command.descriptor().clone();
        let mut inner = self.inner.lock()
            .map_err(|_| ApplicationError::RegistryCorrupted)?;
        let index = inner.commands.len();
        let mut keys = Vec::new();
        keys.push(descriptor.id.as_str().to_lowercase());
        keys.push(descriptor.name.to_lowercase());
        for alias in &descriptor.aliases {
            keys.push(alias.to_lowercase());
        }

        for key in &keys {
            if inner.lookup.contains_key(key) {
                return Err(ApplicationError::CommandAlreadyRegistered(
                    descriptor.name.clone(),
                ));
            }
        }

        inner.commands.push(command);
        for key in keys {
            inner.lookup.insert(key, index);
        }

        Ok(())
    }

    #[instrument(skip(self), fields(command))]
    pub fn resolve(&self, command: &str) -> Result<Option<DynCommand>, ApplicationError> {
        let inner = self.inner.lock()
            .map_err(|_| ApplicationError::RegistryCorrupted)?;
        let key = command.to_lowercase();
        let index = inner.lookup.get(&key);
        let result = index.and_then(|idx| inner.commands.get(*idx).cloned());

        if result.is_some() {
            debug!("Command resolved successfully");
        } else {
            debug!("Command not found in registry");
        }

        Ok(result)
    }

    pub fn descriptors(&self) -> Result<Vec<CommandDescriptor>, ApplicationError> {
        let inner = self.inner.lock()
            .map_err(|_| ApplicationError::RegistryCorrupted)?;
        Ok(inner
            .commands
            .iter()
            .map(|cmd| cmd.descriptor().clone())
            .collect())
    }

    pub fn len(&self) -> Result<usize, ApplicationError> {
        let inner = self.inner.lock()
            .map_err(|_| ApplicationError::RegistryCorrupted)?;
        Ok(inner.commands.len())
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> Result<bool, ApplicationError> {
        Ok(self.len()? == 0)
    }
}
