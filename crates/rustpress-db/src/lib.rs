//! Read-only access to the WordPress MySQL/MariaDB schema.
//!
//! Table names depend on the configurable prefix (`wp_` by default), so the
//! queries are built at runtime. The prefix is validated and every value is a
//! bound parameter. Connections are put into `READ ONLY` mode, so a bug here
//! cannot modify a site's data.
//!
//! Date columns are returned as the strings MySQL stores (`2026-10-24 14:07:01`),
//! because WordPress allows `0000-00-00 00:00:00`, which date types cannot hold.

mod admin;
mod admin_comments;
mod admin_media;
mod admin_users;
mod archives;
mod comments;
mod export;
mod info;
mod models;
mod options;
mod posts;
mod primed;
mod probe;
mod terms;
mod users;
mod write;

use sqlx::Executor;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};

pub use admin::SlugScope;
pub use admin_comments::{CommentCounts, CommentListQuery};
pub use admin_media::AttachmentQuery;
pub use admin_users::UserListQuery;
pub use archives::{ArchiveKind, ArchiveRow};
pub use comments::{CommentOrder, CommentQuery};
pub use export::{ExportComment, ExportFilter};
pub use info::ServerInfo;
pub use models::{AuthUser, Comment, Post, PostMeta, Term, User};
pub use posts::{Order, OrderBy, PostQuery};
pub use probe::{Probe, probe};
pub use write::{Val, Writer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("invalid table prefix {0:?}: only letters, digits and `_` are allowed")]
    InvalidPrefix(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A WordPress table prefix that is safe to put into SQL identifiers.
#[derive(Debug, Clone)]
pub struct Prefix(String);

impl Prefix {
    pub fn new(prefix: impl Into<String>) -> Result<Self> {
        let prefix = prefix.into();
        let ok = prefix.len() <= 32 && prefix.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if ok { Ok(Self(prefix)) } else { Err(Error::InvalidPrefix(prefix)) }
    }

    /// `prefix.table("posts")` is `wp_posts` for the default prefix.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn table(&self, name: &str) -> String {
        format!("`{}{name}`", self.0)
    }
}

#[derive(Clone)]
pub struct Db {
    pool: MySqlPool,
    prefix: Prefix,
}

static ONE_REQUEST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The process answers one request and ends (CGI): every connection costs a round trip to the database server
/// that the request has to wait for, so the pools open one connection each, and the writing one only when the
/// request writes.
pub fn set_one_request(on: bool) {
    ONE_REQUEST.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn one_request() -> bool {
    ONE_REQUEST.load(std::sync::atomic::Ordering::Relaxed)
}

impl Db {
    /// Connects with a `mysql://user:pass@host:port/database` URL.
    pub async fn connect(url: &str, prefix: &str) -> Result<Self> {
        let prefix = Prefix::new(prefix)?;
        let pool = MySqlPoolOptions::new()
            .max_connections(if one_request() { 1 } else { 8 })
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    conn.execute("SET SESSION TRANSACTION READ ONLY").await?;
                    Ok(())
                })
            })
            .connect(url)
            .await?;
        Ok(Self { pool, prefix })
    }

    /// Like [Db::connect], but connects on first use. Handy for tests that never query.
    pub fn connect_lazy(url: &str, prefix: &str) -> Result<Self> {
        let prefix = Prefix::new(prefix)?;
        let pool = MySqlPoolOptions::new().max_connections(8).connect_lazy(url)?;
        Ok(Self { pool, prefix })
    }

    pub fn prefix(&self) -> &Prefix {
        &self.prefix
    }

    /// The underlying pool, for queries this crate does not cover yet.
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_rejects_sql_metacharacters() {
        assert!(Prefix::new("wp_").is_ok());
        assert!(Prefix::new("Site2_").is_ok());
        assert!(Prefix::new("wp`; DROP TABLE x; --").is_err());
        assert!(Prefix::new("wp-").is_err());
        assert!(Prefix::new("x".repeat(33)).is_err());
        assert_eq!(Prefix::new("wp_").unwrap().table("posts"), "`wp_posts`");
    }
}
