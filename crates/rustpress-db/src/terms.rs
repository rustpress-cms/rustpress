use sqlx::AssertSqlSafe;
use crate::models::{TERM_COLUMNS, Term};
use crate::{Db, Result};

impl Db {
    fn term_from(&self) -> String {
        format!(
            "{} t JOIN {} tt ON tt.term_id = t.term_id",
            self.prefix.table("terms"),
            self.prefix.table("term_taxonomy")
        )
    }

    /// Terms attached to a post, optionally restricted to one taxonomy.
    pub async fn terms_for_post(&self, post_id: u64, taxonomy: Option<&str>) -> Result<Vec<Term>> {
        let mut sql = format!(
            "SELECT {TERM_COLUMNS} FROM {} JOIN {} tr ON tr.term_taxonomy_id = tt.term_taxonomy_id \
             WHERE tr.object_id = ?",
            self.term_from(),
            self.prefix.table("term_relationships"),
        );
        if taxonomy.is_some() {
            sql.push_str(" AND tt.taxonomy = ?");
        }
        sql.push_str(" ORDER BY tr.term_order, t.term_id");
        let mut query = sqlx::query_as(AssertSqlSafe(sql)).bind(post_id);
        if let Some(taxonomy) = taxonomy {
            query = query.bind(taxonomy);
        }
        Ok(query.fetch_all(&self.pool).await?)
    }

    pub async fn term_by_slug(&self, taxonomy: &str, slug: &str) -> Result<Option<Term>> {
        let sql = format!(
            "SELECT {TERM_COLUMNS} FROM {} WHERE tt.taxonomy = ? AND t.slug = ?",
            self.term_from()
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(taxonomy).bind(slug).fetch_optional(&self.pool).await?)
    }

    /// A term by its `term_id`, optionally restricted to a taxonomy.
    pub async fn term_by_id(&self, taxonomy: Option<&str>, term_id: u64) -> Result<Option<Term>> {
        let mut sql = format!("SELECT {TERM_COLUMNS} FROM {} WHERE t.term_id = ?", self.term_from());
        if taxonomy.is_some() {
            sql.push_str(" AND tt.taxonomy = ?");
        }
        let mut query = sqlx::query_as(AssertSqlSafe(sql)).bind(term_id);
        if let Some(taxonomy) = taxonomy {
            query = query.bind(taxonomy);
        }
        Ok(query.fetch_optional(&self.pool).await?)
    }

    pub async fn terms(&self, taxonomy: &str) -> Result<Vec<Term>> {
        let sql = format!(
            "SELECT {TERM_COLUMNS} FROM {} WHERE tt.taxonomy = ? ORDER BY t.name",
            self.term_from()
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(taxonomy).fetch_all(&self.pool).await?)
    }
}
