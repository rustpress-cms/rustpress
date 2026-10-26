//! Reads the administration area needs and the public site does not: everything regardless of status.

use sqlx::AssertSqlSafe;

use crate::{Db, Result};

/// Where `wp_unique_post_slug()` looks for a clash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlugScope {
    Attachment,
    Hierarchical,
    Flat,
}

impl Db {
    /// Comments with the given `comment_approved` value (`0` waits for moderation, `1` is approved).
    pub async fn count_comments_with_status(&self, approved: &str) -> Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE comment_approved = ?", self.prefix.table("comments"));
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).bind(approved).fetch_one(&self.pool).await?;
        Ok(count as u64)
    }
    /// The comments of one post with a `comment_approved` value (`wp_count_comments( $post_id )`).
    pub async fn count_comments_for_post_with_status(&self, post_id: u64, approved: &str) -> Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE comment_post_ID = ? AND comment_approved = ?", self.prefix.table("comments"));
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).bind(post_id).bind(approved).fetch_one(&self.pool).await?;
        Ok(count as u64)
    }
    /// `wp_count_posts()`: `(status, count)` for a post type.
    pub async fn post_status_counts(&self, post_type: &str) -> Result<Vec<(String, u64)>> {
        let sql = format!("SELECT post_status, COUNT(*) FROM {} WHERE post_type = ? GROUP BY post_status", self.prefix.table("posts"));
        let rows: Vec<(String, i64)> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(status, count)| (status, count as u64)).collect())
    }

    /// Posts of the user in the types and statuses (`Mine (n)` of the posts list).
    pub async fn count_user_posts(&self, post_type: &str, user_id: u64, exclude_statuses: &[&str]) -> Result<u64> {
        let mut sql = format!("SELECT COUNT(*) FROM {} WHERE post_type = ? AND post_author = ?", self.prefix.table("posts"));
        for _ in exclude_statuses {
            sql.push_str(" AND post_status != ?");
        }
        let mut query = sqlx::query_scalar::<_, i64>(AssertSqlSafe(sql)).bind(post_type).bind(user_id);
        for status in exclude_statuses {
            query = query.bind(*status);
        }
        Ok(query.fetch_one(&self.pool).await? as u64)
    }

    /// The months with posts of a type, newest first (`months_dropdown()`); `trash_only` restricts to trashed posts,
    /// otherwise trashed and auto-draft posts are left out.
    pub async fn post_months(&self, post_type: &str, trash_only: bool) -> Result<Vec<(i32, i32)>> {
        let extra = if trash_only { "AND post_status = 'trash'" } else { "AND post_status != 'auto-draft' AND post_status != 'trash'" };
        let sql = format!(
            "SELECT DISTINCT YEAR(post_date) AS year, MONTH(post_date) AS month FROM {} WHERE post_type = ? {extra} ORDER BY post_date DESC",
            self.prefix.table("posts")
        );
        let rows: Vec<(Option<i32>, Option<i32>)> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(y, m)| (y.unwrap_or(0), m.unwrap_or(0))).collect())
    }

    /// Pending (unapproved) comments per post, for the posts that have some.
    pub async fn pending_comment_counts(&self, post_ids: &[u64]) -> Result<Vec<(u64, u64)>> {
        if post_ids.is_empty() {
            return Ok(Vec::new());
        }
        let marks = vec!["?"; post_ids.len()].join(",");
        let sql = format!("SELECT comment_post_ID, COUNT(*) FROM {} WHERE comment_approved = '0' AND comment_post_ID IN ({marks}) GROUP BY comment_post_ID", self.prefix.table("comments"));
        let mut query = sqlx::query_as::<_, (u64, i64)>(AssertSqlSafe(sql));
        for id in post_ids {
            query = query.bind(*id);
        }
        Ok(query.fetch_all(&self.pool).await?.into_iter().map(|(id, n)| (id, n as u64)).collect())
    }

    /// Users by ID (display names, e-mail addresses).
    pub async fn users_by_ids(&self, ids: &[u64]) -> Result<Vec<crate::models::User>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let marks = vec!["?"; ids.len()].join(",");
        let sql = format!("SELECT {} FROM {} WHERE ID IN ({marks})", crate::models::USER_COLUMNS, self.prefix.table("users"));
        let mut query = sqlx::query_as(AssertSqlSafe(sql));
        for id in ids {
            query = query.bind(*id);
        }
        Ok(query.fetch_all(&self.pool).await?)
    }

    /// All users, ordered by display name (the author drop-down).
    pub async fn all_users(&self) -> Result<Vec<crate::models::User>> {
        let sql = format!("SELECT {} FROM {} ORDER BY display_name", crate::models::USER_COLUMNS, self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }

    /// Post meta values of a key for posts (`_edit_lock` and the like).
    pub async fn post_meta_value(&self, post_id: u64, key: &str) -> Result<Option<String>> {
        if let Some(primed) = crate::primed::primed_meta(post_id) {
            return Ok(primed.into_iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value));
        }
        let sql = format!("SELECT meta_value FROM {} WHERE post_id = ? AND meta_key = ? ORDER BY meta_id LIMIT 1", self.prefix.table("postmeta"));
        Ok(sqlx::query_scalar::<_, Option<String>>(AssertSqlSafe(sql)).bind(post_id).bind(key).fetch_optional(&self.pool).await?.flatten())
    }
    /// Whether another post already uses `name` within the scope `wp_unique_post_slug()` checks.
    pub async fn post_name_taken(&self, scope: SlugScope, name: &str, post_type: &str, post_id: u64, parent: u64) -> Result<bool> {
        let table = self.prefix.table("posts");
        let found: Option<String> = match scope {
            SlugScope::Attachment => {
                let sql = format!("SELECT post_name FROM {table} WHERE post_name = ? AND ID != ? LIMIT 1");
                sqlx::query_scalar(AssertSqlSafe(sql)).bind(name).bind(post_id).fetch_optional(&self.pool).await?
            }
            SlugScope::Hierarchical => {
                let sql = format!("SELECT post_name FROM {table} WHERE post_name = ? AND post_type IN (?, 'attachment') AND ID != ? AND post_parent = ? LIMIT 1");
                sqlx::query_scalar(AssertSqlSafe(sql)).bind(name).bind(post_type).bind(post_id).bind(parent).fetch_optional(&self.pool).await?
            }
            SlugScope::Flat => {
                let sql = format!("SELECT post_name FROM {table} WHERE post_name = ? AND post_type = ? AND ID != ? LIMIT 1");
                sqlx::query_scalar(AssertSqlSafe(sql)).bind(name).bind(post_type).bind(post_id).fetch_optional(&self.pool).await?
            }
        };
        Ok(found.is_some())
    }

    /// `wp_unique_term_slug()` for a new term: `slug`, or `slug-2`, `slug-3`... when taken.
    pub async fn unique_term_slug(&self, slug: &str) -> Result<String> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE slug = ?", self.prefix.table("terms"));
        let mut candidate = slug.to_owned();
        let mut number = 2;
        loop {
            let taken: i64 = sqlx::query_scalar(AssertSqlSafe(sql.clone())).bind(&candidate).fetch_one(&self.pool).await?;
            if taken == 0 {
                return Ok(candidate);
            }
            candidate = format!("{slug}-{number}");
            number += 1;
        }
    }

    /// `(comment_ID, comment_approved)` of all comments of a post.
    pub async fn comment_statuses(&self, post_id: u64) -> Result<Vec<(u64, String)>> {
        let sql = format!("SELECT comment_ID, comment_approved FROM {} WHERE comment_post_ID = ? ORDER BY comment_ID", self.prefix.table("comments"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_id).fetch_all(&self.pool).await?)
    }

    /// `to_ping` and `pinged` of a post.
    pub async fn post_pings(&self, post_id: u64) -> Result<Option<(String, String)>> {
        let sql = format!("SELECT to_ping, pinged FROM {} WHERE ID = ?", self.prefix.table("posts"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_id).fetch_optional(&self.pool).await?)
    }

    /// `meta_form()`: the keys of custom fields that do not start with `_`, at most `limit`, sorted by name.
    pub async fn custom_field_keys(&self, limit: u64) -> Result<Vec<String>> {
        let sql = format!(
            "SELECT DISTINCT meta_key FROM {} WHERE meta_key NOT BETWEEN '_' AND '_z' HAVING meta_key NOT LIKE '\\_%' ORDER BY meta_key LIMIT ?",
            self.prefix.table("postmeta")
        );
        let keys: Vec<Option<String>> = sqlx::query_scalar(AssertSqlSafe(sql)).bind(limit).fetch_all(&self.pool).await?;
        Ok(keys.into_iter().flatten().collect())
    }

    /// `get_comments( array( 'post_id' => $id, 'count' => true ) )`: approved and waiting comments of all types but notes.
    pub async fn comment_count_all_types(&self, post_id: u64) -> Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE comment_post_ID = ? AND comment_approved IN ('0', '1') AND comment_type != 'note'", self.prefix.table("comments"));
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).bind(post_id).fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// The IDs of the revisions of a post.
    pub async fn revision_ids(&self, post_id: u64) -> Result<Vec<u64>> {
        let sql = format!("SELECT ID FROM {} WHERE post_parent = ? AND post_type = 'revision'", self.prefix.table("posts"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(post_id).fetch_all(&self.pool).await?)
    }
    /// Scheduled posts whose GMT date is not after `gmt_now` (`Y-m-d H:i:s`).
    pub async fn due_future_post_ids(&self, gmt_now: &str) -> Result<Vec<u64>> {
        let sql = format!("SELECT ID FROM {} WHERE post_status = 'future' AND post_date_gmt <= ? AND post_date_gmt != '0000-00-00 00:00:00' ORDER BY post_date_gmt", self.prefix.table("posts"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(gmt_now).fetch_all(&self.pool).await?)
    }

    /// Auto-drafts created before `before` (GMT is not stored for them; the local date is compared).
    pub async fn old_auto_draft_ids(&self, before: &str) -> Result<Vec<u64>> {
        let sql = format!("SELECT ID FROM {} WHERE post_status = 'auto-draft' AND post_date < ?", self.prefix.table("posts"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(before).fetch_all(&self.pool).await?)
    }

    /// Trashed posts whose `_wp_trash_meta_time` is before the given Unix time.
    pub async fn trashed_before(&self, timestamp: i64) -> Result<Vec<u64>> {
        let sql = format!(
            "SELECT p.ID FROM {posts} p JOIN {meta} m ON m.post_id = p.ID AND m.meta_key = '_wp_trash_meta_time' WHERE p.post_status = 'trash' AND CAST(m.meta_value AS SIGNED) < ?",
            posts = self.prefix.table("posts"),
            meta = self.prefix.table("postmeta")
        );
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).bind(timestamp).fetch_all(&self.pool).await?)
    }
}
