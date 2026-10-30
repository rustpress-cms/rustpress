//! Options, permalinks, post queries and template hierarchy.
//!
//! [`Site::query`] is the entry point of the request pipeline: it maps a URL to
//! WordPress' conditional flags and the posts of the main query, which
//! [`hierarchy::template_candidates`] turns into a list of template names.

pub mod canonical;
pub mod datetime;
mod entities;
pub mod files;
pub mod formatting;
pub mod health;
pub mod hierarchy;
mod html_entities;
pub mod kses;
mod kses_post_table;
pub mod safe_css;
pub mod i18n;
pub mod locale;
pub mod native;
pub mod options;
pub mod php_lit;
pub mod plugins;
pub mod query;
pub mod rewrite;
pub mod rewrite_gen;
pub mod shortcodes;
pub mod settings;
pub mod site;
pub mod sitemap;
pub mod text;
pub mod text_diff;
pub mod types;
pub mod slug;
mod accents_table;
pub mod urls;
pub mod viewer;

pub use query::{Flags, Queried, QueryResult};
pub use rewrite::QueryVars;
pub use settings::{Settings, ShowOnFront};
pub use site::Site;
pub use viewer::Viewer;
pub use sitemap::SitemapOutput;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The WordPress release whose PHP code RustPress follows (`WORDPRESS_VERSION` in the repository root).
pub const WP_VERSION: &str = include_str!("../../../WORDPRESS_VERSION").trim_ascii();

/// `hash_equals()`: compares two secrets in a time that does not depend on where they differ.
pub fn hash_equals(known: &str, given: &str) -> bool {
    let (a, b) = (known.as_bytes(), given.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

/// A failed write that the caller has no way to report to the user is logged instead of dropped.
pub trait OrLog {
    #[track_caller]
    fn or_log(self);
}

impl<T, E: std::fmt::Display> OrLog for std::result::Result<T, E> {
    #[track_caller]
    fn or_log(self) {
        if let Err(error) = self {
            let at = std::panic::Location::caller();
            tracing::error!(%error, file = at.file(), line = at.line(), "write failed");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wp_version_is_a_release_number() {
        let parts: Vec<&str> = super::WP_VERSION.split('.').collect();
        assert!((2..=3).contains(&parts.len()) && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())), "{}", super::WP_VERSION);
    }
}
