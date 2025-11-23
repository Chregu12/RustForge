use async_trait::async_trait;
use std::fmt;
use crate::error::Result;

/// Priority for listener execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ListenerPriority {
    Highest = 1000,
    High = 750,
    Normal = 500,
    Low = 250,
    Lowest = 0,
}

impl Default for ListenerPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Trait for event listeners
#[async_trait]
pub trait EventListener<E>: Send + Sync {
    /// Handle the event
    async fn handle(&self, event: &E) -> Result<()>;

    /// Get the listener priority
    fn priority(&self) -> ListenerPriority {
        ListenerPriority::Normal
    }

    /// Whether this listener should only run once
    fn once(&self) -> bool {
        false
    }
}

/// Wrapper for function-based listeners
pub struct FunctionListener<E, F>
where
    F: Fn(&E) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync,
{
    handler: F,
    priority: ListenerPriority,
    once: bool,
    _phantom: std::marker::PhantomData<E>,
}

impl<E, F> FunctionListener<E, F>
where
    F: Fn(&E) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            priority: ListenerPriority::Normal,
            once: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_priority(mut self, priority: ListenerPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn once(mut self) -> Self {
        self.once = true;
        self
    }
}

#[async_trait]
impl<E, F> EventListener<E> for FunctionListener<E, F>
where
    E: Send + Sync + 'static,
    F: Fn(&E) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync,
{
    async fn handle(&self, event: &E) -> Result<()> {
        (self.handler)(event).await
    }

    fn priority(&self) -> ListenerPriority {
        self.priority
    }

    fn once(&self) -> bool {
        self.once
    }
}

impl<E, F> fmt::Debug for FunctionListener<E, F>
where
    F: Fn(&E) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionListener")
            .field("priority", &self.priority)
            .field("once", &self.once)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listener_priority_ordering() {
        assert!(ListenerPriority::Highest > ListenerPriority::High);
        assert!(ListenerPriority::High > ListenerPriority::Normal);
        assert!(ListenerPriority::Normal > ListenerPriority::Low);
        assert!(ListenerPriority::Low > ListenerPriority::Lowest);
    }

    #[test]
    fn test_listener_priority_default() {
        assert_eq!(ListenerPriority::default(), ListenerPriority::Normal);
    }
}
