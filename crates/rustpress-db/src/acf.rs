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

/// A post as `get_post()` loads it (`WP_Post`): the row of the posts table with every column.
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct AcfWpPost {
    #[sqlx(flatten)]
    pub post: crate::Post,
    pub to_ping: String,
    pub pinged: String,
    pub post_content_filtered: String,
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

    /// `ACF_Internal_Post_Type::get_post_object( $key )`: the post of an ACF item by its key, also a disabled or
    /// trashed one (`orderby` `menu_order title`).
    pub async fn acf_post_by_key(&self, post_type: &str, key: &str) -> Result<Option<AcfPost>> {
        let sql = format!(
            "SELECT ID, post_parent, post_title, post_name, post_excerpt, menu_order, post_status, post_content FROM {} WHERE post_type = ? AND post_name = ? AND post_status IN ('publish', 'acf-disabled', 'trash') ORDER BY menu_order ASC, post_title ASC LIMIT 1",
            self.prefix.table("posts")
        );
        let row: Option<Row> = sqlx::query_as(AssertSqlSafe(sql)).bind(post_type).bind(key).fetch_optional(&self.pool).await?;
        Ok(row.map(acf_post))
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

impl Db {
    /// `get_posts( [ 'post__in' => $ids, 'post_type' => $types, 'post_status' => 'any', 'posts_per_page' => -1 ] )`
    /// as `acf_get_posts()` runs it: every status that is not excluded from search (`trash`, `auto-draft`), in the
    /// order of `$ids`.
    pub async fn acf_get_posts(&self, ids: &[u64], post_types: &[String]) -> Result<Vec<AcfWpPost>> {
        if ids.is_empty() || post_types.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT {}, to_ping, pinged, post_content_filtered FROM {} WHERE ID IN ({}) AND post_type IN ({}) AND post_status NOT IN ('trash', 'auto-draft')",
            crate::models::POST_COLUMNS,
            self.prefix.table("posts"),
            vec!["?"; ids.len()].join(","),
            vec!["?"; post_types.len()].join(",")
        );
        let mut query = sqlx::query_as::<_, AcfWpPost>(AssertSqlSafe(sql));
        for id in ids {
            query = query.bind(*id);
        }
        for post_type in post_types {
            query = query.bind(post_type);
        }
        let mut posts = query.fetch_all(&self.pool).await?;
        posts.sort_by_key(|p| ids.iter().position(|id| *id == p.post.id));
        Ok(posts)
    }

    /// `term_group` of a term (`WP_Term`), which [`crate::Term`] leaves out.
    pub async fn acf_term_group(&self, term_id: u64) -> Result<i64> {
        let sql = format!("SELECT term_group FROM {} WHERE term_id = ?", self.prefix.table("terms"));
        let group: Option<i64> = sqlx::query_scalar(AssertSqlSafe(sql)).bind(term_id).fetch_optional(&self.pool).await?;
        Ok(group.unwrap_or(0))
    }
}

fn acf_post((id, parent, title, name, excerpt, menu_order, status, content): Row) -> AcfPost {
    AcfPost { id, parent, title, name, excerpt, menu_order, status, content }
}
