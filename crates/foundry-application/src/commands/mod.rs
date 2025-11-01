mod about;
mod auth;
mod cache;
mod config;
mod database;
mod database_setup;
mod env;
mod event;
mod graphql;
mod key;
mod list;
mod maintenance;
mod optimize;
mod queue;
mod queue_failed;
mod route;
mod scaffolding;
mod schedule;
mod seed;
mod serve;
mod test;
mod tinker;
mod websocket;
mod websocket_stats;
pub mod tier3;

pub use about::AboutCommand;
pub use auth::{
    AssignRoleCommand, CheckPermissionsCommand, GenerateTokenCommand, ListUsersCommand,
    MakePermissionCommand, MakeRoleCommand, MakeUserCommand,
};
pub use cache::CacheClearCommand;
pub use config::ConfigClearCommand;
pub use database::{
    DbDumpCommand, DbShowCommand, MigrateCommand, MigrateFreshCommand, MigrateRefreshCommand,
    MigrateRollbackCommand, MigrateSeedCommand, SchemaDumpCommand,
};
pub use database_setup::DatabaseCreateCommand;
pub use env::EnvCommand;
pub use event::EventListCommand;
pub use key::{KeyGenerateCommand, KeyShowCommand};
pub use list::ListCommand;
pub use maintenance::{DownCommand, UpCommand};
pub use optimize::OptimizeCommand;
pub use queue::QueueWorkCommand;
pub use queue_failed::QueueFailedCommand;
pub use route::RouteListCommand;
pub use scaffolding::{
    InstallPackageCommand, MakeAuthCommand, MakeCommandCommand, MakeControllerCommand, MakeEventCommand, MakeJobCommand, MakeListenerCommand, MakeMiddlewareCommand, MakeMigrationCommand, MakeModelCommand, MakeRequestCommand,
};
pub use tier3::{
    AdminPublishCommand, AdminResourceCommand, ExportCsvCommand, ExportExcelCommand,
    ExportPdfCommand, HttpRequestCommand, MakeFormCommand,
};
pub use tier3::factory::{MakeFactoryCommand, MakeSeederCommand};
pub use tinker::TinkerCommand;
pub use schedule::{ScheduleListCommand, ScheduleRunCommand};
pub use seed::SeedCommand;
pub use serve::ServeCommand;
pub use test::TestCommand;
pub use websocket::WebSocketCommand;
pub use websocket_stats::WebSocketStatsCommand;
pub use graphql::MakeGraphQLTypeCommand;

use crate::{ApplicationError, CommandRegistry};
use std::sync::Arc;

pub struct BootstrapCommands;

impl BootstrapCommands {
    pub fn register_all(registry: &CommandRegistry) -> Result<(), ApplicationError> {
        let list = Arc::new(ListCommand::new(registry.clone()));
        registry.register(list)?;

        let env = Arc::new(EnvCommand::default());
        registry.register(env)?;

        let about = Arc::new(AboutCommand::default());
        registry.register(about)?;

        let down = Arc::new(DownCommand::default());
        registry.register(down)?;

        let up = Arc::new(UpCommand::default());
        registry.register(up)?;

        let make_model = Arc::new(MakeModelCommand::default());
        registry.register(make_model)?;

        let make_migration = Arc::new(MakeMigrationCommand::default());
        registry.register(make_migration)?;

        let make_controller = Arc::new(MakeControllerCommand::default());
        registry.register(make_controller)?;

        let make_middleware = Arc::new(MakeMiddlewareCommand::default());
        registry.register(make_middleware)?;

        let make_seeder = Arc::new(MakeSeederCommand::default());
        registry.register(make_seeder)?;

        let make_request = Arc::new(MakeRequestCommand::default());
        registry.register(make_request)?;

        let make_job = Arc::new(MakeJobCommand::default());
        registry.register(make_job)?;

        let make_factory = Arc::new(MakeFactoryCommand::default());
        registry.register(make_factory)?;

        let make_event = Arc::new(MakeEventCommand::default());
        registry.register(make_event)?;

        let make_listener = Arc::new(MakeListenerCommand::default());
        registry.register(make_listener)?;

        let make_command = Arc::new(MakeCommandCommand::default());
        registry.register(make_command)?;

        let migrate = Arc::new(MigrateCommand::default());
        registry.register(migrate)?;

        let rollback = Arc::new(MigrateRollbackCommand::default());
        registry.register(rollback)?;

        let migrate_seed = Arc::new(MigrateSeedCommand::default());
        registry.register(migrate_seed)?;

        let migrate_refresh = Arc::new(MigrateRefreshCommand::default());
        registry.register(migrate_refresh)?;

        let migrate_fresh = Arc::new(MigrateFreshCommand::default());
        registry.register(migrate_fresh)?;

        let db_show = Arc::new(DbShowCommand::default());
        registry.register(db_show)?;

        let db_dump = Arc::new(DbDumpCommand::default());
        registry.register(db_dump)?;

        let schema_dump = Arc::new(SchemaDumpCommand::default());
        registry.register(schema_dump)?;

        let seed = Arc::new(SeedCommand::default());
        registry.register(seed)?;

        let serve = Arc::new(ServeCommand::new());
        registry.register(serve)?;

        let route_list = Arc::new(RouteListCommand::new());
        registry.register(route_list)?;

        let cache_clear = Arc::new(CacheClearCommand::new());
        registry.register(cache_clear)?;

        let queue_work = Arc::new(QueueWorkCommand::new());
        registry.register(queue_work)?;

        let test = Arc::new(TestCommand::new());
        registry.register(test)?;

        // Sprint 7: Quick Win Commands
        let config_clear = Arc::new(ConfigClearCommand::new());
        registry.register(config_clear)?;

        let event_list = Arc::new(EventListCommand::new());
        registry.register(event_list)?;

        let queue_failed = Arc::new(QueueFailedCommand::new());
        registry.register(queue_failed)?;

        // Sprint 8: Medium Effort Commands
        let schedule_list = Arc::new(ScheduleListCommand::new());
        registry.register(schedule_list)?;

        let schedule_run = Arc::new(ScheduleRunCommand::new());
        registry.register(schedule_run)?;

        let optimize = Arc::new(OptimizeCommand::new());
        registry.register(optimize)?;

        // Sprint 9: Low Priority / Optional Commands
        let make_auth = Arc::new(MakeAuthCommand::default());
        registry.register(make_auth)?;

        let tinker = Arc::new(TinkerCommand::default());
        registry.register(tinker)?;

        let install_package = Arc::new(InstallPackageCommand::default());
        registry.register(install_package)?;

        // Sprint 10: Database Setup Commands
        let database_create = Arc::new(DatabaseCreateCommand::default());
        registry.register(database_create)?;

        // WebSocket Commands
        let websocket = Arc::new(WebSocketCommand::default());
        registry.register(websocket)?;

        let websocket_stats = Arc::new(WebSocketStatsCommand::default());
        registry.register(websocket_stats)?;

        // GraphQL Commands
        let make_graphql_type = Arc::new(MakeGraphQLTypeCommand::default());
        registry.register(make_graphql_type)?;

        // Authentication Commands
        let make_user = Arc::new(MakeUserCommand::default());
        registry.register(make_user)?;

        let list_users = Arc::new(ListUsersCommand::default());
        registry.register(list_users)?;

        let assign_role = Arc::new(AssignRoleCommand::default());
        registry.register(assign_role)?;

        let check_permissions = Arc::new(CheckPermissionsCommand::default());
        registry.register(check_permissions)?;

        let make_role = Arc::new(MakeRoleCommand::default());
        registry.register(make_role)?;

        let make_permission = Arc::new(MakePermissionCommand::default());
        registry.register(make_permission)?;

        let generate_token = Arc::new(GenerateTokenCommand::default());
        registry.register(generate_token)?;

        // Tier 3 Commands
        let admin_resource = Arc::new(AdminResourceCommand);
        registry.register(admin_resource)?;

        let admin_publish = Arc::new(AdminPublishCommand);
        registry.register(admin_publish)?;

        let export_pdf = Arc::new(ExportPdfCommand);
        registry.register(export_pdf)?;

        let export_excel = Arc::new(ExportExcelCommand);
        registry.register(export_excel)?;

        let export_csv = Arc::new(ExportCsvCommand);
        registry.register(export_csv)?;

        let make_form = Arc::new(MakeFormCommand);
        registry.register(make_form)?;

        let http_request = Arc::new(HttpRequestCommand);
        registry.register(http_request)?;

        // Key Management Commands
        let key_generate = Arc::new(KeyGenerateCommand);
        registry.register(key_generate)?;

        let key_show = Arc::new(KeyShowCommand);
        registry.register(key_show)?;

        // Tier 3 Advanced Features
        let app_down = Arc::new(foundry_maintenance::AppDownCommand);
        registry.register(app_down)?;

        let app_up = Arc::new(foundry_maintenance::AppUpCommand);
        registry.register(app_up)?;

        let health_check = Arc::new(foundry_health::HealthCheckCommand);
        registry.register(health_check)?;

        let doctor = Arc::new(foundry_health::DoctorCommand);
        registry.register(doctor)?;

        let env_validate = Arc::new(foundry_env::EnvValidateCommand);
        registry.register(env_validate)?;

        let env_reload = Arc::new(foundry_env::EnvReloadCommand);
        registry.register(env_reload)?;

        let asset_publish = Arc::new(foundry_assets::AssetPublishCommand);
        registry.register(asset_publish)?;

        Ok(())
    }
}
