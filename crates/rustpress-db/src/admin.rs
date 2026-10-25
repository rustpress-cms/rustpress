//! Reads the administration area needs and the public site does not: everything regardless of status.

use sqlx::AssertSqlSafe;

use crate::{Db, Result};

impl Db {
    /// Comments with the given `comment_approved` value (`0` waits for moderation, `1` is approved).
    pub async fn count_comments_with_status(&self, approved: &str) -> Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE comment_approved = ?", self.prefix.table("comments"));
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).bind(approved).fetch_one(&self.pool).await?;
        Ok(count as u64)
    }
}
