//! Options, permalinks, post queries and template hierarchy.
//!
//! [`Site::query`] is the entry point of the request pipeline: it maps a URL to
//! WordPress' conditional flags and the posts of the main query, which
//! [`hierarchy::template_candidates`] turns into a list of template names.

pub mod datetime;
pub mod formatting;
pub mod hierarchy;
pub mod options;
pub mod query;
pub mod rewrite;
pub mod settings;
pub mod site;
pub mod slug;
pub mod urls;

pub use query::{Flags, Queried, QueryResult};
pub use rewrite::QueryVars;
pub use settings::{Settings, ShowOnFront};
pub use site::Site;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
