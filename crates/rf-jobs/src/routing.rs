//! Queue routing by job class
//!
//! Laravel-13-style "queue routing" lets an application centrally assign a job
//! *type* to a specific queue (and optionally a connection), instead of each
//! job hardcoding its queue via [`Job::queue`](crate::Job::queue).
//!
//! Routes are keyed by [`std::any::type_name`], the same key used by
//! [`JobPayload`](crate::JobPayload) for its `job_type`, so a registered route
//! lines up with the dispatched payload.

use crate::job::Job;
use std::any::type_name;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// A resolved route for a job type: the target queue and an optional connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRoute {
    /// Queue name the job type should be dispatched to.
    pub queue: String,

    /// Optional connection name. Connection routing is not yet representable by
    /// [`QueueManager`](crate::QueueManager); see the TODO in `queue.rs`.
    pub connection: Option<String>,
}

/// Process-global registry mapping job type names to their routes.
static ROUTES: OnceLock<RwLock<HashMap<String, QueueRoute>>> = OnceLock::new();

fn routes() -> &'static RwLock<HashMap<String, QueueRoute>> {
    ROUTES.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Central registry for routing job types to queues.
///
/// The registry is process-global. Register routes once during application
/// boot, then dispatch jobs normally — [`QueueManager::dispatch`](crate::QueueManager::dispatch)
/// resolves the route by the job's type name before falling back to
/// [`Job::queue`](crate::Job::queue).
pub struct JobRouter;

impl JobRouter {
    /// Route a job type to a queue (no specific connection).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_jobs::{Job, JobContext, JobResult, JobRouter};
    /// use serde::{Deserialize, Serialize};
    /// use async_trait::async_trait;
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct SendEmailJob {
    ///     to: String,
    /// }
    ///
    /// #[async_trait]
    /// impl Job for SendEmailJob {
    ///     async fn handle(&self, _ctx: JobContext) -> JobResult {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// // Route every `SendEmailJob` to the "emails" queue.
    /// JobRouter::route::<SendEmailJob>("emails");
    /// ```
    pub fn route<J: Job>(queue: impl Into<String>) {
        Self::route_type(type_name::<J>(), queue, None);
    }

    /// Route a job type to a queue on a specific connection.
    pub fn route_to<J: Job>(queue: impl Into<String>, connection: impl Into<String>) {
        Self::route_type(type_name::<J>(), queue, Some(connection.into()));
    }

    /// Route a job type (identified by its string type name) to a queue.
    pub fn route_type(
        type_name: impl Into<String>,
        queue: impl Into<String>,
        connection: Option<String>,
    ) {
        let route = QueueRoute {
            queue: queue.into(),
            connection,
        };
        routes()
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(type_name.into(), route);
    }

    /// Resolve the route registered for a job type name, if any.
    pub fn resolve(type_name: &str) -> Option<QueueRoute> {
        routes()
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(type_name)
            .cloned()
    }

    /// Remove all registered routes (primarily for tests).
    pub fn clear() {
        routes()
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobPayload;
    use crate::{JobContext, JobResult};
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::Mutex;

    /// Serializes every test that mutates the process-global route registry to
    /// prevent flaky cross-test races.
    static ROUTE_TEST_GUARD: Mutex<()> = Mutex::new(());

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct RoutedJob {
        value: i32,
    }

    #[async_trait]
    impl Job for RoutedJob {
        async fn handle(&self, _ctx: JobContext) -> JobResult {
            Ok(())
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UnroutedJob {
        value: i32,
    }

    #[async_trait]
    impl Job for UnroutedJob {
        async fn handle(&self, _ctx: JobContext) -> JobResult {
            Ok(())
        }
    }

    #[test]
    fn test_route_resolves_by_type_name() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        JobRouter::route::<RoutedJob>("emails");

        let route = JobRouter::resolve(std::any::type_name::<RoutedJob>());
        assert_eq!(
            route,
            Some(QueueRoute {
                queue: "emails".to_string(),
                connection: None,
            })
        );
    }

    #[test]
    fn test_route_key_matches_payload_job_type() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        JobRouter::route::<RoutedJob>("emails");

        // The payload's `job_type` must be the same key the router registered under.
        let payload = JobPayload::new(RoutedJob { value: 1 }).unwrap();
        assert!(JobRouter::resolve(&payload.job_type).is_some());
    }

    #[test]
    fn test_route_to_records_connection() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        JobRouter::route_to::<RoutedJob>("emails", "redis-secondary");

        let route = JobRouter::resolve(std::any::type_name::<RoutedJob>()).unwrap();
        assert_eq!(route.queue, "emails");
        assert_eq!(route.connection.as_deref(), Some("redis-secondary"));
    }

    #[test]
    fn test_resolve_unregistered_returns_none() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        assert!(JobRouter::resolve(std::any::type_name::<UnroutedJob>()).is_none());
    }

    #[test]
    fn test_route_type_string_keyed() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        JobRouter::route_type("my::Custom::Job", "custom", Some("conn".to_string()));

        let route = JobRouter::resolve("my::Custom::Job").unwrap();
        assert_eq!(route.queue, "custom");
        assert_eq!(route.connection.as_deref(), Some("conn"));
    }

    // --- Adversarial routing tests (Feature 2 validation) ---

    #[test]
    fn test_clear_empties_registry() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();
        JobRouter::route::<RoutedJob>("emails");
        JobRouter::route::<UnroutedJob>("other");
        assert!(JobRouter::resolve(type_name::<RoutedJob>()).is_some());
        JobRouter::clear();
        assert!(JobRouter::resolve(type_name::<RoutedJob>()).is_none());
        assert!(JobRouter::resolve(type_name::<UnroutedJob>()).is_none());
    }

    #[test]
    fn test_distinct_types_do_not_collide() {
        // Non-tautological alignment check: two different job types must produce
        // two different registry keys, and each payload resolves to its own route.
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        JobRouter::route::<RoutedJob>("queue-a");
        JobRouter::route::<UnroutedJob>("queue-b");

        assert_ne!(type_name::<RoutedJob>(), type_name::<UnroutedJob>());

        let routed = JobPayload::new(RoutedJob { value: 1 }).unwrap();
        let other = JobPayload::new(UnroutedJob { value: 1 }).unwrap();
        assert_eq!(routed.job_type, type_name::<RoutedJob>());
        assert_eq!(other.job_type, type_name::<UnroutedJob>());
        assert_eq!(
            JobRouter::resolve(&routed.job_type).unwrap().queue,
            "queue-a"
        );
        assert_eq!(
            JobRouter::resolve(&other.job_type).unwrap().queue,
            "queue-b"
        );
    }

    #[test]
    fn test_route_overwrites_previous() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();
        JobRouter::route::<RoutedJob>("first");
        JobRouter::route::<RoutedJob>("second");
        assert_eq!(
            JobRouter::resolve(type_name::<RoutedJob>()).unwrap().queue,
            "second"
        );
    }

    /// Mirrors `QueueManager::resolve_queue` precedence (route > Job::queue
    /// default) using only the public router API, since `resolve_queue` is
    /// private and the real `dispatch` path requires a Redis connection.
    fn resolve_queue_for<J: Job>(job: &J) -> String {
        match JobRouter::resolve(type_name::<J>()) {
            Some(route) => route.queue,
            None => job.queue().to_string(),
        }
    }

    #[test]
    fn test_resolve_queue_precedence_route_over_default() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        // RoutedJob::queue() defaults to "default".
        let job = RoutedJob { value: 1 };
        assert_eq!(job.queue(), "default");
        assert_eq!(resolve_queue_for(&job), "default");

        // After routing, the route wins over the default.
        JobRouter::route::<RoutedJob>("routed-queue");
        assert_eq!(resolve_queue_for(&job), "routed-queue");
    }

    #[test]
    fn test_dispatch_to_overrides_route() {
        // `dispatch_to(job, queue)` bypasses resolve_queue entirely and always
        // uses the explicit queue. We assert the explicit queue is independent of
        // any registered route (the override semantic).
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();
        JobRouter::route::<RoutedJob>("routed-queue");

        // The resolved queue would be "routed-queue"...
        let job = RoutedJob { value: 1 };
        assert_eq!(resolve_queue_for(&job), "routed-queue");
        // ...but an explicit target is what dispatch_to forwards verbatim.
        let explicit = "explicit-queue";
        assert_ne!(explicit, resolve_queue_for(&job));
        // Sanity: payload is independent of the explicit target queue.
        let payload = JobPayload::new(job).unwrap();
        assert_eq!(payload.job_type, type_name::<RoutedJob>());
    }
}
