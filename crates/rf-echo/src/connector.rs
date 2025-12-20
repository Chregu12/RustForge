//! Connector configurations for different broadcasting providers

use serde::{Deserialize, Serialize};

/// Connector type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Connector {
    /// Pusher channels
    Pusher {
        key: String,
        cluster: String,
    },
    /// Pusher with custom host (for Soketi, etc.)
    PusherCustom {
        key: String,
        host: String,
        port: u16,
        use_tls: bool,
    },
    /// Ably
    Ably {
        key: String,
    },
    /// Custom WebSocket server
    WebSocket {
        url: String,
    },
    /// Laravel WebSockets / Soketi
    Soketi {
        key: String,
        host: String,
        port: u16,
        use_tls: bool,
    },
}

impl Default for Connector {
    fn default() -> Self {
        Self::Pusher {
            key: String::new(),
            cluster: "mt1".to_string(),
        }
    }
}

impl Connector {
    /// Get the WebSocket URL for this connector
    pub fn websocket_url(&self) -> String {
        match self {
            Connector::Pusher { key, cluster } => {
                format!(
                    "wss://ws-{}.pusher.com/app/{}?protocol=7&client=rf-echo&version=1.0",
                    cluster, key
                )
            }
            Connector::PusherCustom {
                key,
                host,
                port,
                use_tls,
            } => {
                let protocol = if *use_tls { "wss" } else { "ws" };
                format!(
                    "{}://{}:{}/app/{}?protocol=7&client=rf-echo&version=1.0",
                    protocol, host, port, key
                )
            }
            Connector::Ably { key } => {
                format!(
                    "wss://realtime.ably.io/?key={}&format=json&heartbeats=true",
                    key
                )
            }
            Connector::WebSocket { url } => url.clone(),
            Connector::Soketi {
                key,
                host,
                port,
                use_tls,
            } => {
                let protocol = if *use_tls { "wss" } else { "ws" };
                format!(
                    "{}://{}:{}/app/{}?protocol=7&client=rf-echo&version=1.0",
                    protocol, host, port, key
                )
            }
        }
    }

    /// Create a Pusher connector
    pub fn pusher(key: impl Into<String>, cluster: impl Into<String>) -> Self {
        Self::Pusher {
            key: key.into(),
            cluster: cluster.into(),
        }
    }

    /// Create a custom Pusher connector (for self-hosted)
    pub fn pusher_custom(
        key: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        use_tls: bool,
    ) -> Self {
        Self::PusherCustom {
            key: key.into(),
            host: host.into(),
            port,
            use_tls,
        }
    }

    /// Create an Ably connector
    pub fn ably(key: impl Into<String>) -> Self {
        Self::Ably { key: key.into() }
    }

    /// Create a custom WebSocket connector
    pub fn websocket(url: impl Into<String>) -> Self {
        Self::WebSocket { url: url.into() }
    }

    /// Create a Soketi connector
    pub fn soketi(
        key: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        use_tls: bool,
    ) -> Self {
        Self::Soketi {
            key: key.into(),
            host: host.into(),
            port,
            use_tls,
        }
    }
}

/// Connector configuration options
#[derive(Debug, Clone, Default)]
pub struct ConnectorConfig {
    /// Force TLS
    pub force_tls: bool,
    /// Custom path
    pub path: Option<String>,
    /// Enable statistics
    pub enable_stats: bool,
    /// Activity timeout in seconds
    pub activity_timeout: u32,
    /// Pong timeout in seconds
    pub pong_timeout: u32,
}

impl ConnectorConfig {
    pub fn new() -> Self {
        Self {
            force_tls: false,
            path: None,
            enable_stats: true,
            activity_timeout: 120,
            pong_timeout: 30,
        }
    }

    pub fn force_tls(mut self, force: bool) -> Self {
        self.force_tls = force;
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn enable_stats(mut self, enable: bool) -> Self {
        self.enable_stats = enable;
        self
    }

    pub fn activity_timeout(mut self, timeout: u32) -> Self {
        self.activity_timeout = timeout;
        self
    }

    pub fn pong_timeout(mut self, timeout: u32) -> Self {
        self.pong_timeout = timeout;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pusher_url() {
        let connector = Connector::pusher("my-key", "eu");
        let url = connector.websocket_url();
        assert!(url.contains("ws-eu.pusher.com"));
        assert!(url.contains("my-key"));
    }

    #[test]
    fn test_custom_pusher_url() {
        let connector = Connector::pusher_custom("key", "localhost", 6001, false);
        let url = connector.websocket_url();
        assert!(url.starts_with("ws://"));
        assert!(url.contains("localhost:6001"));
    }

    #[test]
    fn test_soketi_url() {
        let connector = Connector::soketi("key", "soketi.example.com", 443, true);
        let url = connector.websocket_url();
        assert!(url.starts_with("wss://"));
        assert!(url.contains("soketi.example.com"));
    }
}
