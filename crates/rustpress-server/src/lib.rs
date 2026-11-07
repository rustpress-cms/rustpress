//! The RustPress HTTP server: static assets plus WordPress routing on top of
//! the read-only database.

// The future of `admin_page()` holds every screen; its layout needs more than the 128 levels of the default.
#![recursion_limit = "256"]

pub mod admin;
pub mod app;
pub mod cgi;
pub mod check;
pub mod client;
pub mod config;
pub mod install;
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
