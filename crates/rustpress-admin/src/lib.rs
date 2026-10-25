//! The administration area of RustPress: the script and style loader of WordPress, the sign-in page
//! and the screens of `wp-admin`.

pub mod adminbar;
pub mod deps;
pub mod chrome;
pub mod content;
pub mod cron;
pub mod die;
pub mod list_table;
pub mod login;
pub mod menu;
pub mod registry;
pub mod screens;
pub mod scripts;
pub mod styles;
pub mod tags;
pub mod updates;

pub use scripts::Scripts;
pub use styles::Styles;
