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

    /// `get_lastcommentmodified( 'GMT' )`.
    pub async fn last_comment_modified_gmt(&self) -> Result<Option<String>> {
        let sql = format!("SELECT CAST(comment_date_gmt AS CHAR) FROM {} WHERE comment_approved = '1' ORDER BY comment_date_gmt DESC LIMIT 1", self.prefix.table("comments"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).fetch_optional(&self.pool).await?)
    }

    /// The comments of a feed, newest first: those of one post, or of all
    /// published posts (`WP_Query` comment feeds).
    pub async fn feed_comments(&self, post_id: Option<u64>, limit: u64) -> Result<Vec<Comment>> {
        let comments = self.prefix.table("comments");
        let posts = self.prefix.table("posts");
        let select = format!("SELECT {COMMENT_COLUMNS} FROM {comments}");
        let sql = match post_id {
            Some(_) => format!("{select} WHERE comment_post_ID = ? AND comment_approved = '1' AND {comments}.comment_type != 'note' ORDER BY comment_date_gmt DESC LIMIT {limit}"),
            None => format!(
                "{select} JOIN {posts} ON ( {comments}.comment_post_ID = {posts}.ID ) WHERE ( post_status = 'publish' OR ( post_status = 'inherit' AND post_type = 'attachment' ) ) AND comment_approved = '1' AND {comments}.comment_type != 'note' ORDER BY comment_date_gmt DESC LIMIT {limit}"
            ),
        };
        let mut query = sqlx::query_as(AssertSqlSafe(sql));
        if let Some(id) = post_id {
            query = query.bind(id);
        }
        Ok(query.fetch_all(&self.pool).await?)
    }
}