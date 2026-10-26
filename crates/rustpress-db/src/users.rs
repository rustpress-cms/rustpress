use sqlx::AssertSqlSafe;
use crate::models::{AUTH_USER_COLUMNS, AuthUser, USER_COLUMNS, User};
use crate::{Db, Result};

impl Db {
    pub async fn user(&self, id: u64) -> Result<Option<User>> {
        let sql = format!("SELECT {USER_COLUMNS} FROM {} WHERE ID = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(id).fetch_optional(&self.pool).await?)
    }

    pub async fn user_by_nicename(&self, nicename: &str) -> Result<Option<User>> {
        let sql = format!("SELECT {USER_COLUMNS} FROM {} WHERE user_nicename = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(nicename).fetch_optional(&self.pool).await?)
    }

    /// Users that have published posts of the given types, ordered by login
    /// (`WP_User_Query` with `has_published_posts`).
    pub async fn users_with_published_posts(&self, post_types: &[&str], limit: u64, offset: u64) -> Result<Vec<User>> {
        let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(format!("SELECT {USER_COLUMNS} FROM {} WHERE ID IN (SELECT post_author FROM {} WHERE post_status = 'publish' AND post_type IN (", self.prefix.table("users"), self.prefix.table("posts")));
        let mut separated = qb.separated(", ");
        for t in post_types {
            separated.push_bind((*t).to_owned());
        }
        qb.push(")) ORDER BY user_login ASC LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
        Ok(qb.build_query_as().fetch_all(&self.pool).await?)
    }

    pub async fn count_users_with_published_posts(&self, post_types: &[&str]) -> Result<u64> {
        let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(format!("SELECT COUNT(*) FROM {} WHERE ID IN (SELECT post_author FROM {} WHERE post_status = 'publish' AND post_type IN (", self.prefix.table("users"), self.prefix.table("posts")));
        let mut separated = qb.separated(", ");
        for t in post_types {
            separated.push_bind((*t).to_owned());
        }
        qb.push("))");
        let count: i64 = qb.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// One `wp_usermeta` value (`get_user_meta( $id, $key, true )`).
    pub async fn user_meta(&self, user_id: u64, key: &str) -> Result<Option<String>> {
        let sql = format!("SELECT meta_value FROM {} WHERE user_id = ? AND meta_key = ? ORDER BY umeta_id LIMIT 1", self.prefix.table("usermeta"));
        Ok(sqlx::query_scalar::<_, Option<String>>(AssertSqlSafe(sql)).bind(user_id).bind(key).fetch_optional(&self.pool).await?.flatten())
    }
}
impl Db {
    /// The table prefix (`wp_`), for option and meta keys that contain it.
    pub fn table_prefix(&self) -> &str {
        self.prefix.as_str()
    }

    /// A user with the password hash, for signing in. Only the authentication code asks for this.
    pub async fn auth_user_by_login(&self, login: &str) -> Result<Option<AuthUser>> {
        let sql = format!("SELECT {AUTH_USER_COLUMNS} FROM {} WHERE user_login = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(login).fetch_optional(&self.pool).await?)
    }

    /// `user_activation_key` of a user: the password reset key (`time:hash`) or the activation key.
    pub async fn user_activation_key(&self, id: u64) -> Result<String> {
        let sql = format!("SELECT user_activation_key FROM {} WHERE ID = ?", self.prefix.table("users"));
        Ok(sqlx::query_scalar::<_, String>(AssertSqlSafe(sql)).bind(id).fetch_optional(&self.pool).await?.unwrap_or_default())
    }

    pub async fn auth_user_by_id(&self, id: u64) -> Result<Option<AuthUser>> {
        let sql = format!("SELECT {AUTH_USER_COLUMNS} FROM {} WHERE ID = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(id).fetch_optional(&self.pool).await?)
    }

    pub async fn auth_user_by_email(&self, email: &str) -> Result<Option<AuthUser>> {
        let sql = format!("SELECT {AUTH_USER_COLUMNS} FROM {} WHERE user_email = ?", self.prefix.table("users"));
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(email).fetch_optional(&self.pool).await?)
    }
}
