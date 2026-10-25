//! Reads for the WXR export (`wp-admin/includes/export.php`).

use sqlx::{AssertSqlSafe, FromRow};

use crate::models::{POST_COLUMNS, Post, TERM_COLUMNS, Term};
use crate::{Db, Result};

/// Which posts an export covers (`export_wp()` arguments).
#[derive(Debug, Clone, Default)]
pub struct ExportFilter {
    /// The post types to export.
    pub post_types: Vec<String>,
    /// `post_status = %s`; without it everything but auto-drafts.
    pub status: Option<String>,
    /// `term_taxonomy_id` of the category.
    pub category: Option<u64>,
    pub author: Option<u64>,
    /// `post_date >= %s`.
    pub start: Option<String>,
    /// `post_date < %s`.
    pub end: Option<String>,
}

/// A comment as the export writes it.
#[derive(Debug, Clone, Default, FromRow)]
pub struct ExportComment {
    #[sqlx(rename = "comment_ID")]
    pub comment_id: u64,
    pub comment_author: String,
    pub comment_author_email: String,
    pub comment_author_url: String,
    #[sqlx(rename = "comment_author_IP")]
    pub comment_author_ip: String,
    pub comment_date: String,
    pub comment_date_gmt: String,
    pub comment_content: String,
    pub comment_approved: String,
    pub comment_type: String,
    pub comment_parent: u64,
    pub user_id: u64,
}

impl Db {
    /// The IDs of the posts to export, by type, status and ID (what MariaDB hands out for the query of WordPress).
    pub async fn export_post_ids(&self, filter: &ExportFilter) -> Result<Vec<u64>> {
        let posts = self.prefix.table("posts");
        let mut sql = format!("SELECT {posts}.ID FROM {posts} ");
        if filter.category.is_some() {
            sql.push_str(&format!("INNER JOIN {} ON ({posts}.ID = {0}.object_id) ", self.prefix.table("term_relationships")));
        }
        let types = filter.post_types.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!("WHERE {posts}.post_type IN ({types})"));
        if filter.status.is_some() {
            sql.push_str(&format!(" AND {posts}.post_status = ?"));
        } else {
            sql.push_str(&format!(" AND {posts}.post_status != 'auto-draft'"));
        }
        if filter.category.is_some() {
            sql.push_str(&format!(" AND {}.term_taxonomy_id = ?", self.prefix.table("term_relationships")));
        }
        if filter.author.is_some() {
            sql.push_str(&format!(" AND {posts}.post_author = ?"));
        }
        if filter.start.is_some() {
            sql.push_str(&format!(" AND {posts}.post_date >= ?"));
        }
        if filter.end.is_some() {
            sql.push_str(&format!(" AND {posts}.post_date < ?"));
        }
        sql.push_str(&format!(" ORDER BY {posts}.post_type, {posts}.post_status, {posts}.ID"));
        let mut query = sqlx::query_scalar::<_, u64>(AssertSqlSafe(sql));
        for t in &filter.post_types {
            query = query.bind(t.clone());
        }
        if let Some(status) = &filter.status {
            query = query.bind(status.clone());
        }
        if let Some(category) = filter.category {
            query = query.bind(category);
        }
        if let Some(author) = filter.author {
            query = query.bind(author);
        }
        if let Some(start) = &filter.start {
            query = query.bind(start.clone());
        }
        if let Some(end) = &filter.end {
            query = query.bind(end.clone());
        }
        Ok(query.fetch_all(&self.pool).await?)
    }

    /// Attachments of these posts and the posts' featured images.
    pub async fn export_attachment_ids(&self, post_ids: &[u64]) -> Result<Vec<u64>> {
        if post_ids.is_empty() {
            return Ok(Vec::new());
        }
        let list = post_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let attachments: Vec<u64> = sqlx::query_scalar(AssertSqlSafe(format!("SELECT ID FROM {} WHERE post_parent IN ({list}) AND post_type = 'attachment'", self.prefix.table("posts")))).fetch_all(&self.pool).await?;
        let thumbnails: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!("SELECT meta_value FROM {} WHERE post_id IN ({list}) AND meta_key = '_thumbnail_id'", self.prefix.table("postmeta")))).fetch_all(&self.pool).await?;
        let mut out = attachments;
        out.extend(thumbnails.iter().filter_map(|v| v.trim().parse::<u64>().ok()).filter(|id| *id > 0));
        Ok(out)
    }

    /// The posts with these IDs, whatever their type and status, ordered by ID.
    pub async fn posts_by_ids(&self, ids: &[u64]) -> Result<Vec<Post>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let list = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT {POST_COLUMNS} FROM {} WHERE ID IN ({list}) ORDER BY ID", self.prefix.table("posts"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }

    /// The distinct authors of these posts, in the order the posts list them.
    pub async fn export_authors(&self, ids: &[u64]) -> Result<Vec<u64>> {
        let posts = self.prefix.table("posts");
        let mut authors: Vec<u64> = Vec::new();
        let chunks: Vec<&[u64]> = if ids.is_empty() { vec![&[][..]] } else { ids.chunks(20).collect() };
        for chunk in chunks {
            let and = if chunk.is_empty() { String::new() } else { format!("AND ID IN ({})", chunk.iter().map(u64::to_string).collect::<Vec<_>>().join(", ")) };
            let rows: Vec<u64> = sqlx::query_scalar(AssertSqlSafe(format!("SELECT post_author FROM {posts} WHERE post_status != 'auto-draft' {and} ORDER BY ID"))).fetch_all(&self.pool).await?;
            for author in rows {
                if !authors.contains(&author) {
                    authors.push(author);
                }
            }
        }
        Ok(authors)
    }

    /// The comments of a post for the export: everything but spam.
    pub async fn export_comments(&self, post_id: u64) -> Result<Vec<ExportComment>> {
        let sql = format!(
            "SELECT comment_ID, comment_author, comment_author_email, comment_author_url, comment_author_IP, CAST(comment_date AS CHAR) AS comment_date, CAST(comment_date_gmt AS CHAR) AS comment_date_gmt, comment_content, comment_approved, comment_type, comment_parent, user_id FROM {} WHERE comment_post_ID = ? AND comment_approved <> 'spam' ORDER BY comment_ID",
            self.prefix.table("comments")
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_id).fetch_all(&self.pool).await?)
    }

    pub async fn comment_meta_pairs(&self, comment_id: u64) -> Result<Vec<(Option<String>, Option<String>)>> {
        let sql = format!("SELECT meta_key, meta_value FROM {} WHERE comment_id = ? ORDER BY meta_id", self.prefix.table("commentmeta"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(comment_id).fetch_all(&self.pool).await?)
    }

    pub async fn term_meta_pairs(&self, term_id: u64) -> Result<Vec<(Option<String>, Option<String>)>> {
        let sql = format!("SELECT meta_key, meta_value FROM {} WHERE term_id = ? ORDER BY meta_id", self.prefix.table("termmeta"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(term_id).fetch_all(&self.pool).await?)
    }

    /// Every term of the taxonomies (or of all taxonomies that are not built in), ordered by name.
    pub async fn export_terms(&self, taxonomies: &[&str], exclude: &[&str]) -> Result<Vec<Term>> {
        let mut sql = format!("SELECT {TERM_COLUMNS} FROM {} WHERE 1=1", self.term_from());
        if !taxonomies.is_empty() {
            sql.push_str(&format!(" AND tt.taxonomy IN ({})", taxonomies.iter().map(|_| "?").collect::<Vec<_>>().join(",")));
        }
        if !exclude.is_empty() {
            sql.push_str(&format!(" AND tt.taxonomy NOT IN ({})", exclude.iter().map(|_| "?").collect::<Vec<_>>().join(",")));
        }
        sql.push_str(" ORDER BY t.name ASC, t.term_id ASC");
        let mut query = sqlx::query_as::<_, Term>(AssertSqlSafe(sql));
        for t in taxonomies.iter().chain(exclude.iter()) {
            query = query.bind((*t).to_owned());
        }
        Ok(query.fetch_all(&self.pool).await?)
    }

    /// The months that have posts of the type, newest first.
    pub async fn export_months(&self, post_type: &str) -> Result<Vec<(i32, i32)>> {
        let sql = format!("SELECT DISTINCT YEAR(post_date) AS year, MONTH(post_date) AS month FROM {} WHERE post_type = ? AND post_status != 'auto-draft' ORDER BY post_date DESC", self.prefix.table("posts"));
        let rows: Vec<(i32, i32)> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
        let mut out: Vec<(i32, i32)> = Vec::new();
        for row in rows {
            if !out.contains(&row) {
                out.push(row);
            }
        }
        Ok(out)
    }

    /// The authors of posts of the type, ordered by display name.
    pub async fn export_authors_of_type(&self, post_type: &str) -> Result<Vec<crate::models::User>> {
        let sql = format!(
            "SELECT {} FROM {} WHERE ID IN (SELECT DISTINCT post_author FROM {} WHERE post_type = ? AND post_status != 'auto-draft') ORDER BY display_name ASC",
            crate::models::USER_COLUMNS,
            self.prefix.table("users"),
            self.prefix.table("posts")
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?)
    }

    /// A user's first and last name from the usermeta.
    pub async fn user_names(&self, user_id: u64) -> Result<(String, String)> {
        let first = self.user_meta(user_id, "first_name").await?.unwrap_or_default();
        let last = self.user_meta(user_id, "last_name").await?.unwrap_or_default();
        Ok((first, last))
    }
}
