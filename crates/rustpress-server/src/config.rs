//! Server configuration: an optional TOML file, overridden by environment variables.
//!
//! ```toml
//! database_url = "mysql://wordpress:secret@127.0.0.1:3306/wordpress"
//! table_prefix = "wp_"
//! wp_root      = "/var/www/html"
//! listen       = "127.0.0.1:3000"
//! reload_secs  = 60
//! ```
//!
//! Environment: `RUSTPRESS_CONFIG` (path of the TOML file), `RUSTPRESS_DATABASE_URL`,
//! `RUSTPRESS_TABLE_PREFIX`, `RUSTPRESS_WP_ROOT`, `RUSTPRESS_LISTEN`, `RUSTPRESS_RELOAD_SECS`.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub database_url: String,
    pub table_prefix: String,
    /// The WordPress directory (`wp-content/`, `wp-includes/`).
    pub wp_root: PathBuf,
    pub listen: String,
    /// Options and theme are re-read from the database after this many seconds.
    pub reload_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: String::new(),
            table_prefix: "wp_".to_owned(),
            wp_root: PathBuf::from("."),
            listen: "127.0.0.1:3000".to_owned(),
            reload_secs: 60,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file `{0}`: {1}")]
    Read(String, std::io::Error),
    #[error("invalid config file `{0}`: {1}")]
    Parse(String, toml::de::Error),
    #[error("`{0}` must be a number")]
    Number(&'static str),
    #[error("no database configured: set `database_url` or RUSTPRESS_DATABASE_URL")]
    NoDatabase,
}

impl Config {
    /// File first, then environment.
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = match std::env::var("RUSTPRESS_CONFIG") {
            Ok(path) => {
                let text = std::fs::read_to_string(&path).map_err(|e| ConfigError::Read(path.clone(), e))?;
                toml::from_str(&text).map_err(|e| ConfigError::Parse(path, e))?
            }
            Err(_) => Config::default(),
        };
        if let Ok(v) = std::env::var("RUSTPRESS_DATABASE_URL") {
            config.database_url = v;
        }
        if let Ok(v) = std::env::var("RUSTPRESS_TABLE_PREFIX") {
            config.table_prefix = v;
        }
        if let Ok(v) = std::env::var("RUSTPRESS_WP_ROOT") {
            config.wp_root = v.into();
        }
        if let Ok(v) = std::env::var("RUSTPRESS_LISTEN") {
            config.listen = v;
        }
        if let Ok(v) = std::env::var("RUSTPRESS_RELOAD_SECS") {
            config.reload_secs = v.parse().map_err(|_| ConfigError::Number("RUSTPRESS_RELOAD_SECS"))?;
        }
        if config.database_url.is_empty() {
            return Err(ConfigError::NoDatabase);
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_overrides_defaults() {
        let c: Config = toml::from_str("database_url = \"mysql://x\"\nlisten = \"0.0.0.0:80\"").unwrap();
        assert_eq!(c.database_url, "mysql://x");
        assert_eq!(c.listen, "0.0.0.0:80");
        assert_eq!(c.table_prefix, "wp_");
        assert_eq!(c.reload_secs, 60);
    }
}
