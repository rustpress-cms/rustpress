//! The RustPress HTTP server: static assets plus WordPress routing on top of
//! the read-only database.

pub mod admin;
pub mod app;
pub mod cgi;
pub mod client;
pub mod config;
pub mod statics;

pub use app::App;
pub use config::Config;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] rustpress_db::Error),
    #[error(transparent)]
    Core(#[from] rustpress_core::Error),
    #[error(transparent)]
    Theme(#[from] rustpress_theme::Error),
    #[error(transparent)]
    Rest(#[from] rustpress_rest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
