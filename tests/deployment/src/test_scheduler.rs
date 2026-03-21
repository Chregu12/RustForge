//! Deployment tests for rf-scheduler

#[cfg(test)]
mod tests {
    use rf_scheduler::{Scheduler, Task, TaskBuilder};
    use async_trait::async_trait;

    struct CleanupTask;

    #[async_trait]
    impl Task for CleanupTask {
        async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn name(&self) -> &str {
            "cleanup"
        }

        fn prevent_overlap(&self) -> bool {
            true
        }
    }

    struct ReportTask;

    #[async_trait]
    impl Task for ReportTask {
        async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn name(&self) -> &str {
            "report"
        }
    }

    // ── Scheduler ────────────────────────────────────────────────

    #[tokio::test]
    async fn scheduler_schedule_cron() {
        let scheduler = Scheduler::new();
        scheduler.schedule("0 * * * *", CleanupTask).await.expect("schedule");
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn scheduler_hourly() {
        let scheduler = Scheduler::new();
        scheduler.hourly(CleanupTask).await.expect("hourly");
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn scheduler_daily() {
        let scheduler = Scheduler::new();
        scheduler.daily(CleanupTask).await.expect("daily");
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn scheduler_every_minutes() {
        let scheduler = Scheduler::new();
        scheduler.every_minutes(5, CleanupTask).await.expect("every 5m");
        assert_eq!(scheduler.task_count().await, 1);
    }

    #[tokio::test]
    async fn scheduler_multiple_tasks() {
        let scheduler = Scheduler::new();
        scheduler.hourly(CleanupTask).await.expect("hourly");
        scheduler.daily(ReportTask).await.expect("daily");
        assert_eq!(scheduler.task_count().await, 2);
    }

    #[tokio::test]
    async fn scheduler_invalid_cron() {
        let scheduler = Scheduler::new();
        let result = scheduler.schedule("invalid cron", CleanupTask).await;
        assert!(result.is_err());
    }

    // ── TaskBuilder ──────────────────────────────────────────────

    #[test]
    fn task_builder_daily() {
        let builder = TaskBuilder::new().name("cleanup").daily();
        assert!(builder.cron().is_some());
    }

    #[test]
    fn task_builder_hourly() {
        let builder = TaskBuilder::new().name("report").hourly();
        assert!(builder.cron().is_some());
    }

    #[test]
    fn task_builder_weekly() {
        let builder = TaskBuilder::new().name("weekly").weekly();
        assert!(builder.cron().is_some());
    }

    #[test]
    fn task_builder_monthly() {
        let builder = TaskBuilder::new().name("monthly").monthly();
        assert!(builder.cron().is_some());
    }

    #[test]
    fn task_builder_every_five_minutes() {
        let builder = TaskBuilder::new().name("frequent").every_five_minutes();
        assert!(builder.cron().is_some());
    }

    #[test]
    fn task_builder_at_time() {
        let builder = TaskBuilder::new().name("timed").daily().at("14:30");
        assert!(builder.cron().is_some());
    }

    #[test]
    fn task_builder_weekdays() {
        let builder = TaskBuilder::new().name("workday").daily().weekdays();
        assert!(builder.cron().is_some());
    }

    #[test]
    fn task_builder_between() {
        let builder = TaskBuilder::new()
            .name("business_hours")
            .hourly()
            .between("09:00", "17:00");
        assert!(builder.cron().is_some());
    }
}
