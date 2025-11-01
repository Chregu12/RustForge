//! Broadcast events

use serde::{Serialize, Deserialize};

pub trait BroadcastEvent: Serialize {
    fn channel(&self) -> String;
    fn event_name(&self) -> String;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelEvent {
    pub model: String,
    pub action: String,
    pub data: serde_json::Value,
}

impl BroadcastEvent for ModelEvent {
    fn channel(&self) -> String {
        format!("model.{}", self.model)
    }

    fn event_name(&self) -> String {
        self.action.clone()
    }
}
