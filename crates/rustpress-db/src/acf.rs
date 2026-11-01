//! Reads for Advanced Custom Fields: its internal post types (`acf-field-group`, `acf-field`, `acf-post-type`,
//! `acf-taxonomy`) and the meta values of fields.

use sqlx::AssertSqlSafe;

use crate::{Db, Result};

/// A post of an ACF internal post type as `ACF_Internal_Post_Type::get_raw_post()` reads it.
#[derive(Debug, Clone, Default)]
pub struct AcfPost {
    pub id: u64,
    pub parent: u64,
    pub title: String,
    /// `post_name`: the key (`group_…`, `field_…`).
    pub name: String,
    /// `post_excerpt`: the field name.
    pub excerpt: String,
    pub menu_order: i64,
    pub status: String,
    /// `post_content`: the serialized settings.
    pub content: String,
}

/// The meta tables ACF stores values in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaKind {
    Post,
    Term,
    User,
    Comment,
}

type Row = (u64, u64, String, String, String, i64, String, String);

impl Db {
    /// `ACF_Internal_Post_Type::get_raw_posts()`: the published and disabled posts of an internal post type,
    /// ordered by `menu_order` and title (`get_posts()` with `orderby => 'menu_order title'`).
    pub async fn acf_posts(&self, post_type: &str) -> Result<Vec<AcfPost>> {
        let sql = format!(
            "SELECT ID, post_parent, post_title, post_name, post_excerpt, menu_order, post_status, post_content FROM {} WHERE post_type = ? AND post_status IN ('publish', 'acf-disabled') ORDER BY menu_order ASC, post_title ASC",
            self.prefix.table("posts")
        );
        let rows: Vec<Row> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(acf_post).collect())
    }

    /// `acf_get_raw_fields()` for every parent at once: the published and trashed `acf-field` posts, ordered by
    /// `menu_order`.
    pub async fn acf_fields(&self) -> Result<Vec<AcfPost>> {
        let sql = format!(
            "SELECT ID, post_parent, post_title, post_name, post_excerpt, menu_order, post_status, post_content FROM {} WHERE post_type = 'acf-field' AND post_status IN ('publish', 'trash') ORDER BY menu_order ASC, ID ASC",
            self.prefix.table("posts")
        );
        let rows: Vec<Row> = sqlx::query_as(AssertSqlSafe(sql)).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(acf_post).collect())
    }

    /// `get_posts( [ 'post_type' => $type, 'post_parent' => $id, 'posts_per_page' => 1 ] )` is not empty: a
    /// published child of the same type exists.
    pub async fn acf_has_child(&self, post_type: &str, parent: u64) -> Result<bool> {
        let sql = format!("SELECT ID FROM {} WHERE post_type = ? AND post_parent = ? AND post_status = 'publish' LIMIT 1", self.prefix.table("posts"));
        let id: Option<u64> = sqlx::query_scalar(AssertSqlSafe(sql)).bind(post_type).bind(parent).fetch_optional(&self.pool).await?;
        Ok(id.is_some())
    }

    /// `get_metadata( $type, $id, $key )[0]`: the first value of a meta key.
    pub async fn acf_meta(&self, kind: MetaKind, object_id: u64, key: &str) -> Result<Option<String>> {
        let (table, column, id) = match kind {
            MetaKind::Post => ("postmeta", "post_id", "meta_id"),
            MetaKind::Term => ("termmeta", "term_id", "meta_id"),
            MetaKind::User => ("usermeta", "user_id", "umeta_id"),
            MetaKind::Comment => ("commentmeta", "comment_id", "meta_id"),
        };
        let sql = format!("SELECT meta_value FROM {} WHERE {column} = ? AND meta_key = ? ORDER BY {id} ASC LIMIT 1", self.prefix.table(table));
        let value: Option<Option<String>> = sqlx::query_scalar(AssertSqlSafe(sql)).bind(object_id).bind(key).fetch_optional(&self.pool).await?;
        Ok(value.map(Option::unwrap_or_default))
    }
}

fn acf_post((id, parent, title, name, excerpt, menu_order, status, content): Row) -> AcfPost {
    AcfPost { id, parent, title, name, excerpt, menu_order, status, content }
}
