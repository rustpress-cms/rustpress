//! Block theme support: theme.json, templates, patterns and rendering.

pub mod classic;
pub mod document;
pub mod embed;
pub mod feed;
pub mod functions_php;
pub mod global_styles;
pub mod head;
pub mod page;
pub mod phpt;
mod plugin_assets;
pub mod registered_patterns;
pub mod theme;
pub mod theme_headers;
pub mod theme_json;

pub use document::{Document, Extras, ToolbarFn, ToolbarInfo, render_document, render_document_with};
pub use feed::{Feed, render_feed};
pub use page::{RenderedBody, render_body};
pub use theme::Theme;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("theme directory `{0}` not found")]
    ThemeMissing(String),
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
    #[error(transparent)]
    Blocks(#[from] rustpress_blocks::Error),
    #[error(transparent)]
    Core(#[from] rustpress_core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
