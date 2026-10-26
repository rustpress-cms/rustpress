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
    /// A comment in any status.
    pub async fn comment_any(&self, id: u64) -> Result<Option<Comment>> {
        let sql = format!("SELECT {COMMENT_COLUMNS} FROM {} WHERE comment_ID = ?", self.prefix.table("comments"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(id).fetch_optional(&self.pool).await?)
    }

    pub async fn comment(&self, id: u64) -> Result<Option<Comment>> {
        let sql = format!("SELECT {COMMENT_COLUMNS} FROM {} WHERE comment_ID = ?", self.prefix.table("comments"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(id).fetch_optional(&self.pool).await?)
    }

    /// The comment `wp_allow_comment()` calls a duplicate: the same author, address and text on the same post under
    /// the same parent, not in the trash.
    pub async fn duplicate_comment(&self, post_id: u64, parent: u64, author: &str, email: &str, content: &str) -> Result<Option<u64>> {
        let mut sql = format!("SELECT comment_ID FROM {} WHERE comment_post_ID = ? AND comment_parent = ? AND comment_approved != 'trash' AND ( comment_author = ? ", self.prefix.table("comments"));
        if !email.is_empty() {
            sql.push_str("AND comment_author_email = ? ");
        }
        sql.push_str(") AND comment_content = ? LIMIT 1");
        let mut query = sqlx::query_scalar::<_, u64>(AssertSqlSafe(sql)).bind(post_id).bind(parent.to_string()).bind(author.to_owned());
        if !email.is_empty() {
            query = query.bind(email.to_owned());
        }
        Ok(query.bind(content.to_owned()).fetch_optional(&self.pool).await?)
    }

    /// The date (GMT) of the last comment of a user or an address since `since`: `wp_check_comment_flood()`.
    pub async fn last_comment_since(&self, by_user: Option<u64>, ip: &str, email: &str, since: &str) -> Result<Option<String>> {
        let column = if by_user.is_some() { "`user_id`" } else { "`comment_author_IP`" };
        let sql = format!(
            "SELECT CAST(`comment_date_gmt` AS CHAR) FROM {} WHERE `comment_date_gmt` >= ? AND ( {column} = ? OR `comment_author_email` = ? ) ORDER BY `comment_date_gmt` DESC LIMIT 1",
            self.prefix.table("comments")
        );
        let query = sqlx::query_scalar::<_, String>(AssertSqlSafe(sql)).bind(since.to_owned());
        let query = match by_user {
            Some(user) => query.bind(user.to_string()),
            None => query.bind(ip.to_owned()),
        };
        Ok(query.bind(email.to_owned()).fetch_optional(&self.pool).await?)
    }

    /// Whether an earlier comment of this person was approved (`check_comment()`, "comment author must have a
    /// previously approved comment"): by the user, or by name and address.
    pub async fn has_approved_comment(&self, user_id: Option<u64>, author: &str, email: &str) -> Result<bool> {
        let table = self.prefix.table("comments");
        let found: Option<String> = match user_id {
            Some(user) => sqlx::query_scalar(AssertSqlSafe(format!("SELECT comment_approved FROM {table} WHERE user_id = ? AND comment_approved = '1' LIMIT 1"))).bind(user).fetch_optional(&self.pool).await?,
            None => sqlx::query_scalar(AssertSqlSafe(format!("SELECT comment_approved FROM {table} WHERE comment_author = ? AND comment_author_email = ? AND comment_approved = '1' LIMIT 1"))).bind(author.to_owned()).bind(email.to_owned()).fetch_optional(&self.pool).await?,
        };
        Ok(found.is_some_and(|v| v == "1"))
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

    /// Public (approved) comments matching `query`, newest/oldest first as asked.
    pub async fn comments_query(&self, query: &CommentQuery) -> Result<Vec<Comment>> {
        let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(format!("SELECT {COMMENT_COLUMNS}"));
        query.push_filters(&mut qb, self);
        let column = match query.order_by {
            CommentOrder::DateGmt => "c.comment_date_gmt",
            CommentOrder::Date => "c.comment_date",
            CommentOrder::Id => "c.comment_ID",
            CommentOrder::Post => "c.comment_post_ID",
            CommentOrder::Parent => "c.comment_parent",
        };
        let direction = if query.descending { "DESC" } else { "ASC" };
        qb.push(format!(" ORDER BY {column} {direction}, c.comment_ID {direction}"));
        if let Some(limit) = query.limit {
            qb.push(" LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(query.offset);
        }
        Ok(qb.build_query_as().fetch_all(&self.pool).await?)
    }

    pub async fn count_comments(&self, query: &CommentQuery) -> Result<u64> {
        let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new("SELECT COUNT(*)");
        query.push_filters(&mut qb, self);
        let count: i64 = qb.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(count as u64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentOrder {
    DateGmt,
    Date,
    Id,
    Post,
    Parent,
}

/// What an anonymous visitor may list: approved comments of published,
/// unprotected posts, narrowed by the given filters.
#[derive(Debug, Clone)]
pub struct CommentQuery {
    pub ids: Vec<u64>,
    pub exclude_ids: Vec<u64>,
    pub post_in: Vec<u64>,
    pub parent_in: Vec<u64>,
    pub parent_not_in: Vec<u64>,
    /// Comment types to include (`""` is the classic comment).
    pub types: Vec<String>,
    pub search: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub order_by: CommentOrder,
    pub descending: bool,
    pub limit: Option<u64>,
    pub offset: u64,
}

impl Default for CommentQuery {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            exclude_ids: Vec::new(),
            post_in: Vec::new(),
            parent_in: Vec::new(),
            parent_not_in: Vec::new(),
            types: vec![String::new(), "comment".into()],
            search: None,
            after: None,
            before: None,
            order_by: CommentOrder::DateGmt,
            descending: true,
            limit: None,
            offset: 0,
        }
    }
}

impl CommentQuery {
    fn push_filters(&self, qb: &mut sqlx::QueryBuilder<sqlx::MySql>, db: &Db) {
        qb.push(" FROM ").push(db.prefix.table("comments")).push(" c JOIN ").push(db.prefix.table("posts")).push(" p ON p.ID = c.comment_post_ID");
        qb.push(" WHERE c.comment_approved = '1' AND (p.post_status = 'publish' OR (p.post_type = 'attachment' AND p.post_status = 'inherit')) AND p.post_password = ''");
        push_ids(qb, "c.comment_type", &self.types, false);
        push_numbers(qb, "c.comment_ID", &self.ids, false);
        push_numbers(qb, "c.comment_ID", &self.exclude_ids, true);
        push_numbers(qb, "c.comment_post_ID", &self.post_in, false);
        push_numbers(qb, "c.comment_parent", &self.parent_in, false);
        push_numbers(qb, "c.comment_parent", &self.parent_not_in, true);
        if let Some(search) = &self.search {
            let pattern = format!("%{}%", search.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
            qb.push(" AND (c.comment_author LIKE ").push_bind(pattern.clone());
            qb.push(" OR c.comment_author_email LIKE ").push_bind(pattern.clone());
            qb.push(" OR c.comment_author_url LIKE ").push_bind(pattern.clone());
            qb.push(" OR c.comment_author_IP LIKE ").push_bind(pattern.clone());
            qb.push(" OR c.comment_content LIKE ").push_bind(pattern).push(")");
        }
        if let Some(after) = &self.after {
            qb.push(" AND c.comment_date > ").push_bind(after.clone());
        }
        if let Some(before) = &self.before {
            qb.push(" AND c.comment_date < ").push_bind(before.clone());
        }
    }
}

fn push_numbers(qb: &mut sqlx::QueryBuilder<sqlx::MySql>, column: &str, values: &[u64], negate: bool) {
    if values.is_empty() {
        return;
    }
    qb.push(format!(" AND {column} {}IN (", if negate { "NOT " } else { "" }));
    let mut list = qb.separated(", ");
    for v in values {
        list.push_bind(*v);
    }
    qb.push(")");
}

fn push_ids(qb: &mut sqlx::QueryBuilder<sqlx::MySql>, column: &str, values: &[String], negate: bool) {
    if values.is_empty() {
        return;
    }
    qb.push(format!(" AND {column} {}IN (", if negate { "NOT " } else { "" }));
    let mut list = qb.separated(", ");
    for v in values {
        list.push_bind(v.clone());
    }
    qb.push(")");
}