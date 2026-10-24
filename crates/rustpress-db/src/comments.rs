use sqlx::AssertSqlSafe;
use crate::models::{COMMENT_COLUMNS, Comment};
use crate::{Db, Result};

impl Db {
    /// Approved comments of a post in chronological order.
    pub async fn approved_comments(&self, post_id: u64) -> Result<Vec<Comment>> {
        let sql = format!(
            "SELECT {COMMENT_COLUMNS} FROM {} WHERE comment_post_ID = ? AND comment_approved = '1' \
             ORDER BY comment_date_gmt, comment_ID",
            self.prefix.table("comments")
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_id).fetch_all(&self.pool).await?)
    }

    /// A single comment by ID, whatever its status.
    pub async fn comment(&self, id: u64) -> Result<Option<Comment>> {
        let sql = format!("SELECT {COMMENT_COLUMNS} FROM {} WHERE comment_ID = ?", self.prefix.table("comments"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(id).fetch_optional(&self.pool).await?)
    }
}