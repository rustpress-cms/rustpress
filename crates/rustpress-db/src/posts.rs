use sqlx::AssertSqlSafe;
use sqlx::{MySql, QueryBuilder};

use crate::models::{POST_COLUMNS, Post, PostMeta};
use crate::{Db, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderBy {
    Date,
    Modified,
    Title,
    Id,
    MenuOrder,
    /// Search relevance as in `WP_Query::parse_search_order`: title matches
    /// first, then excerpt/content phrase matches, then newest first.
    /// Needs `search` terms (and `search_phrase` for multi-word searches).
    Relevance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

/// A subset of `WP_Query`'s arguments.
#[derive(Debug, Clone)]
pub struct PostQuery {
    pub post_types: Vec<String>,
    pub statuses: Vec<String>,
    pub id: Option<u64>,
    /// Only these IDs / not these IDs (`post__in`, `post__not_in`).
    pub ids: Vec<u64>,
    pub exclude_ids: Vec<u64>,
    pub name: Option<String>,
    /// `post_name IN (...)`, for resolving hierarchical page paths.
    pub names: Vec<String>,
    pub parent: Option<u64>,
    pub author: Option<u64>,
    /// Taxonomy filters: a post must match every group (AND), and a group
    /// matches if the post has any of its `term_taxonomy_id`s (OR).
    pub tax_groups: Vec<Vec<u64>>,
    /// Every term must appear in the title, excerpt or content (`LIKE`).
    pub search: Vec<String>,
    /// The complete search string, used by [`OrderBy::Relevance`].
    pub search_phrase: Option<String>,
    pub year: Option<u32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
    pub order_by: OrderBy,
    pub order: Order,
    pub limit: Option<u64>,
    pub offset: u64,
}

impl Default for PostQuery {
    fn default() -> Self {
        Self {
            post_types: vec!["post".into()],
            statuses: vec!["publish".into()],
            id: None,
            ids: Vec::new(),
            exclude_ids: Vec::new(),
            name: None,
            names: Vec::new(),
            parent: None,
            author: None,
            tax_groups: Vec::new(),
            search: Vec::new(),
            search_phrase: None,
            year: None,
            month: None,
            day: None,
            order_by: OrderBy::Date,
            order: Order::Desc,
            limit: None,
            offset: 0,
        }
    }
}

impl PostQuery {
    /// Appends `FROM <posts> WHERE ...` including the `WHERE` clause.
    fn push_filters(&self, qb: &mut QueryBuilder<MySql>, db: &Db) {
        qb.push(" FROM ").push(db.prefix.table("posts")).push(" p WHERE 1 = 1");
        push_in(qb, "p.post_type", &self.post_types);
        push_in(qb, "p.post_status", &self.statuses);
        if let Some(id) = self.id {
            qb.push(" AND p.ID = ").push_bind(id);
        }
        push_in(qb, "p.ID", &self.ids);
        push_in(qb, "p.ID NOT", &self.exclude_ids);
        if let Some(name) = &self.name {
            qb.push(" AND p.post_name = ").push_bind(name.clone());
        }
        push_in(qb, "p.post_name", &self.names);
        if let Some(parent) = self.parent {
            qb.push(" AND p.post_parent = ").push_bind(parent);
        }
        if let Some(author) = self.author {
            qb.push(" AND p.post_author = ").push_bind(author);
        }
        for group in self.tax_groups.iter().filter(|g| !g.is_empty()) {
            qb.push(" AND p.ID IN (SELECT object_id FROM ").push(db.prefix.table("term_relationships")).push(" WHERE term_taxonomy_id IN (");
            let mut ids = qb.separated(", ");
            for id in group {
                ids.push_bind(*id);
            }
            qb.push("))");
        }
        for term in &self.search {
            let pattern = like_pattern(term);
            qb.push(" AND (p.post_title LIKE ").push_bind(pattern.clone());
            qb.push(" OR p.post_excerpt LIKE ").push_bind(pattern.clone());
            qb.push(" OR p.post_content LIKE ").push_bind(pattern).push(")");
        }
        for (column, value) in [("YEAR", self.year), ("MONTH", self.month), ("DAYOFMONTH", self.day)] {
            if let Some(value) = value {
                qb.push(format!(" AND {column}(p.post_date) = ")).push_bind(value);
            }
        }
    }
}

/// `%term%` with LIKE wildcards in `term` escaped.
fn like_pattern(term: &str) -> String {
    format!("%{}%", term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
}

/// `(CASE WHEN ... THEN 1 ... ELSE 6 END)`, ported from `parse_search_order`.
fn push_relevance(qb: &mut QueryBuilder<MySql>, query: &PostQuery) {
    let terms = &query.search;
    let phrase = query.search_phrase.as_deref().filter(|_| terms.len() > 1);
    qb.push("(CASE ");
    if let Some(phrase) = phrase {
        qb.push("WHEN p.post_title LIKE ").push_bind(like_pattern(phrase)).push(" THEN 1 ");
    }
    // Beyond six terms WordPress only ranks by the whole phrase.
    if !terms.is_empty() && terms.len() < 7 {
        for (joiner, rank) in [(" AND ", 2), (" OR ", 3)] {
            if rank == 3 && terms.len() < 2 {
                break;
            }
            qb.push("WHEN (");
            for (i, term) in terms.iter().enumerate() {
                if i > 0 {
                    qb.push(joiner);
                }
                qb.push("p.post_title LIKE ").push_bind(like_pattern(term));
            }
            qb.push(format!(") THEN {rank} "));
        }
    }
    if let Some(phrase) = phrase {
        qb.push("WHEN p.post_excerpt LIKE ").push_bind(like_pattern(phrase)).push(" THEN 4 ");
        qb.push("WHEN p.post_content LIKE ").push_bind(like_pattern(phrase)).push(" THEN 5 ");
    }
    qb.push("ELSE 6 END)");
}

fn push_in<T>(qb: &mut QueryBuilder<MySql>, column: &str, values: &[T])
where
    T: Clone + Send + sqlx::Type<MySql> + for<'q> sqlx::Encode<'q, MySql> + 'static,
{
    if values.is_empty() {
        return;
    }
    qb.push(format!(" AND {column} IN ("));
    let mut list = qb.separated(", ");
    for value in values {
        list.push_bind(value.clone());
    }
    qb.push(")");
}

impl Db {
    pub async fn posts(&self, query: &PostQuery) -> Result<Vec<Post>> {
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT {POST_COLUMNS}"));
        query.push_filters(&mut qb, self);
        let column = match query.order_by {
            OrderBy::Date => "p.post_date",
            OrderBy::Modified => "p.post_modified",
            OrderBy::Title => "p.post_title",
            OrderBy::Id => "p.ID",
            OrderBy::MenuOrder => "p.menu_order",
            OrderBy::Relevance => "",
        };
        let direction = if query.order == Order::Asc { "ASC" } else { "DESC" };
        if query.order_by == OrderBy::Relevance {
            // Same rows in the same order as WordPress: rank, newest first, then oldest ID.
            qb.push(" ORDER BY ");
            push_relevance(&mut qb, query);
            qb.push(", p.post_date DESC, p.ID ASC");
        } else {
            // `ID` as tie-breaker keeps pagination stable (WordPress shows equal dates newest ID first).
            qb.push(format!(" ORDER BY {column} {direction}, p.ID {direction}"));
        }
        if let Some(limit) = query.limit {
            qb.push(" LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(query.offset);
        }
        Ok(qb.build_query_as().fetch_all(&self.pool).await?)
    }

    /// Number of matching posts, ignoring `limit`/`offset` (for pagination).
    pub async fn count_posts(&self, query: &PostQuery) -> Result<u64> {
        let mut qb = QueryBuilder::<MySql>::new("SELECT COUNT(*)");
        query.push_filters(&mut qb, self);
        let count: i64 = qb.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    pub async fn post(&self, id: u64) -> Result<Option<Post>> {
        let query = PostQuery {
            id: Some(id),
            post_types: Vec::new(),
            statuses: Vec::new(),
            limit: Some(1),
            ..PostQuery::default()
        };
        Ok(self.posts(&query).await?.pop())
    }

    pub async fn post_meta(&self, post_id: u64) -> Result<Vec<PostMeta>> {
        let sql = format!(
            "SELECT meta_id, post_id, meta_key, meta_value FROM {} WHERE post_id = ? ORDER BY meta_id",
            self.prefix.table("postmeta")
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_id).fetch_all(&self.pool).await?)
    }
}
