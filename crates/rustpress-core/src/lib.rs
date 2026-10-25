//! Options, permalinks, post queries and template hierarchy.
//!
//! [`Site::query`] is the entry point of the request pipeline: it maps a URL to
//! WordPress' conditional flags and the posts of the main query, which
//! [`hierarchy::template_candidates`] turns into a list of template names.

pub mod canonical;
pub mod datetime;
mod entities;
pub mod formatting;
pub mod hierarchy;
pub mod i18n;
pub mod locale;
pub mod options;
pub mod php_lit;
pub mod query;
pub mod rewrite;
pub mod rewrite_gen;
pub mod shortcodes;
pub mod settings;
pub mod site;
pub mod sitemap;
pub mod text;
pub mod types;
pub mod slug;
pub mod urls;

pub use query::{Flags, Queried, QueryResult};
pub use rewrite::QueryVars;
pub use settings::{Settings, ShowOnFront};
pub use site::Site;
pub use sitemap::SitemapOutput;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
