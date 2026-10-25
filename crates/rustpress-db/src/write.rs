//! The only way RustPress changes the database. A [`Writer`] exists only when the site was configured
//! with `allow_writes`; its connection is not read-only. Everything the frontend does goes through the
//! read-only [`crate::Db`].

use sqlx::mysql::MySqlPoolOptions;
use sqlx::{AssertSqlSafe, MySqlPool};

use crate::{Prefix, Result};

#[derive(Clone)]
pub struct Writer {
    pool: MySqlPool,
    prefix: Prefix,
}

impl Writer {
    /// Connects with a `mysql://user:pass@host:port/database` URL; the session can write.
    pub async fn connect(url: &str, prefix: &str) -> Result<Self> {
        let prefix = Prefix::new(prefix)?;
        let pool = MySqlPoolOptions::new().max_connections(4).connect(url).await?;
        Ok(Self { pool, prefix })
    }

    pub fn table_prefix(&self) -> &str {
        self.prefix.as_str()
    }

    /// `update_user_meta( $user_id, $key, $value )` for a single value: every row with the key is
    /// updated, and one is added if there is none.
    pub async fn update_user_meta(&self, user_id: u64, key: &str, value: &str) -> Result<()> {
        let table = self.prefix.table("usermeta");
        let update = format!("UPDATE {table} SET meta_value = ? WHERE user_id = ? AND meta_key = ?");
        let existing: i64 = sqlx::query_scalar(AssertSqlSafe(format!("SELECT COUNT(*) FROM {table} WHERE user_id = ? AND meta_key = ?"))).bind(user_id).bind(key).fetch_one(&self.pool).await?;
        if existing > 0 {
            sqlx::query(AssertSqlSafe(update)).bind(value).bind(user_id).bind(key).execute(&self.pool).await?;
        } else {
            sqlx::query(AssertSqlSafe(format!("INSERT INTO {table} (user_id, meta_key, meta_value) VALUES (?, ?, ?)"))).bind(user_id).bind(key).bind(value).execute(&self.pool).await?;
        }
        Ok(())
    }

    /// `delete_user_meta( $user_id, $key )`.
    pub async fn delete_user_meta(&self, user_id: u64, key: &str) -> Result<()> {
        let sql = format!("DELETE FROM {} WHERE user_id = ? AND meta_key = ?", self.prefix.table("usermeta"));
        sqlx::query(AssertSqlSafe(sql)).bind(user_id).bind(key).execute(&self.pool).await?;
        Ok(())
    }
}
