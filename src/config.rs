use std::env;

use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Config {
    pub db_path: String,
    pub work_start: u32,
    pub work_end: u32,
    pub interval_hours: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            db_path: env::var("DB_PATH").unwrap_or_else(|_| "wrklog.db".into()),
            work_start: var_or("WORK_START", 10)?,
            work_end: var_or("WORK_END", 19)?,
            interval_hours: var_or("INTERVAL_HOURS", 2)?.max(1),
        })
    }
}

fn var_or(name: &str, default: u32) -> Result<u32> {
    match env::var(name) {
        Ok(v) => v
            .parse()
            .with_context(|| format!("{name} must be a number")),
        Err(_) => Ok(default),
    }
}
