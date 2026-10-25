//! Reads for the user screens: the list with search, role filter, sorting and paging, and per-user counts.

use sqlx::{AssertSqlSafe, MySql, QueryBuilder};

use crate::models::{USER_COLUMNS, User};
use crate::{Db, Result};

/// The filters of the users list.
#[derive(Debug, Clone, Default)]
pub struct UserListQuery {
    /// `*term*` over login, URL, email, nicename and display name.
    pub search: Option<String>,
    /// Only users who have this role (as stored in the capabilities meta).
    pub role: Option<String>,
    /// Users who have no role at all.
    pub no_role: bool,
    /// `login`, `email`, `registered`, `name` (display name), `ID`.
    pub orderby: String,
    pub desc: bool,
    pub limit: u64,
    pub offset: u64,
}

impl Db {
    fn push_user_filters(&self, qb: &mut QueryBuilder<MySql>, query: &UserListQuery) {
        let users = self.prefix.table("users");
        let usermeta = self.prefix.table("usermeta");
        qb.push(format!(" FROM {users} u WHERE 1=1"));
        if let Some(search) = query.search.as_deref().filter(|s| !s.is_empty()) {
            let like = format!("%{}%", search.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
            qb.push(" AND (u.user_login LIKE ").push_bind(like.clone());
            qb.push(" OR u.user_url LIKE ").push_bind(like.clone());
            qb.push(" OR u.user_email LIKE ").push_bind(like.clone());
            qb.push(" OR u.user_nicename LIKE ").push_bind(like.clone());
            qb.push(" OR u.display_name LIKE ").push_bind(like);
            qb.push(")");
        }
        let caps_key = format!("{}capabilities", self.prefix.as_str());
        if let Some(role) = query.role.as_deref().filter(|r| !r.is_empty()) {
            qb.push(format!(" AND EXISTS (SELECT 1 FROM {usermeta} m WHERE m.user_id = u.ID AND m.meta_key = "));
            qb.push_bind(caps_key.clone());
            qb.push(" AND m.meta_value LIKE ").push_bind(format!("%\"{}\";b:1;%", role.replace('%', "").replace('_', "\\_")));
            qb.push(")");
        }
        if query.no_role {
            qb.push(format!(" AND NOT EXISTS (SELECT 1 FROM {usermeta} m WHERE m.user_id = u.ID AND m.meta_key = "));
            qb.push_bind(caps_key);
            qb.push(" AND m.meta_value LIKE '%;b:1;%')");
        }
    }

    /// The users of the list and the number of all that match.
    pub async fn list_users(&self, query: &UserListQuery) -> Result<(Vec<User>, u64)> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*)");
        self.push_user_filters(&mut count, query);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT {}", USER_COLUMNS.replace("ID,", "u.ID,").replace(", user_", ", u.user_").replace(" display_name", " u.display_name").replace("CAST(user_registered", "CAST(u.user_registered")));
        self.push_user_filters(&mut qb, query);
        let column = match query.orderby.as_str() {
            "email" => "u.user_email",
            "registered" => "u.user_registered",
            "name" => "u.display_name",
            "ID" | "id" => "u.ID",
            _ => "u.user_login",
        };
        qb.push(format!(" ORDER BY {column} {}", if query.desc { "DESC" } else { "ASC" }));
        if query.limit > 0 {
            qb.push(" LIMIT ").push_bind(query.limit as i64).push(" OFFSET ").push_bind(query.offset as i64);
        }
        let users: Vec<User> = qb.build_query_as().fetch_all(&self.pool).await?;
        Ok((users, total as u64))
    }

    /// Every user with the raw capabilities meta (to count the roles).
    pub async fn user_capability_rows(&self) -> Result<Vec<(u64, Option<String>)>> {
        let usermeta = self.prefix.table("usermeta");
        let sql = format!("SELECT u.ID, m.meta_value FROM {} u LEFT JOIN {usermeta} m ON m.user_id = u.ID AND m.meta_key = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(format!("{}capabilities", self.prefix.as_str())).fetch_all(&self.pool).await?)
    }

    /// `count_many_users_posts()`: published posts of the public post types by author.
    pub async fn published_post_counts(&self, user_ids: &[u64], post_types: &[&str]) -> Result<Vec<(u64, u64)>> {
        if user_ids.is_empty() || post_types.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT post_author, COUNT(*) FROM {} WHERE post_status IN ('publish', 'private') AND post_type IN (", self.prefix.table("posts")));
        let mut separated = qb.separated(", ");
        for post_type in post_types {
            separated.push_bind(post_type.to_string());
        }
        separated.push_unseparated(") AND post_author IN (");
        let mut separated = qb.separated(", ");
        for id in user_ids {
            separated.push_bind(*id);
        }
        separated.push_unseparated(") GROUP BY post_author");
        let rows: Vec<(u64, i64)> = qb.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(id, n)| (id, n as u64)).collect())
    }

    /// A user with every column (`ID`, `user_login`, ...), by login, e-mail or nicename.
    pub async fn user_by_login(&self, login: &str) -> Result<Option<User>> {
        let sql = format!("SELECT {USER_COLUMNS} FROM {} WHERE user_login = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(login).fetch_optional(&self.pool).await?)
    }

    pub async fn user_by_email(&self, email: &str) -> Result<Option<User>> {
        let sql = format!("SELECT {USER_COLUMNS} FROM {} WHERE user_email = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(email).fetch_optional(&self.pool).await?)
    }

    pub async fn user_nicename_taken(&self, nicename: &str, except: u64) -> Result<bool> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE user_nicename = ? AND ID != ?", self.prefix.table("users"));
        let n: i64 = sqlx::query_scalar(AssertSqlSafe(sql)).bind(nicename).bind(except).fetch_one(&self.pool).await?;
        Ok(n > 0)
    }

    /// All meta rows of a user as `(key, value)`.
    pub async fn user_meta_all(&self, user_id: u64) -> Result<Vec<(String, String)>> {
        let sql = format!("SELECT meta_key, COALESCE(meta_value, '') FROM {} WHERE user_id = ? ORDER BY umeta_id", self.prefix.table("usermeta"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(user_id).fetch_all(&self.pool).await?)
    }

    /// The IDs of posts and links of an author (for reassigning when the user is deleted).
    pub async fn post_ids_of_author(&self, user_id: u64) -> Result<Vec<(u64, String)>> {
        let sql = format!("SELECT ID, post_type FROM {} WHERE post_author = ?", self.prefix.table("posts"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(user_id).fetch_all(&self.pool).await?)
    }
}
