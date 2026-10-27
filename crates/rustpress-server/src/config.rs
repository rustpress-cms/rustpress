//! Server configuration: an optional TOML file, overridden by environment variables.
//!
//! ```toml
//! database_url = "mysql://wordpress:secret@127.0.0.1:3306/wordpress"
//! table_prefix = "wp_"
//! wp_root      = "/var/www/html"
//! listen       = "127.0.0.1:3000"
//! reload_secs  = 60
//! trusted_proxies = ["127.0.0.1"]
//! ```
//!
//! Environment: `RUSTPRESS_CONFIG` (path of the TOML file), `RUSTPRESS_DATABASE_URL`, `RUSTPRESS_ALLOW_WRITES`,
//! `RUSTPRESS_WP_CONFIG`,
//! `RUSTPRESS_TABLE_PREFIX`, `RUSTPRESS_WP_ROOT`, `RUSTPRESS_LISTEN`, `RUSTPRESS_RELOAD_SECS`,
//! `RUSTPRESS_TRUSTED_PROXIES` (addresses separated by commas). The administration reads
//! `RUSTPRESS_MAX_UPLOAD_SIZE` (bytes, `wp_max_upload_size()`, 64 MiB by default) itself.

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
    /// Signing in and changing the site need a connection that can write; without this switch RustPress
    /// only reads and the administration area stays closed.
    pub allow_writes: bool,
    /// The `wp-config.php` whose keys and salts sign the cookies (default: the one in `wp_root`).
    pub wp_config: Option<PathBuf>,
    /// The addresses of reverse proxies whose `X-Forwarded-For`/`X-Forwarded-Proto` are believed.
    pub trusted_proxies: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: String::new(),
            table_prefix: "wp_".to_owned(),
            wp_root: PathBuf::from("."),
            listen: "127.0.0.1:3000".to_owned(),
            reload_secs: 60,
            allow_writes: false,
            wp_config: None,
            trusted_proxies: Vec::new(),
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
        Self::load_with(std::env::var_os("RUSTPRESS_CONFIG").map(PathBuf::from), None)
    }

    /// Like `load()` with the config file given, and a WordPress directory for when neither the file nor the
    /// environment names one (CGI: the `DOCUMENT_ROOT` of the request).
    pub fn load_with(file: Option<PathBuf>, wp_root: Option<PathBuf>) -> Result<Self, ConfigError> {
        let mut given = toml::Table::new();
        let mut config = match file {
            Some(path) => {
                let shown = path.display().to_string();
                let text = std::fs::read_to_string(&path).map_err(|e| ConfigError::Read(shown.clone(), e))?;
                given = text.parse().unwrap_or_default();
                toml::from_str(&text).map_err(|e| ConfigError::Parse(shown, e))?
            }
            None => Config::default(),
        };
        let root_given = given.contains_key("wp_root") || std::env::var_os("RUSTPRESS_WP_ROOT").is_some();
        let prefix_given = given.contains_key("table_prefix") || std::env::var_os("RUSTPRESS_TABLE_PREFIX").is_some();
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
        if let Ok(v) = std::env::var("RUSTPRESS_ALLOW_WRITES") {
            config.allow_writes = matches!(v.as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("RUSTPRESS_WP_CONFIG") {
            config.wp_config = Some(v.into());
        }
        if let Ok(v) = std::env::var("RUSTPRESS_TRUSTED_PROXIES") {
            config.trusted_proxies = v.split(',').map(str::trim).filter(|p| !p.is_empty()).map(str::to_owned).collect();
        }
        if let Some(root) = wp_root.filter(|_| !root_given) {
            config.wp_root = root;
        }
        if config.database_url.is_empty() {
            let text = config.wp_config_path().and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default();
            if let Some((url, prefix)) = database_from_wp_config(&text) {
                config.database_url = url;
                if !prefix_given && let Some(prefix) = prefix {
                    config.table_prefix = prefix;
                }
            }
        }
        if config.database_url.is_empty() {
            return Err(ConfigError::NoDatabase);
        }
        Ok(config)
    }

    /// The `wp-config.php` of the site: the configured one, else the one in `wp_root` or the directory above it
    /// (where WordPress looks too).
    pub fn wp_config_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.wp_config {
            return Some(path.clone());
        }
        let inside = self.wp_root.join("wp-config.php");
        if inside.is_file() {
            return Some(inside);
        }
        let above = self.wp_root.parent()?.join("wp-config.php");
        above.is_file().then_some(above)
    }
}

/// The database URL and table prefix that a `wp-config.php` defines. `DB_HOST` may carry a port
/// (`db.example.com:3307`) or a socket (`localhost:/run/mysqld/mysqld.sock`) like in WordPress. Like `wpdb`
/// the connection is unencrypted unless `MYSQL_CLIENT_FLAGS` asks for `MYSQLI_CLIENT_SSL` (the TLS handshake
/// alone costs a CGI request more than the rest of starting up).
fn database_from_wp_config(config_php: &str) -> Option<(String, Option<String>)> {
    let constants = rustpress_auth::salts::parse_config_constants(config_php);
    let name = constants.get("DB_NAME").filter(|n| !n.is_empty())?;
    let user = constants.get("DB_USER").map(String::as_str).unwrap_or_default();
    let password = constants.get("DB_PASSWORD").map(String::as_str).unwrap_or_default();
    let host = constants.get("DB_HOST").map(String::as_str).filter(|h| !h.is_empty()).unwrap_or("localhost");
    let (host, socket) = match host.split_once(':') {
        Some((host, socket)) if socket.starts_with('/') => (host, Some(socket)),
        _ => (host, None),
    };
    let ssl = config_php.lines().any(|line| line.contains("MYSQL_CLIENT_FLAGS") && line.contains("MYSQLI_CLIENT_SSL") && !line.trim_start().starts_with("//"));
    let mut url = format!("mysql://{}:{}@{}/{}?ssl-mode={}", encode(user), encode(password), host, encode(name), if ssl { "required" } else { "disabled" });
    if let Some(socket) = socket {
        url.push_str("&socket=");
        url.push_str(&encode(socket));
    }
    // `$table_prefix = 'wp_';`
    let prefix = config_php.lines().find_map(|line| {
        let value = line.trim_start().strip_prefix("$table_prefix")?.trim_start().strip_prefix('=')?.trim_start();
        let quote = value.chars().next().filter(|c| matches!(c, '\'' | '"'))?;
        let rest = &value[1..];
        Some(rest[..rest.find(quote)?].to_owned())
    });
    Some((url, prefix))
}

/// Percent-encoding for the parts of a URL.
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
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

    #[test]
    fn database_from_wp_config_php() {
        let php = "<?php\ndefine( 'DB_NAME', 'dbs123' );\ndefine( 'DB_USER', 'dbu123' );\ndefine( 'DB_PASSWORD', 'p@ss:w/rd' );\ndefine( 'DB_HOST', 'rdbms.example.de:3307' );\n$table_prefix = 'xyz_';\n";
        let (url, prefix) = database_from_wp_config(php).unwrap();
        assert_eq!(url, "mysql://dbu123:p%40ss%3Aw%2Frd@rdbms.example.de:3307/dbs123?ssl-mode=disabled");
        assert_eq!(prefix.as_deref(), Some("xyz_"));
        let socket = "define('DB_NAME', 'wp');\ndefine('DB_HOST', 'localhost:/run/mysqld/mysqld.sock');\n";
        assert_eq!(database_from_wp_config(socket).unwrap().0, "mysql://:@localhost/wp?ssl-mode=disabled&socket=%2Frun%2Fmysqld%2Fmysqld.sock");
        let tls = "define('DB_NAME', 'wp');\ndefine('MYSQL_CLIENT_FLAGS', MYSQLI_CLIENT_SSL);\n";
        assert_eq!(database_from_wp_config(tls).unwrap().0, "mysql://:@localhost/wp?ssl-mode=required");
        assert!(database_from_wp_config("<?php // nothing").is_none());
    }
}
