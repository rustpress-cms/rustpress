//! Queries behind `wp_get_archives()`, the calendar and padded term counts.

use sqlx::AssertSqlSafe;

use crate::models::{POST_COLUMNS, Post};
use crate::{Db, Result};

/// One row of `wp_get_archives()`.
#[derive(Debug, Clone, Default)]
pub struct ArchiveRow {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub week: i32,
    /// `DATE_FORMAT( post_date, '%Y-%m-%d' )` (weekly archives).
    pub yyyymmdd: String,
    pub posts: u64,
}

/// The grouping of `wp_get_archives()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Yearly,
    Monthly,
    Daily,
    /// With the `start_of_week` option.
    Weekly(u32),
}

impl Db {
    /// The groups of published posts of one type, as `wp_get_archives()` queries them.
    pub async fn archive_rows(&self, kind: ArchiveKind, post_type: &str, descending: bool, limit: Option<u64>) -> Result<Vec<ArchiveRow>> {
        let table = self.prefix.table("posts");
        let order = if descending { "DESC" } else { "ASC" };
        let aggregate = if descending { "MAX" } else { "MIN" };
        let limit = limit.map(|l| format!(" LIMIT {l}")).unwrap_or_default();
        let filter = "WHERE post_type = ? AND post_status = 'publish'";
        match kind {
            ArchiveKind::Yearly => {
                let sql = format!("SELECT YEAR(post_date), COUNT(ID) FROM {table} {filter} GROUP BY YEAR(post_date) ORDER BY {aggregate}(post_date) {order}{limit}");
                let rows: Vec<(i32, i64)> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
                Ok(rows.into_iter().map(|(year, n)| ArchiveRow { year, posts: n as u64, ..ArchiveRow::default() }).collect())
            }
            ArchiveKind::Monthly => {
                let sql = format!("SELECT YEAR(post_date), MONTH(post_date), COUNT(ID) FROM {table} {filter} GROUP BY YEAR(post_date), MONTH(post_date) ORDER BY {aggregate}(post_date) {order}{limit}");
                let rows: Vec<(i32, i32, i64)> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
                Ok(rows.into_iter().map(|(year, month, n)| ArchiveRow { year, month, posts: n as u64, ..ArchiveRow::default() }).collect())
            }
            ArchiveKind::Daily => {
                let sql = format!("SELECT YEAR(post_date), MONTH(post_date), DAYOFMONTH(post_date), COUNT(ID) FROM {table} {filter} GROUP BY YEAR(post_date), MONTH(post_date), DAYOFMONTH(post_date) ORDER BY {aggregate}(post_date) {order}{limit}");
                let rows: Vec<(i32, i32, i32, i64)> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
                Ok(rows.into_iter().map(|(year, month, day, n)| ArchiveRow { year, month, day, posts: n as u64, ..ArchiveRow::default() }).collect())
            }
            ArchiveKind::Weekly(start) => {
                // `_wp_mysql_week()`
                let week = match start {
                    1 => "WEEK( `post_date`, 1 )".to_owned(),
                    2..=6 => format!("WEEK( DATE_SUB( `post_date`, INTERVAL {start} DAY ), 0 )"),
                    _ => "WEEK( `post_date`, 0 )".to_owned(),
                };
                let sql = format!(
                    "SELECT {week}, YEAR( `post_date` ), DATE_FORMAT( {aggregate}(`post_date`), '%Y-%m-%d' ), count( `ID` ) FROM {table} {filter} GROUP BY {week}, YEAR( `post_date` ) ORDER BY {aggregate}(`post_date`) {order}{limit}"
                );
                let rows: Vec<(i32, i32, String, i64)> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
                Ok(rows.into_iter().map(|(week, year, yyyymmdd, n)| ArchiveRow { week, year, yyyymmdd, posts: n as u64, ..ArchiveRow::default() }).collect())
            }
        }
    }

    /// The published posts of one type for `postbypost`/`alpha` archives.
    pub async fn archive_posts(&self, post_type: &str, alpha: bool, limit: Option<u64>) -> Result<Vec<Post>> {
        let sql = format!(
            "SELECT {POST_COLUMNS} FROM {} WHERE post_type = ? AND post_status = 'publish' ORDER BY {}{}",
            self.prefix.table("posts"),
            if alpha { "post_title ASC" } else { "post_date DESC, ID DESC" },
            limit.map(|l| format!(" LIMIT {l}")).unwrap_or_default()
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?)
    }

    /// `(object_id, term_taxonomy_id)` of published posts of `post_types` in these terms (`_pad_term_counts()`).
    pub async fn term_objects(&self, term_taxonomy_ids: &[u64], post_types: &[&str]) -> Result<Vec<(u64, u64)>> {
        if term_taxonomy_ids.is_empty() || post_types.is_empty() {
            return Ok(Vec::new());
        }
        let ids = term_taxonomy_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let types = post_types.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT object_id, term_taxonomy_id FROM {} INNER JOIN {} ON object_id = ID WHERE term_taxonomy_id IN ({ids}) AND post_type IN ({types}) AND post_status = 'publish'",
            self.prefix.table("term_relationships"),
            self.prefix.table("posts")
        );
        let mut query = sqlx::query_as(AssertSqlSafe(sql));
        for t in post_types {
            query = query.bind(*t);
        }
        Ok(query.fetch_all(&self.pool).await?)
    }
}

impl Db {
    /// The distinct `post_type` values in use.
    pub async fn used_post_types(&self) -> Result<Vec<String>> {
        let sql = format!("SELECT DISTINCT post_type FROM {} ORDER BY post_type", self.prefix.table("posts"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }

    /// The distinct `taxonomy` values in use.
    pub async fn used_taxonomies(&self) -> Result<Vec<String>> {
        let sql = format!("SELECT DISTINCT taxonomy FROM {} ORDER BY taxonomy", self.prefix.table("term_taxonomy"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }
}

impl Db {
    /// The taxonomies in which some term has a parent.
    pub async fn taxonomies_with_parents(&self) -> Result<Vec<String>> {
        let sql = format!("SELECT DISTINCT taxonomy FROM {} WHERE parent <> 0 ORDER BY taxonomy", self.prefix.table("term_taxonomy"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }

    /// `(taxonomy, post_type)` pairs that have at least one relationship.
    pub async fn taxonomy_post_types(&self) -> Result<Vec<(String, String)>> {
        let sql = format!(
            "SELECT DISTINCT tt.taxonomy, p.post_type FROM {} tr INNER JOIN {} tt ON tt.term_taxonomy_id = tr.term_taxonomy_id INNER JOIN {} p ON p.ID = tr.object_id ORDER BY tt.taxonomy, p.post_type",
            self.prefix.table("term_relationships"),
            self.prefix.table("term_taxonomy"),
            self.prefix.table("posts")
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?)
    }
}
