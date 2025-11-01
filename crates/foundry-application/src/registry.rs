use crate::ApplicationError;
use foundry_domain::CommandDescriptor;
use foundry_plugins::DynCommand;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
    pub fn register(&self, command: DynCommand) -> Result<(), ApplicationError> {
        let descriptor = command.descriptor().clone();
        let mut inner = self.inner.lock().expect("registry poisoned");
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

    pub fn resolve(&self, command: &str) -> Option<DynCommand> {
        let inner = self.inner.lock().expect("registry poisoned");
        let key = command.to_lowercase();
        let index = inner.lookup.get(&key)?;
        inner.commands.get(*index).cloned()
    }

    pub fn descriptors(&self) -> Vec<CommandDescriptor> {
        let inner = self.inner.lock().expect("registry poisoned");
        inner
            .commands
            .iter()
            .map(|cmd| cmd.descriptor().clone())
            .collect()
    }

    pub fn len(&self) -> usize {
        let inner = self.inner.lock().expect("registry poisoned");
        inner.commands.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
