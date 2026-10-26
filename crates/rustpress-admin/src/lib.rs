//! The administration area of RustPress: the script and style loader of WordPress, the sign-in page
//! and the screens of `wp-admin`.

pub mod adminbar;
pub mod deps;
pub mod chrome;
pub mod cron;
pub mod die;
pub mod list_table;
pub mod login;
pub mod mail;
mod mail_texts;
pub mod menu;
pub mod multipart;
pub mod plugins;
pub mod registry;
pub mod screens;
pub mod scripts;
pub mod styles;
pub mod tags;
pub mod themes;
pub mod updates;

pub use rustpress_content::{content, media, sanitize};
pub use scripts::Scripts;
pub use styles::Styles;
