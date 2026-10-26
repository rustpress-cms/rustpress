use sqlx::FromRow;

/// A row of `wp_posts` (posts, pages, attachments, navigation menus, ...).
#[derive(Debug, Clone, Default, FromRow)]
pub struct Post {
    #[sqlx(rename = "ID")]
    pub id: u64,
    pub post_author: u64,
    pub post_date: String,
    pub post_date_gmt: String,
    pub post_content: String,
    pub post_title: String,
    pub post_excerpt: String,
    pub post_status: String,
    pub comment_status: String,
    pub ping_status: String,
    pub post_password: String,
    pub post_name: String,
    pub post_modified: String,
    pub post_modified_gmt: String,
    pub post_parent: u64,
    pub guid: String,
    pub menu_order: i32,
    pub post_type: String,
    pub post_mime_type: String,
    pub comment_count: i64,
}

/// SELECT list for [`Post`]; dates are cast so that zero dates decode.
pub(crate) const POST_COLUMNS: &str = "ID, post_author, \
    CAST(post_date AS CHAR) AS post_date, CAST(post_date_gmt AS CHAR) AS post_date_gmt, \
    post_content, post_title, post_excerpt, post_status, comment_status, ping_status, \
    post_password, post_name, CAST(post_modified AS CHAR) AS post_modified, \
    CAST(post_modified_gmt AS CHAR) AS post_modified_gmt, post_parent, guid, menu_order, \
    post_type, post_mime_type, comment_count";

#[derive(Debug, Clone, Default, FromRow)]
pub struct PostMeta {
    pub meta_id: u64,
    pub post_id: u64,
    pub meta_key: Option<String>,
    /// Raw column value; may be PHP-serialized (see `rustpress-phpser`).
    pub meta_value: Option<String>,
}

/// A term joined with its taxonomy row.
#[derive(Debug, Clone, Default, FromRow)]
pub struct Term {
    pub term_id: u64,
    pub term_taxonomy_id: u64,
    pub name: String,
    pub slug: String,
    pub taxonomy: String,
    pub description: String,
    pub parent: u64,
    pub count: i64,
}

pub(crate) const TERM_COLUMNS: &str = "t.term_id, tt.term_taxonomy_id, t.name, t.slug, \
    tt.taxonomy, tt.description, tt.parent, tt.count";

/// A user without the password hash or activation key.
#[derive(Debug, Clone, Default, FromRow)]
pub struct User {
    #[sqlx(rename = "ID")]
    pub id: u64,
    pub user_login: String,
    pub user_nicename: String,
    pub user_email: String,
    pub user_url: String,
    pub user_registered: String,
    pub display_name: String,
}

pub(crate) const USER_COLUMNS: &str = "ID, user_login, user_nicename, user_email, user_url, \
    CAST(user_registered AS CHAR) AS user_registered, display_name";

/// A user row including the password hash. `Debug` leaves the hash out.
#[derive(Clone, FromRow)]
pub struct AuthUser {
    #[sqlx(rename = "ID")]
    pub id: u64,
    pub user_login: String,
    pub user_pass: String,
    pub user_nicename: String,
    pub user_email: String,
    pub display_name: String,
}

impl std::fmt::Debug for AuthUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthUser").field("id", &self.id).field("user_login", &self.user_login).finish_non_exhaustive()
    }
}

pub(crate) const AUTH_USER_COLUMNS: &str = "ID, user_login, user_pass, user_nicename, user_email, display_name";

#[derive(Debug, Clone, FromRow)]
pub struct Comment {
    #[sqlx(rename = "comment_ID")]
    pub comment_id: u64,
    #[sqlx(rename = "comment_post_ID")]
    pub comment_post_id: u64,
    pub comment_author: String,
    pub comment_author_email: String,
    pub comment_author_url: String,
    pub comment_date: String,
    pub comment_date_gmt: String,
    pub comment_content: String,
    /// `1`, `0`, `spam` or `trash`.
    pub comment_approved: String,
    pub comment_type: String,
    pub comment_parent: u64,
    pub user_id: u64,
    #[sqlx(rename = "comment_author_IP")]
    pub comment_author_ip: String,
    pub comment_agent: String,
}

pub(crate) const COMMENT_COLUMNS: &str = "comment_ID, comment_post_ID, comment_author, \
    comment_author_email, comment_author_url, CAST(comment_date AS CHAR) AS comment_date, \
    CAST(comment_date_gmt AS CHAR) AS comment_date_gmt, comment_content, comment_approved, \
    comment_type, comment_parent, user_id, comment_author_IP, comment_agent";
