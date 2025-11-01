//! Broadcasting implementation

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use crate::{Channel, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastMessage {
    pub channel: String,
    pub event: String,
    pub data: serde_json::Value,
}

pub struct Broadcaster {
    channels: Arc<RwLock<HashMap<String, Channel>>>,
}

impl Broadcaster {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn channel(&self, name: &str) -> Result<Channel> {
        let channels = self.channels.read().unwrap();
        channels.get(name).cloned()
            .ok_or_else(|| crate::BroadcastError::ChannelNotFound(name.to_string()))
    }

    pub fn create_channel(&self, name: String) -> Channel {
        let channel = Channel::new(name.clone());
        self.channels.write().unwrap().insert(name, channel.clone());
        channel
    }

    pub async fn broadcast(&self, message: BroadcastMessage) -> Result<()> {
        let channel = self.channel(&message.channel)?;
        channel.send(message.event, message.data).await
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}
