use std::path::Path;

use anyhow::{Context, Result};
use config::{Config, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]

pub struct MySQLOptions {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub db: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub mysql: MySQLOptions,
}

pub fn load_settings<P: AsRef<Path>>(path: P) -> Result<Settings> {
    let path = path.as_ref();
    let c = Config::builder()
        .add_source(File::from(path))
        .build()
        .context(format!(
            "failed to build configuration from path: {:?}",
            path
        ))?;

    c.try_deserialize().context(format!(
        "failed to deserialize configurations from path: {:?}",
        path
    ))
}
