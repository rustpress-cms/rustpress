use sqlx::AssertSqlSafe;
use crate::{Db, Result};

impl Db {
    /// Every option, by name: `SELECT * FROM $wpdb->options ORDER BY option_name`.
    pub async fn all_options(&self) -> Result<Vec<(String, String)>> {
        let sql = format!("SELECT option_name, option_value FROM {} ORDER BY option_name", self.prefix.table("options"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }

    /// Raw `option_value` for `name`, or `None` if the option does not exist.
    pub async fn option(&self, name: &str) -> Result<Option<String>> {
        let sql = format!("SELECT option_value FROM {} WHERE option_name = ?", self.prefix.table("options"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(name).fetch_optional(&self.pool).await?)
    }

    /// The values of the options named, in one query (`(name, value)`; missing options are left out).
    pub async fn options_named(&self, names: &[&str]) -> Result<Vec<(String, String)>> {
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let mut sql = crate::sql::Sql::new(format!("SELECT option_name, option_value FROM {} WHERE option_name IN (", self.prefix.table("options")));
        sql.push_list(names.iter().copied()).push(")");
        sql.fetch_all(&self.pool).await
    }

    /// All options WordPress loads on every request (`autoload` is `yes`/`on`/...).
    pub async fn autoload_options(&self) -> Result<Vec<(String, String)>> {
        let sql = format!(
            "SELECT option_name, option_value FROM {} WHERE autoload IN ('yes', 'on', 'auto-on', 'auto')",
            self.prefix.table("options")
        );
        crate::sql::Sql::new(sql).fetch_all(&self.pool).await
    }
}
