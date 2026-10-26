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
    Author,
    Slug,
    Parent,
    CommentCount,
    /// `menu_order title` (the order of the pages list).
    MenuOrderTitle,
    /// The order of ids (orderby=include).
    Include,
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
    /// uthor__in / uthor__not_in.
    pub author_in: Vec<u64>,
    pub author_not_in: Vec<u64>,
    /// post_parent__in / post_parent__not_in.
    pub parent_in: Vec<u64>,
    pub parent_not_in: Vec<u64>,
    /// Posts that have none of these `term_taxonomy_id`s (`NOT IN` tax queries).
    pub tax_not: Vec<u64>,
    /// MySQL datetimes (post_date / post_modified, local time).
    pub date_after: Option<String>,
    pub date_before: Option<String>,
    pub modified_after: Option<String>,
    pub modified_before: Option<String>,
    pub menu_order: Option<i32>,
    /// post_mime_type LIKE 'image/%' style filter and an exact mime type.
    pub mime_prefix: Option<String>,
    pub mime_type: Option<String>,
    /// Only attachments whose parent is published (or that have none).
    pub public_parent: bool,
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
    pub hour: Option<u32>,
    pub minute: Option<u32>,
    pub second: Option<u32>,
    /// `(week, start_of_week)`: `WEEK( post_date )` the way `_wp_mysql_week()` writes it.
    pub week: Option<(u32, u32)>,
    pub order_by: OrderBy,
    pub order: Order,
    pub limit: Option<u64>,
    pub offset: u64,
    /// Private posts of these post types are listed, too (the viewer may read them).
    pub private_types: Vec<String>,
    /// Private posts of this author are listed, too (the viewer may read their own).
    pub private_author: Option<u64>,
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
            author_in: Vec::new(),
            author_not_in: Vec::new(),
            parent_in: Vec::new(),
            parent_not_in: Vec::new(),
            tax_not: Vec::new(),
            date_after: None,
            date_before: None,
            modified_after: None,
            modified_before: None,
            menu_order: None,
            mime_prefix: None,
            mime_type: None,
            public_parent: false,
            tax_groups: Vec::new(),
            search: Vec::new(),
            search_phrase: None,
            year: None,
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            week: None,
            order_by: OrderBy::Date,
            order: Order::Desc,
            limit: None,
            offset: 0,
            private_types: Vec::new(),
            private_author: None,
        }
    }
}

impl PostQuery {
    /// Appends `FROM <posts> WHERE ...` including the `WHERE` clause.
    fn push_filters(&self, qb: &mut QueryBuilder<MySql>, db: &Db) {
        qb.push(" FROM ").push(db.prefix.table("posts")).push(" p WHERE 1 = 1");
        push_in(qb, "p.post_type", &self.post_types);
        if self.statuses.iter().any(|s| s == "publish") && (!self.private_types.is_empty() || self.private_author.is_some()) {
            // `WP_Query::get_posts()` for a signed-in viewer: `OR post_status = 'private'` where they may read it.
            qb.push(" AND (p.post_status IN (");
            let mut list = qb.separated(", ");
            for status in &self.statuses {
                list.push_bind(status.clone());
            }
            qb.push(") OR (p.post_status = 'private' AND (");
            let mut first = true;
            if !self.private_types.is_empty() {
                qb.push("p.post_type IN (");
                let mut list = qb.separated(", ");
                for post_type in &self.private_types {
                    list.push_bind(post_type.clone());
                }
                qb.push(")");
                first = false;
            }
            if let Some(author) = self.private_author {
                if !first {
                    qb.push(" OR ");
                }
                qb.push("p.post_author = ").push_bind(author);
            }
            qb.push(")))");
        } else {
            push_in(qb, "p.post_status", &self.statuses);
        }
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
        push_in(qb, "p.post_author", &self.author_in);
        push_in(qb, "p.post_author NOT", &self.author_not_in);
        push_in(qb, "p.post_parent", &self.parent_in);
        push_in(qb, "p.post_parent NOT", &self.parent_not_in);
        if let Some(prefix) = &self.mime_prefix {
            qb.push(" AND p.post_mime_type LIKE ").push_bind(format!("{}/%", prefix.replace('%', "\\%").replace('_', "\\_")));
        }
        if let Some(mime) = &self.mime_type {
            qb.push(" AND p.post_mime_type = ").push_bind(mime.clone());
        }
        if self.public_parent {
            qb.push(" AND (p.post_parent = 0 OR p.post_parent IN (SELECT parent.ID FROM ").push(db.prefix.table("posts")).push(" parent WHERE parent.post_status = 'publish'))");
        }
        if let Some(menu_order) = self.menu_order {
            qb.push(" AND p.menu_order = ").push_bind(menu_order);
        }
        for (column, op, value) in [
            ("p.post_date", ">", &self.date_after),
            ("p.post_date", "<", &self.date_before),
            ("p.post_modified", ">", &self.modified_after),
            ("p.post_modified", "<", &self.modified_before),
        ] {
            if let Some(value) = value {
                qb.push(format!(" AND {column} {op} ")).push_bind(value.clone());
            }
        }
        if !self.tax_not.is_empty() {
            qb.push(" AND p.ID NOT IN (SELECT object_id FROM ").push(db.prefix.table("term_relationships")).push(" WHERE term_taxonomy_id IN (");
            let mut ids = qb.separated(", ");
            for id in &self.tax_not {
                ids.push_bind(*id);
            }
            qb.push("))");
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
        for (column, value) in [("YEAR", self.year), ("MONTH", self.month), ("DAYOFMONTH", self.day), ("HOUR", self.hour), ("MINUTE", self.minute), ("SECOND", self.second)] {
            if let Some(value) = value {
                qb.push(format!(" AND {column}(p.post_date) = ")).push_bind(value);
            }
        }
        if let Some((week, start)) = self.week {
            let expression = match start {
                1 => "WEEK(p.post_date, 1)".to_owned(),
                2..=6 => format!("WEEK(DATE_SUB(p.post_date, INTERVAL {start} DAY), 0)"),
                _ => "WEEK(p.post_date, 0)".to_owned(),
            };
            qb.push(format!(" AND {expression} = ")).push_bind(week);
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
            OrderBy::Author => "p.post_author",
            OrderBy::Slug => "p.post_name",
            OrderBy::Parent => "p.post_parent",
            OrderBy::CommentCount => "p.comment_count",
            OrderBy::MenuOrderTitle => "p.menu_order ASC, p.post_title",
            OrderBy::Include => "",
            OrderBy::Relevance => "",
        };
        let direction = if query.order == Order::Asc { "ASC" } else { "DESC" };
        if query.order_by == OrderBy::Include {
            qb.push(" ORDER BY FIELD(p.ID");
            for id in &query.ids {
                qb.push(", ").push_bind(*id);
            }
            qb.push(")");
        } else if query.order_by == OrderBy::Relevance {
            // Same rows in the same order as WordPress: rank, newest first, then oldest ID.
            qb.push(" ORDER BY ");
            push_relevance(&mut qb, query);
            qb.push(", p.post_date DESC, p.ID ASC");
        } else {
            // `ID` as tie-breaker keeps pagination stable. WordPress shows equal dates newest
            // ID first, except for term queries: their `GROUP BY ID` feeds the sort oldest first.
            let tie = if query.tax_groups.is_empty() { direction } else { "ASC" };
            qb.push(format!(" ORDER BY {column} {direction}, p.ID {tie}"));
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

    /// `(year, month, posts)` of the published posts, newest month first (`wp_get_archives()`).
    pub async fn monthly_archives(&self) -> Result<Vec<(u32, u32, u64)>> {
        let sql = format!(
            "SELECT YEAR(post_date) AS y, MONTH(post_date) AS m, COUNT(ID) AS n FROM {} WHERE post_type = 'post' AND post_status = 'publish' GROUP BY y, m ORDER BY MAX(post_date) DESC",
            self.prefix.table("posts")
        );
        let rows: Vec<(i32, i32, i64)> = sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(y, m, n)| (y as u32, m as u32, n as u64)).collect())
    }

    /// Days of a month with published posts (`get_calendar()`).
    pub async fn calendar_days(&self, year: u32, month: u32, last_day: u32) -> Result<Vec<u32>> {
        let sql = format!(
            "SELECT DISTINCT DAYOFMONTH(post_date) FROM {} WHERE post_date >= ? AND post_type = 'post' AND post_status = 'publish' AND post_date <= ?",
            self.prefix.table("posts")
        );
        let rows: Vec<(i32,)> = sqlx::query_as(AssertSqlSafe(sql)).bind(format!("{year:04}-{month:02}-01 00:00:00")).bind(format!("{year:04}-{month:02}-{last_day:02} 23:59:59")).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(d,)| d as u32).collect())
    }

    /// The month before or after `year`/`month` that has published posts (`get_calendar()`).
    pub async fn calendar_neighbor(&self, year: u32, month: u32, last_day: u32, previous: bool) -> Result<Option<(u32, u32)>> {
        let (cmp, order, bound) = if previous {
            ("<", "DESC", format!("{year:04}-{month:02}-01"))
        } else {
            (">", "ASC", format!("{year:04}-{month:02}-{last_day:02} 23:59:59"))
        };
        let sql = format!(
            "SELECT YEAR(post_date), MONTH(post_date) FROM {} WHERE post_date {cmp} ? AND post_type = 'post' AND post_status = 'publish' ORDER BY post_date {order} LIMIT 1",
            self.prefix.table("posts")
        );
        let row: Option<(i32, i32)> = sqlx::query_as(AssertSqlSafe(sql)).bind(bound).fetch_optional(&self.pool).await?;
        Ok(row.map(|(y, m)| (y as u32, m as u32)))
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

    /// `redirect_guess_404_permalink()`: the first publicly viewable post whose
    /// slug starts with `name` (`LIKE 'name%'`), optionally narrowed by type and date.
    pub async fn guess_post_by_name(&self, name: &str, post_type: Option<&str>, year: Option<u32>, month: Option<u32>, day: Option<u32>) -> Result<Option<u64>> {
        let escaped: String = name.chars().flat_map(|c| if matches!(c, '\\' | '%' | '_') { vec!['\\', c] } else { vec![c] }).collect();
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT ID FROM {} WHERE post_name LIKE ", self.prefix.table("posts")));
        qb.push_bind(format!("{escaped}%"));
        match post_type {
            Some(t) => {
                qb.push(" AND post_type = ").push_bind(t.to_owned());
            }
            None => {
                qb.push(" AND post_type NOT IN ('revision','nav_menu_item','customize_changeset','oembed_cache','user_request') AND post_type NOT LIKE 'wp\\_%'");
            }
        }
        if let Some(y) = year {
            qb.push(" AND YEAR(post_date) = ").push_bind(y);
        }
        if let Some(m) = month {
            qb.push(" AND MONTH(post_date) = ").push_bind(m);
        }
        if let Some(d) = day {
            qb.push(" AND DAYOFMONTH(post_date) = ").push_bind(d);
        }
        qb.push(" AND post_status = 'publish' ORDER BY post_name, ID LIMIT 1");
        Ok(qb.build_query_scalar::<u64>().fetch_optional(&self.pool).await?)
    }

    /// `get_adjacent_post()` without term filters: the published post of the same type that comes
    /// directly before or after `(date, id)` in the order `post_date, ID`.
    pub async fn adjacent_post_id(&self, post_type: &str, date: &str, id: u64, previous: bool) -> Result<Option<u64>> {
        let (op, order) = if previous { ("<", "DESC") } else { (">", "ASC") };
        let sql = format!(
            "SELECT ID FROM {} WHERE (post_date {op} ? OR (post_date = ? AND ID {op} ?)) AND post_type = ? AND post_status = 'publish' ORDER BY post_date {order}, ID {order} LIMIT 1",
            self.prefix.table("posts")
        );
        Ok(sqlx::query_scalar::<_, u64>(AssertSqlSafe(sql)).bind(date.to_owned()).bind(date.to_owned()).bind(id).bind(post_type.to_owned()).fetch_optional(&self.pool).await?)
    }

    /// `wp_old_slug_redirect()` lookup: a post that once had this slug (`_wp_old_slug`).
    pub async fn post_by_old_slug(&self, name: &str, post_type: &str, year: Option<u32>, month: Option<u32>, day: Option<u32>) -> Result<Option<u64>> {
        let mut qb = QueryBuilder::<MySql>::new(format!(
            "SELECT post_id FROM {} AS m INNER JOIN {} AS p ON p.ID = m.post_id WHERE m.meta_key = '_wp_old_slug' AND m.meta_value = ",
            self.prefix.table("postmeta"),
            self.prefix.table("posts")
        ));
        qb.push_bind(name.to_owned()).push(" AND p.post_type = ").push_bind(post_type.to_owned());
        if let Some(y) = year {
            qb.push(" AND YEAR(p.post_date) = ").push_bind(y);
        }
        if let Some(m) = month {
            qb.push(" AND MONTH(p.post_date) = ").push_bind(m);
        }
        if let Some(d) = day {
            qb.push(" AND DAYOFMONTH(p.post_date) = ").push_bind(d);
        }
        qb.push(" AND p.post_status = 'publish' ORDER BY m.meta_id LIMIT 1");
        Ok(qb.build_query_scalar::<u64>().fetch_optional(&self.pool).await?)
    }
    /// `get_lastpostmodified( 'GMT' )`: the newest `post_modified_gmt` of published content.
    pub async fn last_post_modified_gmt(&self, post_types: &[&str]) -> Result<Option<String>> {
        let types = post_types.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT CAST(post_modified_gmt AS CHAR) FROM {} WHERE post_status = 'publish' AND post_type IN ({types}) ORDER BY post_modified_gmt DESC LIMIT 1",
            self.prefix.table("posts")
        );
        let mut query = sqlx::query_scalar(AssertSqlSafe(sql));
        for t in post_types {
            query = query.bind(*t);
        }
        Ok(query.fetch_optional(&self.pool).await?)
    }
    pub async fn post_meta(&self, post_id: u64) -> Result<Vec<PostMeta>> {
        let sql = format!(
            "SELECT meta_id, post_id, meta_key, meta_value FROM {} WHERE post_id = ? ORDER BY meta_id",
            self.prefix.table("postmeta")
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(post_id).fetch_all(&self.pool).await?)
    }
}
