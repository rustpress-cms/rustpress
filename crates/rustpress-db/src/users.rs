use sqlx::AssertSqlSafe;
use crate::models::{USER_COLUMNS, User};
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
}
