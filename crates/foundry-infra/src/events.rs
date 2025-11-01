use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use foundry_plugins::{CommandError, DomainEvent, EventPort};
use tracing::info;

#[derive(Clone, Default)]
pub struct InMemoryEventBus {
    events: Arc<Mutex<Vec<DomainEvent>>>,
}

impl InMemoryEventBus {
    pub fn events(&self) -> Vec<DomainEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl EventPort for InMemoryEventBus {
    async fn publish(&self, event: DomainEvent) -> Result<(), CommandError> {
        info!(name = %event.name, "Domain event published");
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}
