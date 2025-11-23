use crate::config::{DatabaseConfig, DatabaseDriver};
use sea_orm::DbErr;
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("datenbankverbindung fehlgeschlagen: {0}")]
    ConnectionFailed(#[source] DbErr),
    #[error("konnte datenbank-url nicht parsen: {0}")]
    UrlParseFailed(#[source] url::ParseError),
    #[error("postgres datenbankname konnte nicht ermittelt werden")]
    MissingDatabaseName,
    #[error("postgres datenbank konnte nicht erstellt werden: {0}")]
    CreateDatabaseFailed(#[source] DbErr),
}

pub async fn connect(config: &DatabaseConfig) -> Result<DatabaseConnection, ConnectionError> {
    match Database::connect(&config.url).await {
        Ok(conn) => Ok(conn),
        Err(err) => {
            if matches!(config.driver, DatabaseDriver::Postgres) && is_missing_database(&err) {
                ensure_postgres_database(&config.url).await?;
                Database::connect(&config.url)
                    .await
                    .map_err(ConnectionError::ConnectionFailed)
            } else {
                Err(ConnectionError::ConnectionFailed(err))
            }
        }
    }
}

fn is_missing_database(err: &DbErr) -> bool {
    let message = err.to_string().to_lowercase();
    message.contains("does not exist") || message.contains("unknown database")
}

fn database_already_exists(err: &DbErr) -> bool {
    let message = err.to_string().to_lowercase();
    message.contains("already exists")
}

async fn ensure_postgres_database(url: &str) -> Result<(), ConnectionError> {
    let mut parsed = Url::parse(url).map_err(ConnectionError::UrlParseFailed)?;
    let path = parsed.path().trim_start_matches('/');
    let database_name = path
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or(ConnectionError::MissingDatabaseName)?
        .to_string();

    parsed.set_path("/postgres");
    let admin_url = parsed.as_ref().to_string();

    let admin_conn = Database::connect(&admin_url)
        .await
        .map_err(ConnectionError::ConnectionFailed)?;

    let escaped_name = escape_identifier(&database_name);
    let statement = Statement::from_string(
        DatabaseBackend::Postgres,
        format!(r#"CREATE DATABASE "{}""#, escaped_name),
    );

    match admin_conn.execute(statement).await {
        Ok(_) => Ok(()),
        Err(err) if database_already_exists(&err) => Ok(()),
        Err(err) => Err(ConnectionError::CreateDatabaseFailed(err)),
    }
}

fn escape_identifier(value: &str) -> String {
    value.replace('"', "\"\"")
}
