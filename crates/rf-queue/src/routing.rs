use std::collections::HashMap;
use std::time::Duration;

/// Configuration for routing a job type to a specific queue connection.
#[derive(Debug, Clone)]
pub struct QueueRoute {
    pub connection: String,
    pub queue: String,
    pub delay: Option<Duration>,
    pub max_tries: Option<u32>,
    pub timeout: Option<Duration>,
}

impl QueueRoute {
    pub fn new(connection: impl Into<String>, queue: impl Into<String>) -> Self {
        Self {
            connection: connection.into(),
            queue: queue.into(),
            delay: None,
            max_tries: None,
            timeout: None,
        }
    }

    pub fn delay(mut self, d: Duration) -> Self {
        self.delay = Some(d);
        self
    }

    pub fn max_tries(mut self, n: u32) -> Self {
        self.max_tries = Some(n);
        self
    }

    pub fn timeout(mut self, t: Duration) -> Self {
        self.timeout = Some(t);
        self
    }
}

/// Routes job types to specific queue connections and queues.
pub struct QueueRouter {
    routes: HashMap<String, QueueRoute>,
    default_connection: String,
    default_queue: String,
}

impl QueueRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            default_connection: "default".to_string(),
            default_queue: "default".to_string(),
        }
    }

    /// Register a route for a specific job type.
    pub fn route(&mut self, job_type: &str, route: QueueRoute) -> &mut Self {
        self.routes.insert(job_type.to_string(), route);
        self
    }

    /// Resolve the route for a job type, falling back to the default if not registered.
    pub fn resolve(&self, job_type: &str) -> QueueRoute {
        if let Some(route) = self.routes.get(job_type) {
            route.clone()
        } else {
            QueueRoute::new(&self.default_connection, &self.default_queue)
        }
    }

    /// Set the default connection used when no specific route is configured.
    pub fn default_connection(mut self, conn: impl Into<String>) -> Self {
        self.default_connection = conn.into();
        self
    }

    /// Set the default queue used when no specific route is configured.
    pub fn default_queue(mut self, queue: impl Into<String>) -> Self {
        self.default_queue = queue.into();
        self
    }

    /// Check whether a route is registered for the given job type.
    pub fn has_route(&self, job_type: &str) -> bool {
        self.routes.contains_key(job_type)
    }

    /// Return a reference to all registered routes.
    pub fn routes(&self) -> &HashMap<String, QueueRoute> {
        &self.routes
    }
}

impl Default for QueueRouter {
    fn default() -> Self {
        Self::new()
    }
}
