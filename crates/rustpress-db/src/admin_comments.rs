//! Reads for the comment screens: every status, search, sorting, counts.

use sqlx::{AssertSqlSafe, MySql, QueryBuilder};

use crate::models::{COMMENT_COLUMNS, Comment};
use crate::{Db, Result};

/// The filters of the comment list.
#[derive(Debug, Clone, Default)]
pub struct CommentListQuery {
    /// `all` (approved and pending), `0`, `1`, `spam`, `trash`.
    pub status: String,
    pub post_id: Option<u64>,
    pub user_id: Option<u64>,
    /// `author_email`: the comments written under this address.
    pub author_email: Option<String>,
    /// `comment`, `pings` or empty for every type.
    pub kind: String,
    pub search: Option<String>,
    /// `comment_date`, `comment_author`, `comment_post_ID`, `comment_ID`; anything else sorts by date.
    pub orderby: String,
    pub desc: bool,
    pub limit: u64,
    pub offset: u64,
}

/// `wp_count_comments()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommentCounts {
    pub approved: u64,
    pub moderated: u64,
    pub spam: u64,
    pub trash: u64,
    pub post_trashed: u64,
    pub total_comments: u64,
    pub all: u64,
}

impl Db {
    fn push_comment_filters(&self, qb: &mut QueryBuilder<MySql>, query: &CommentListQuery) {
        qb.push(format!(" FROM {} c WHERE 1=1", self.prefix.table("comments")));
        match query.status.as_str() {
            "0" | "moderated" => {
                qb.push(" AND c.comment_approved = '0'");
            }
            "1" | "approved" => {
                qb.push(" AND c.comment_approved = '1'");
            }
            "spam" => {
                qb.push(" AND c.comment_approved = 'spam'");
            }
            "trash" => {
                qb.push(" AND c.comment_approved = 'trash'");
            }
            _ => {
                qb.push(" AND c.comment_approved IN ('0', '1')");
            }
        }
        if let Some(post) = query.post_id {
            qb.push(" AND c.comment_post_ID = ").push_bind(post);
        }
        if let Some(user) = query.user_id {
            qb.push(" AND c.user_id = ").push_bind(user);
        }
        if let Some(email) = &query.author_email {
            qb.push(" AND c.comment_author_email = ").push_bind(email.clone());
        }
        match query.kind.as_str() {
            "comment" => {
                qb.push(" AND c.comment_type IN ('comment', '')");
            }
            "pings" => {
                qb.push(" AND c.comment_type IN ('pingback', 'trackback')");
            }
            "" => {}
            other => {
                qb.push(" AND c.comment_type = ").push_bind(other.to_owned());
            }
        }
        if let Some(search) = query.search.as_deref().filter(|s| !s.is_empty()) {
            let like = format!("%{}%", search.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
            qb.push(" AND (c.comment_author LIKE ").push_bind(like.clone());
            qb.push(" OR c.comment_author_email LIKE ").push_bind(like.clone());
            qb.push(" OR c.comment_author_url LIKE ").push_bind(like.clone());
            qb.push(" OR c.comment_author_IP LIKE ").push_bind(like.clone());
            qb.push(" OR c.comment_content LIKE ").push_bind(like).push(")");
        }
    }

    /// The comments of the list and the number of all that match.
    pub async fn list_comments_admin(&self, query: &CommentListQuery) -> Result<(Vec<Comment>, u64)> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*)");
        self.push_comment_filters(&mut count, query);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT {COMMENT_COLUMNS}"));
        self.push_comment_filters(&mut qb, query);
        let column = match query.orderby.as_str() {
            "comment_author" => "c.comment_author",
            "comment_post_ID" => "c.comment_post_ID",
            "comment_ID" => "c.comment_ID",
            _ => "c.comment_date_gmt",
        };
        let direction = if query.desc { "DESC" } else { "ASC" };
        qb.push(format!(" ORDER BY {column} {direction}, c.comment_ID {direction}"));
        if query.limit > 0 {
            qb.push(" LIMIT ").push_bind(query.limit as i64).push(" OFFSET ").push_bind(query.offset as i64);
        }
        let comments: Vec<Comment> = qb.build_query_as().fetch_all(&self.pool).await?;
        Ok((comments, total as u64))
    }

    /// `wp_count_comments( $post_id )`.
    pub async fn comment_counts(&self, post_id: Option<u64>) -> Result<CommentCounts> {
        let posts = self.prefix.table("posts");
        let mut sql = format!("SELECT c.comment_approved, COUNT(*) FROM {} c", self.prefix.table("comments"));
        if post_id.is_none() {
            sql.push_str(&format!(" LEFT JOIN {posts} p ON p.ID = c.comment_post_ID"));
        }
        sql.push_str(" WHERE 1=1");
        if let Some(post) = post_id {
            sql.push_str(&format!(" AND c.comment_post_ID = {post}"));
        }
        sql.push_str(" GROUP BY c.comment_approved");
        let rows: Vec<(String, i64)> = sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?;
        let mut counts = CommentCounts::default();
        for (status, n) in rows {
            let n = n as u64;
            match status.as_str() {
                "0" => counts.moderated = n,
                "1" => counts.approved = n,
                "spam" => counts.spam = n,
                "trash" => counts.trash = n,
                "post-trashed" => counts.post_trashed = n,
                _ => {}
            }
        }
        counts.total_comments = counts.approved + counts.moderated + counts.post_trashed;
        counts.all = counts.approved + counts.moderated;
        Ok(counts)
    }

    /// The columns `Comment` leaves out: author IP, agent and karma are for the edit screen.
    pub async fn comment_extra(&self, id: u64) -> Result<Option<(String, String)>> {
        let sql = format!("SELECT comment_author_IP, comment_agent FROM {} WHERE comment_ID = ?", self.prefix.table("comments"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(id).fetch_optional(&self.pool).await?)
    }

    pub async fn comment_meta_value(&self, comment_id: u64, key: &str) -> Result<Option<String>> {
        let sql = format!("SELECT meta_value FROM {} WHERE comment_id = ? AND meta_key = ? ORDER BY meta_id LIMIT 1", self.prefix.table("commentmeta"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(comment_id).bind(key).fetch_optional(&self.pool).await?)
    }

    /// The comments with a `comment_approved` value that are older than `before` (GMT): what "Empty Spam" deletes.
    pub async fn comment_ids_before(&self, approved: &str, before: &str) -> Result<Vec<u64>> {
        let sql = format!("SELECT comment_ID FROM {} WHERE comment_approved = ? AND ? > comment_date_gmt", self.prefix.table("comments"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(approved).bind(before).fetch_all(&self.pool).await?)
    }

    /// Comment IDs whose parent is `parent`.
    pub async fn comment_children(&self, parent: u64) -> Result<Vec<u64>> {
        let sql = format!("SELECT comment_ID FROM {} WHERE comment_parent = ?", self.prefix.table("comments"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(parent).fetch_all(&self.pool).await?)
    }
}
