//! Performance Metrics Commands

use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_infra::PerformanceMonitor;
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;

/// Metrics Report Command
pub struct MetricsReportCommand {
    descriptor: CommandDescriptor,
}

impl MetricsReportCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("metrics.report", "metrics:report")
                .summary("Performance Metrics Report")
                .description("Zeigt einen detaillierten Performance-Report an")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for MetricsReportCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // In Production würde dies auf einen shared PerformanceMonitor zugreifen
        let monitor = PerformanceMonitor::new();
        let report = monitor.report().await;

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(format!(
                "Performance Report: {} Metriken gesammelt",
                report.total_metrics
            )),
            data: Some(serde_json::to_value(&report).unwrap()),
            error: None,
        })
    }
}

impl Default for MetricsReportCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics Clear Command
pub struct MetricsClearCommand {
    descriptor: CommandDescriptor,
}

impl MetricsClearCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("metrics.clear", "metrics:clear")
                .summary("Löscht alle Metriken")
                .description("Löscht alle gesammelten Performance-Metriken")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for MetricsClearCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let monitor = PerformanceMonitor::new();
        monitor.collector().clear().await;

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some("Alle Metriken wurden gelöscht".to_string()),
            data: None,
            error: None,
        })
    }
}

impl Default for MetricsClearCommand {
    fn default() -> Self {
        Self::new()
    }
}
