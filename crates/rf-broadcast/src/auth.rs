//! WebSocket authentication

use crate::Channel;
use async_trait::async_trait;
use std::sync::Arc;

pub type UserId = String;

/// WebSocket authentication trait
#[async_trait]
pub trait WebSocketAuth: Send + Sync {
    /// Authenticate a connection with a token
    async fn authenticate(&self, token: &str) -> Result<UserId, String>;
}

/// Channel authorization trait
#[async_trait]
pub trait ChannelAuthorizer: Send + Sync {
    /// Check if user can subscribe to channel
    async fn can_subscribe(&self, user_id: &UserId, channel: &Channel) -> bool;
}

/// Allow-all authorizer (for testing/development)
pub struct AllowAllAuthorizer;

#[async_trait]
impl ChannelAuthorizer for AllowAllAuthorizer {
    async fn can_subscribe(&self, _user_id: &UserId, _channel: &Channel) -> bool {
        true
    }
}

/// Public channel authorizer (allows public, denies private/presence)
pub struct PublicOnlyAuthorizer;

#[async_trait]
impl ChannelAuthorizer for PublicOnlyAuthorizer {
    async fn can_subscribe(&self, _user_id: &UserId, channel: &Channel) -> bool {
        channel.is_public()
    }
}

/// WebSocket state with authentication
pub struct AuthenticatedWsState<A: WebSocketAuth, C: ChannelAuthorizer> {
    pub broadcaster: Arc<dyn crate::Broadcaster>,
    pub auth: Arc<A>,
    pub authorizer: Arc<C>,
}

impl<A: WebSocketAuth, C: ChannelAuthorizer> Clone for AuthenticatedWsState<A, C> {
    fn clone(&self) -> Self {
        Self {
            broadcaster: Arc::clone(&self.broadcaster),
            auth: Arc::clone(&self.auth),
            authorizer: Arc::clone(&self.authorizer),
        }
    }
}
