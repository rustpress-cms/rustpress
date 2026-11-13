//! Reads for the Complianz plugin (`CMPLZ_COOKIEBANNER`, `cmplz_get_default_banner_id()`).

use sqlx::AssertSqlSafe;

use crate::{Db, Result};

impl Db {
    /// The rows of `{prefix}cmplz_cookiebanners` in the order of the table, each as its columns and their values as
    /// `$wpdb` returns them (strings, `NULL` as `None`). Empty when the table does not exist.
    pub async fn complianz_banners(&self) -> Result<Vec<Vec<(String, Option<String>)>>> {
        self.complianz_rows("cmplz_cookiebanners").await
    }

    /// The rows of `{prefix}cmplz_cookies` (`select * from {prefix}cmplz_cookies`), like
    /// [`complianz_banners()`](Self::complianz_banners).
    pub async fn complianz_cookies(&self) -> Result<Vec<Vec<(String, Option<String>)>>> {
        self.complianz_rows("cmplz_cookies").await
    }

    /// The rows of `{prefix}cmplz_services`, like [`complianz_banners()`](Self::complianz_banners).
    pub async fn complianz_services(&self) -> Result<Vec<Vec<(String, Option<String>)>>> {
        self.complianz_rows("cmplz_services").await
    }

    async fn complianz_rows(&self, name: &str) -> Result<Vec<Vec<(String, Option<String>)>>> {
        let table = self.prefix.table(name);
        let columns: Vec<String> = match sqlx::query_scalar(AssertSqlSafe(format!("SELECT COLUMN_NAME FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = '{}{name}' ORDER BY ORDINAL_POSITION", self.prefix.as_str()))).fetch_all(&self.pool).await {
            Ok(columns) => columns,
            Err(_) => return Ok(Vec::new()),
        };
        if columns.is_empty() {
            return Ok(Vec::new());
        }
        let select = columns.iter().map(|c| format!("CAST(`{}` AS CHAR)", c.replace('`', ""))).collect::<Vec<_>>().join(", ");
        let rows = sqlx::query(AssertSqlSafe(format!("SELECT {select} FROM {table}"))).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| {
                use sqlx::Row;
                columns.iter().enumerate().map(|(i, name)| (name.clone(), row.try_get::<Option<String>, _>(i).ok().flatten())).collect()
            })
            .collect())
    }

    /// The published pages that hold a document of the plugin (`[cmplz-document` or `wp:complianz/document`), as
    /// `get_shortcode_page_id()` queries them, each with its content and its meta `cmplz_shortcode`.
    pub async fn complianz_document_pages(&self) -> Result<Vec<(u64, String, Option<String>)>> {
        let posts = self.prefix.table("posts");
        let meta = self.prefix.table("postmeta");
        let rows: Vec<(u64, String)> = sqlx::query_as(AssertSqlSafe(format!(
            "SELECT ID, post_content FROM {posts} WHERE (post_content LIKE '%[cmplz-document%' OR post_content LIKE '%wp:complianz\\/document%') AND post_status = 'publish' AND post_type = 'page'"
        )))
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for (id, content) in rows {
            let value: Option<String> = sqlx::query_scalar(AssertSqlSafe(format!("SELECT meta_value FROM {meta} WHERE post_id = ? AND meta_key = 'cmplz_shortcode' ORDER BY meta_id LIMIT 1"))).bind(id).fetch_optional(&self.pool).await?.flatten();
            out.push((id, content, value));
        }
        Ok(out)
    }

    /// The posts with a document stored in the meta `cmplz_shortcode` (`cmplz_document::is_complianz_page()`).
    pub async fn complianz_shortcode_posts(&self) -> Result<Vec<u64>> {
        let meta = self.prefix.table("postmeta");
        Ok(sqlx::query_scalar(AssertSqlSafe(format!("SELECT DISTINCT post_id FROM {meta} WHERE meta_key = 'cmplz_shortcode' AND meta_value <> '' AND meta_value <> '0'"))).fetch_all(&self.pool).await?)
    }

    /// Whether a cookie of the plugin's list (not ignored, not deleted, not a translation) belongs to a service that
    /// shares data (`secondParty` or `thirdParty`), the end of `cmplz_banner_loader::site_shares_data()`. `false`
    /// when the tables do not exist.
    pub async fn complianz_cookies_share_data(&self) -> Result<bool> {
        let cookies = self.prefix.table("cmplz_cookies");
        let services = self.prefix.table("cmplz_services");
        let found: std::result::Result<Option<i64>, _> = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT 1 FROM {cookies} c JOIN {services} s ON s.ID = c.serviceID WHERE NOT COALESCE(c.ignored, 0) AND NOT COALESCE(c.deleted, 0) AND NOT COALESCE(c.isTranslationFrom, 0) AND (COALESCE(s.secondParty, 0) OR COALESCE(s.thirdParty, 0)) LIMIT 1"
        )))
        .fetch_optional(&self.pool)
        .await;
        Ok(matches!(found, Ok(Some(_))))
    }
}
