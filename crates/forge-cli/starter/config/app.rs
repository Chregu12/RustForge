use anyhow::{Context, Result};
use std::{env, io};

pub struct Settings {
    pub host: String,
    pub port: u16,
    pub database_url: String,
}

impl Settings {
    pub fn load() -> Result<Self> {
        match dotenvy::from_path(".env") {
            Ok(()) => {}
            Err(dotenvy::Error::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Load application .env"),
        }
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8000".into())
                .parse()
                .context("PORT must be a valid TCP port")?,
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://database/database.sqlite?mode=rwc".into()),
        })
    }
}
