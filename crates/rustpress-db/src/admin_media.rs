//! Reads for the media library: attachments with MIME filter, search, date, parent and paging.

use sqlx::{AssertSqlSafe, MySql, QueryBuilder};

use crate::models::{POST_COLUMNS, Post};
use crate::{Db, Result};

/// The filters of the attachment list.
#[derive(Debug, Clone, Default)]
pub struct AttachmentQuery {
    /// `image`, `audio`, `video`, `image/png`, or a comma separated list of MIME types or groups.
    pub mime: Option<String>,
    /// The whole search string.
    pub search: Option<String>,
    /// `WP_Query::parse_search_terms()`: each must match title, excerpt, content or file name. The caller splits the
    /// string; without terms, `search` is the one term.
    pub search_terms: Vec<String>,
    /// `WP_Query::parse_search_order()` before `orderby`: no order was asked for.
    pub relevance: bool,
    pub year: Option<u32>,
    pub month: Option<u32>,
    /// `Some(0)` lists unattached files.
    pub parent: Option<u64>,
    pub author: Option<u64>,
    pub ids: Vec<u64>,
    pub exclude_ids: Vec<u64>,
    /// `inherit` (default), `trash`, `private`.
    pub status: Option<String>,
    /// `date`, `title`, `ID`, `modified`, `parent`, `author`.
    pub orderby: String,
    pub desc: bool,
    pub limit: u64,
    pub offset: u64,
}

fn search_terms(query: &AttachmentQuery) -> Vec<&str> {
    if query.search_terms.is_empty() {
        query.search.as_deref().filter(|s| !s.is_empty()).into_iter().collect()
    } else {
        query.search_terms.iter().map(String::as_str).collect()
    }
}

/// `%term%` with the wildcards of LIKE escaped.
fn like_pattern(term: &str) -> String {
    format!("%{}%", term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
}

/// `WP_Query::parse_search_order()`, followed by a comma.
fn push_search_order(qb: &mut QueryBuilder<MySql>, query: &AttachmentQuery) {
    let terms = search_terms(query);
    let [first, rest @ ..] = terms.as_slice() else { return };
    if rest.is_empty() {
        qb.push("p.post_title LIKE ").push_bind(like_pattern(first)).push(" DESC, ");
        return;
    }
    // Negative terms are not ported, as in the search itself: the whole string always counts.
    let phrase = like_pattern(query.search.as_deref().unwrap_or_default());
    qb.push("(CASE WHEN p.post_title LIKE ").push_bind(phrase.clone()).push(" THEN 1 ");
    if terms.len() < 7 {
        for (joiner, rank) in [(" AND ", 2), (" OR ", 3)] {
            qb.push("WHEN ");
            for (index, term) in terms.iter().enumerate() {
                if index > 0 {
                    qb.push(joiner);
                }
                qb.push("p.post_title LIKE ").push_bind(like_pattern(term));
            }
            qb.push(format!(" THEN {rank} "));
        }
    }
    qb.push("WHEN p.post_excerpt LIKE ").push_bind(phrase.clone()).push(" THEN 4 ");
    qb.push("WHEN p.post_content LIKE ").push_bind(phrase).push(" THEN 5 ELSE 6 END), ");
}

impl Db {
    fn push_attachment_filters(&self, qb: &mut QueryBuilder<MySql>, query: &AttachmentQuery) {
        qb.push(format!(" FROM {} p WHERE p.post_type = 'attachment'", self.prefix.table("posts")));
        match query.status.as_deref() {
            Some("trash") => {
                qb.push(" AND p.post_status = 'trash'");
            }
            Some("private") => {
                qb.push(" AND p.post_status = 'private'");
            }
            _ => {
                qb.push(" AND p.post_status IN ('inherit', 'private')");
            }
        }
        if let Some(mime) = query.mime.as_deref().filter(|m| !m.is_empty() && *m != "all") {
            qb.push(" AND (");
            for (index, part) in mime.split(',').map(str::trim).filter(|p| !p.is_empty()).enumerate() {
                if index > 0 {
                    qb.push(" OR ");
                }
                if part.contains('/') {
                    qb.push("p.post_mime_type = ").push_bind(part.to_owned());
                } else {
                    qb.push("p.post_mime_type LIKE ").push_bind(format!("{part}/%"));
                }
            }
            qb.push(")");
        }
        for term in search_terms(query) {
            let like = like_pattern(term);
            qb.push(" AND (p.post_title LIKE ").push_bind(like.clone());
            qb.push(" OR p.post_excerpt LIKE ").push_bind(like.clone());
            qb.push(" OR p.post_content LIKE ").push_bind(like.clone());
            qb.push(format!(" OR EXISTS (SELECT 1 FROM {} m WHERE m.post_id = p.ID AND m.meta_key = '_wp_attached_file' AND m.meta_value LIKE ", self.prefix.table("postmeta")));
            qb.push_bind(like);
            qb.push("))");
        }
        if let Some(year) = query.year {
            qb.push(" AND YEAR(p.post_date) = ").push_bind(year as i64);
        }
        if let Some(month) = query.month {
            qb.push(" AND MONTH(p.post_date) = ").push_bind(month as i64);
        }
        if let Some(parent) = query.parent {
            qb.push(" AND p.post_parent = ").push_bind(parent);
        }
        if let Some(author) = query.author {
            qb.push(" AND p.post_author = ").push_bind(author);
        }
        if !query.ids.is_empty() {
            qb.push(" AND p.ID IN (");
            let mut list = qb.separated(", ");
            for id in &query.ids {
                list.push_bind(*id);
            }
            qb.push(")");
        }
        if !query.exclude_ids.is_empty() {
            qb.push(" AND p.ID NOT IN (");
            let mut list = qb.separated(", ");
            for id in &query.exclude_ids {
                list.push_bind(*id);
            }
            qb.push(")");
        }
    }

    /// The attachments of the list and the number of all that match.
    pub async fn list_attachments(&self, query: &AttachmentQuery) -> Result<(Vec<Post>, u64)> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*)");
        self.push_attachment_filters(&mut count, query);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT {POST_COLUMNS}"));
        self.push_attachment_filters(&mut qb, query);
        let column = match query.orderby.as_str() {
            "title" => "p.post_title",
            "ID" | "id" => "p.ID",
            "modified" => "p.post_modified",
            "parent" => "p.post_parent",
            "author" => "p.post_author",
            _ => "p.post_date",
        };
        let direction = if query.desc { "DESC" } else { "ASC" };
        qb.push(" ORDER BY ");
        if query.relevance {
            push_search_order(&mut qb, query);
        }
        qb.push(format!("{column} {direction}, p.ID {direction}"));
        if query.limit > 0 {
            qb.push(" LIMIT ").push_bind(query.limit as i64).push(" OFFSET ").push_bind(query.offset as i64);
        }
        let posts: Vec<Post> = qb.build_query_as().fetch_all(&self.pool).await?;
        Ok((posts, total as u64))
    }

    /// `update_gallery_tab()`: the files attached to a post, those in the trash aside.
    pub async fn count_attached(&self, post_id: u64) -> Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE post_type = 'attachment' AND post_status != 'trash' AND post_parent = ?", self.prefix.table("posts"));
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).bind(post_id).fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// `get_children()` in `get_media_items()`. Its `orderby` of `menu_order ASC, ID` comes out of `WP_Query` as
    /// `menu_order DESC, ID DESC`: words are split at spaces, `ASC,` is no key, and `order` applies to both.
    pub async fn attached_posts(&self, post_id: u64) -> Result<Vec<Post>> {
        let sql = format!(
            "SELECT {POST_COLUMNS} FROM {} p WHERE p.post_type = 'attachment' AND p.post_parent = ? AND p.post_status NOT IN ('trash', 'auto-draft') ORDER BY p.menu_order DESC, p.ID DESC",
            self.prefix.table("posts")
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_id).fetch_all(&self.pool).await?)
    }

    /// `get_available_post_mime_types( 'attachment' )`.
    pub async fn attachment_mime_types(&self) -> Result<Vec<String>> {
        let sql = format!("SELECT DISTINCT post_mime_type FROM {} WHERE post_type = 'attachment'", self.prefix.table("posts"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }

    /// `(post_mime_type, count)` of every attachment, for the type filter and the "Unattached" counts.
    pub async fn attachment_mime_counts(&self) -> Result<Vec<(String, u64)>> {
        let sql = format!("SELECT post_mime_type, COUNT(*) FROM {} WHERE post_type = 'attachment' AND post_status IN ('inherit', 'private') GROUP BY post_mime_type", self.prefix.table("posts"));
        let rows: Vec<(String, i64)> = sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(m, n)| (m, n as u64)).collect())
    }

    pub async fn attachment_unattached_count(&self) -> Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {} p WHERE p.post_type = 'attachment' AND p.post_status IN ('inherit', 'private') AND p.post_parent = 0", self.prefix.table("posts"));
        let n: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).fetch_one(&self.pool).await?;
        Ok(n as u64)
    }

    pub async fn attachment_trash_count(&self) -> Result<u64> {
        let sql = format!("SELECT COUNT(*) FROM {} p WHERE p.post_type = 'attachment' AND p.post_status = 'trash'", self.prefix.table("posts"));
        let n: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).fetch_one(&self.pool).await?;
        Ok(n as u64)
    }

    /// The months with attachments, newest first: `(year, month)`.
    pub async fn attachment_months(&self) -> Result<Vec<(i32, i32)>> {
        let sql = format!(
            "SELECT DISTINCT YEAR(post_date) AS y, MONTH(post_date) AS m FROM {} WHERE post_type = 'attachment' AND post_status IN ('inherit', 'private') ORDER BY post_date DESC",
            self.prefix.table("posts")
        );
        let rows: Vec<(i32, i32)> = sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?;
        Ok(rows)
    }
}
