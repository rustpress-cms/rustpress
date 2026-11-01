//! Reads for the Redirection plugin (`Red_Item`, `Random_Action`).

use sqlx::AssertSqlSafe;

use crate::{Db, Result};

/// A row of `redirection_items` as `Red_Item` loads it.
#[derive(Debug, Clone, Default)]
pub struct RedirectionItem {
    pub id: u64,
    pub url: String,
    pub match_url: Option<String>,
    pub match_data: Option<String>,
    pub regex: bool,
    pub position: u64,
    pub action_type: String,
    pub action_code: i64,
    pub action_data: Option<String>,
    pub match_type: String,
    pub status: String,
    pub group_id: i64,
}

type Row = (u32, String, Option<String>, Option<String>, u32, u32, String, u32, Option<String>, String, String, i32);

impl Db {
    /// `Red_Item::get_for_matched_url()`: the enabled redirects of a match URL (with and without its query) and all
    /// regular expressions, in the order of the table (the plugin sorts them by position and id).
    pub async fn redirection_items_for(&self, without_query: &str, with_query: &str) -> Result<Vec<RedirectionItem>> {
        let sql = format!(
            "SELECT id, url, match_url, match_data, regex, position, action_type, action_code, action_data, match_type, status, group_id FROM {} WHERE match_url IN (?, ?, 'regex') AND status='enabled' LIMIT 20000",
            self.prefix.table("redirection_items")
        );
        let rows: Vec<Row> = sqlx::query_as(AssertSqlSafe(sql)).bind(without_query).bind(with_query).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|(id, url, match_url, match_data, regex, position, action_type, action_code, action_data, match_type, status, group_id)| RedirectionItem {
                id: u64::from(id),
                url,
                match_url,
                match_data,
                regex: regex != 0,
                position: u64::from(position),
                action_type,
                action_code: i64::from(action_code),
                action_data,
                match_type,
                status,
                group_id: i64::from(group_id),
            })
            .collect())
    }

    /// `Random_Action::get_random_url()`: a random published post without a password.
    pub async fn random_public_post_id(&self) -> Result<Option<u64>> {
        let sql = format!("SELECT ID FROM {} WHERE post_status='publish' AND post_password='' AND post_type='post' ORDER BY RAND() LIMIT 0,1", self.prefix.table("posts"));
        Ok(sqlx::query_scalar(AssertSqlSafe(sql)).fetch_optional(&self.pool).await?)
    }
}
