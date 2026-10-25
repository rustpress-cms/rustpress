//! The only way RustPress changes the database. A [`Writer`] exists only when the site was configured
//! with `allow_writes`; its connection is not read-only. Everything the frontend does goes through the
//! read-only [`crate::Db`].

use sqlx::mysql::MySqlPoolOptions;
use sqlx::{AssertSqlSafe, MySqlPool};

use crate::{Prefix, Result};

/// A value for a column.
#[derive(Debug, Clone)]
pub enum Val {
    Str(String),
    Int(i64),
    Null,
}

impl Val {
    fn bind<'q>(&'q self, query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        match self {
            Val::Str(s) => query.bind(s.as_str()),
            Val::Int(i) => query.bind(*i),
            Val::Null => query.bind(Option::<String>::None),
        }
    }
}

#[derive(Clone)]
pub struct Writer {
    pool: MySqlPool,
    prefix: Prefix,
}

impl Writer {
    /// Connects with a `mysql://user:pass@host:port/database` URL; the session can write.
    pub async fn connect(url: &str, prefix: &str) -> Result<Self> {
        let prefix = Prefix::new(prefix)?;
        // Like `wpdb::set_sql_mode()`: no strict mode and zero dates allowed.
        let pool = MySqlPoolOptions::new()
            .max_connections(4)
            .after_connect(|connection, _| {
                Box::pin(async move {
                    sqlx::query("SET SESSION sql_mode = 'NO_ENGINE_SUBSTITUTION'").execute(&mut *connection).await?;
                    Ok(())
                })
            })
            .connect(url)
            .await?;
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
    /// `update_option( $name, $value )` for a value that is already serialized: nothing happens when the
    /// stored value is the same, a missing option is added (autoload `auto`, like WordPress does).
    /// Returns whether the database changed.
    pub async fn update_option(&self, name: &str, value: &str) -> Result<bool> {
        self.update_option_with_autoload(name, value, "auto").await
    }

    /// `update_option()` where a missing option is added with the given `autoload` value.
    pub async fn update_option_with_autoload(&self, name: &str, value: &str, autoload: &str) -> Result<bool> {
        let table = self.prefix.table("options");
        let old: Option<String> = sqlx::query_scalar(AssertSqlSafe(format!("SELECT option_value FROM {table} WHERE option_name = ? LIMIT 1"))).bind(name).fetch_optional(&self.pool).await?;
        match old {
            Some(old) if old == value => Ok(false),
            Some(_) => {
                sqlx::query(AssertSqlSafe(format!("UPDATE {table} SET option_value = ? WHERE option_name = ?"))).bind(value).bind(name).execute(&self.pool).await?;
                Ok(true)
            }
            None => {
                sqlx::query(AssertSqlSafe(format!("INSERT INTO {table} (option_name, option_value, autoload) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE option_value = VALUES(option_value)")))
                    .bind(name)
                    .bind(value)
                    .bind(autoload)
                    .execute(&self.pool)
                    .await?;
                Ok(true)
            }
        }
    }

    /// `delete_option( $name )`.
    pub async fn delete_option(&self, name: &str) -> Result<bool> {
        let sql = format!("DELETE FROM {} WHERE option_name = ?", self.prefix.table("options"));
        Ok(sqlx::query(AssertSqlSafe(sql)).bind(name).execute(&self.pool).await?.rows_affected() > 0)
    }

    /// `set_transient( $name, $value, $expiration )`: the value and its timeout, serialized already.
    pub async fn set_transient(&self, name: &str, value: &str, expiration: i64, now: i64) -> Result<()> {
        self.update_option_with_autoload(&format!("_transient_{name}"), value, "off").await?;
        if expiration > 0 {
            self.update_option_with_autoload(&format!("_transient_timeout_{name}"), &(now + expiration).to_string(), "off").await?;
        }
        Ok(())
    }
    // ------------------------------------------------------------------ generic row access

    /// `INSERT` into a table (given without prefix); returns the new auto-increment id.
    pub async fn insert_row(&self, table: &str, fields: &[(&str, Val)]) -> Result<u64> {
        let columns: Vec<String> = fields.iter().map(|(c, _)| format!("`{c}`")).collect();
        let marks = vec!["?"; fields.len()].join(", ");
        let sql = format!("INSERT INTO {} ({}) VALUES ({marks})", self.prefix.table(table), columns.join(", "));
        let mut query = sqlx::query(AssertSqlSafe(sql));
        for (_, value) in fields {
            query = value.bind(query);
        }
        Ok(query.execute(&self.pool).await?.last_insert_id())
    }

    /// `UPDATE table SET fields WHERE key_column = key`; returns the rows changed.
    pub async fn update_rows(&self, table: &str, key_column: &str, key: u64, fields: &[(&str, Val)]) -> Result<u64> {
        if fields.is_empty() {
            return Ok(0);
        }
        let sets: Vec<String> = fields.iter().map(|(c, _)| format!("`{c}` = ?")).collect();
        let sql = format!("UPDATE {} SET {} WHERE `{key_column}` = ?", self.prefix.table(table), sets.join(", "));
        let mut query = sqlx::query(AssertSqlSafe(sql));
        for (_, value) in fields {
            query = value.bind(query);
        }
        Ok(query.bind(key).execute(&self.pool).await?.rows_affected())
    }

    /// `DELETE FROM table WHERE column = value`.
    pub async fn delete_rows(&self, table: &str, column: &str, value: u64) -> Result<u64> {
        let sql = format!("DELETE FROM {} WHERE `{column}` = ?", self.prefix.table(table));
        Ok(sqlx::query(AssertSqlSafe(sql)).bind(value).execute(&self.pool).await?.rows_affected())
    }

    /// `UPDATE table SET column = value WHERE key IN (ids)` for one column.
    pub async fn set_column_for_ids(&self, table: &str, key_column: &str, ids: &[u64], column: &str, value: Val) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let marks = vec!["?"; ids.len()].join(",");
        let sql = format!("UPDATE {} SET `{column}` = ? WHERE `{key_column}` IN ({marks})", self.prefix.table(table));
        let mut query = value.bind(sqlx::query(AssertSqlSafe(sql)));
        for id in ids {
            query = query.bind(*id);
        }
        Ok(query.execute(&self.pool).await?.rows_affected())
    }

    // ------------------------------------------------------------------ post meta

    /// `add_post_meta( $post_id, $key, $value )` (non-unique).
    pub async fn add_post_meta(&self, post_id: u64, key: &str, value: &str) -> Result<u64> {
        self.insert_row("postmeta", &[("post_id", Val::Int(post_id as i64)), ("meta_key", Val::Str(key.to_owned())), ("meta_value", Val::Str(value.to_owned()))]).await
    }

    /// `update_post_meta( $post_id, $key, $value )` for a single value: the first row is updated, one is added when none exists.
    pub async fn update_post_meta(&self, post_id: u64, key: &str, value: &str) -> Result<()> {
        let table = self.prefix.table("postmeta");
        let first: Option<u64> = sqlx::query_scalar(AssertSqlSafe(format!("SELECT meta_id FROM {table} WHERE post_id = ? AND meta_key = ? ORDER BY meta_id LIMIT 1"))).bind(post_id).bind(key).fetch_optional(&self.pool).await?;
        match first {
            Some(id) => {
                self.update_rows("postmeta", "meta_id", id, &[("meta_value", Val::Str(value.to_owned()))]).await?;
            }
            None => {
                self.add_post_meta(post_id, key, value).await?;
            }
        }
        Ok(())
    }

    /// `delete_post_meta( $post_id, $key )`.
    pub async fn delete_post_meta(&self, post_id: u64, key: &str) -> Result<u64> {
        let sql = format!("DELETE FROM {} WHERE post_id = ? AND meta_key = ?", self.prefix.table("postmeta"));
        Ok(sqlx::query(AssertSqlSafe(sql)).bind(post_id).bind(key).execute(&self.pool).await?.rows_affected())
    }

    /// Runs a statement with no arguments (maintenance queries such as term counts).
    pub async fn execute(&self, sql: String) -> Result<u64> {
        Ok(sqlx::query(AssertSqlSafe(sql)).execute(&self.pool).await?.rows_affected())
    }

    /// `DELETE FROM table WHERE <condition>`; the condition is built by the caller from trusted parts.
    pub async fn delete_rows_where(&self, table: &str, condition: &str) -> Result<u64> {
        let sql = format!("DELETE FROM {} WHERE {condition}", self.prefix.table(table));
        Ok(sqlx::query(AssertSqlSafe(sql)).execute(&self.pool).await?.rows_affected())
    }

    /// The first column of a query as numbers.
    pub async fn column_u64(&self, sql: &str) -> Result<Vec<u64>> {
        Ok(sqlx::query_scalar(AssertSqlSafe(sql.to_owned())).fetch_all(&self.pool).await?)
    }

    /// The first column of a query as text.
    pub async fn column_string(&self, sql: &str) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(AssertSqlSafe(sql.to_owned())).fetch_all(&self.pool).await?)
    }

    /// The table name with its prefix, for statements built by the caller.
    pub fn table(&self, name: &str) -> String {
        self.prefix.table(name)
    }
}
