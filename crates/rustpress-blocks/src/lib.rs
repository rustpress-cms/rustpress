//! Gutenberg block parser and renderer.
//!
//! [`parser`] splits post content into blocks; [`render`] turns them into HTML
//! the way `WP_Block::render()` does, including the block supports
//! ([`supports`], [`layout`], [`elements`], [`dimensions`]) and the server-side
//! blocks in [`dynamic`].

pub mod content;
pub mod dimensions;
pub mod dynamic;
pub mod elements;
pub mod excerpt;
pub mod fluid;
pub mod general;
pub mod env;
pub mod html;
pub mod images;
pub mod interactivity;
pub mod layout;
pub mod parser;
pub mod php_json;
pub mod registry;
pub mod render;
pub mod safe_css;
pub mod shortcodes;
pub mod style_engine;
pub mod supports;
pub mod variations;

pub use env::{Context, Env, ThemeAccess};
pub use parser::{Block, Content, parse_blocks};
pub use registry::{BlockType, Registry};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] rustpress_core::Error),
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
