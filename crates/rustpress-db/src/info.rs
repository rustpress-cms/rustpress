//! What the Site Health screen asks the database about itself.

use sqlx::AssertSqlSafe;

use crate::{Db, Result};

/// The settings of the database server (`$wpdb`, `SHOW VARIABLES`).
#[derive(Debug, Clone, Default)]
pub struct ServerInfo {
    pub version: String,
    pub user: String,
    pub host: String,
    pub name: String,
    pub charset: String,
    pub collation: String,
    pub max_allowed_packet: u64,
    pub max_connections: u64,
}

impl Db {
    /// The server version, the connection and the character set of the tables.
    pub async fn server_info(&self) -> Result<ServerInfo> {
        let row: (String, String, String, Option<String>, u64, u64) = sqlx::query_as(
            "SELECT VERSION(), SUBSTRING_INDEX(USER(), '@', 1), COALESCE(@@hostname, ''), DATABASE(), CAST(@@max_allowed_packet AS UNSIGNED), CAST(@@max_connections AS UNSIGNED)",
        )
        .fetch_one(&self.pool)
        .await?;
        // The bare name: `TABLE_NAME` holds it without the quotes of `Prefix::table()`.
        let table = format!("{}posts", self.prefix.as_str());
        let collation: Option<String> = sqlx::query_scalar(AssertSqlSafe("SELECT TABLE_COLLATION FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?".to_owned())).bind(&table).fetch_optional(&self.pool).await?;
        let collation = collation.unwrap_or_default();
        let charset = collation.split('_').next().unwrap_or_default().to_owned();
        Ok(ServerInfo { version: row.0, user: row.1, host: row.2, name: row.3.unwrap_or_default(), charset, collation, max_allowed_packet: row.4, max_connections: row.5 })
    }

    /// The size of the tables of the site (`WP_Debug_Data::get_database_size()`).
    pub async fn database_size(&self) -> Result<u64> {
        let like = format!("{}%", self.prefix.as_str().replace('_', "\\_"));
        let size: u64 = sqlx::query_scalar("SELECT CAST(COALESCE(SUM(data_length + index_length), 0) AS UNSIGNED) FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME LIKE ?").bind(like).fetch_one(&self.pool).await?;
        Ok(size)
    }
}
